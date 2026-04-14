mod commands;
mod core;

use clap::{Parser, Subcommand};
use std::process;

#[derive(Parser)]
#[command(
    name = "notarai",
    version,
    about = "CLI validator for NotarAI spec files"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate spec files (default: .notarai/)
    Validate {
        /// File or directory to validate
        path: Option<String>,
    },
    /// Deterministic drift detection (coverage gaps, orphaned globs, changed files, overlaps)
    Check {
        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
        /// Base branch for changed-since detection
        #[arg(long, default_value = "main")]
        base_branch: String,
        /// Promote all warnings to errors (zero-tolerance mode for CI)
        #[arg(long)]
        strict: bool,
    },
    /// Set up NotarAI in a project
    Init {
        /// Comma-separated agents: claude, gemini, codex, opencode, all, none.
        /// Omit for interactive prompt (TTY) or filesystem auto-detect (non-TTY).
        #[arg(long, value_delimiter = ',')]
        agents: Option<Vec<String>>,
        /// Deprecated: alias for --agents. `claude` maps to `--agents claude`;
        /// `generic` maps to `--agents opencode`.
        #[arg(long, hide = true)]
        agent: Option<String>,
    },
    /// Internal hook commands
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },
    /// Hash-based file cache for context footprint reduction
    Cache {
        #[command(subcommand)]
        action: commands::cache::CacheAction,
    },
    /// Export reconciliation context for any LLM agent
    ExportContext {
        /// Spec file path (relative to project root)
        #[arg(long)]
        spec: Option<String>,
        /// Export context for all affected specs
        #[arg(long)]
        all: bool,
        /// Output the bootstrap prompt for a new project with no specs yet
        #[arg(long)]
        bootstrap: bool,
        /// Base branch for diff
        #[arg(long, default_value = "main")]
        base_branch: String,
        /// Output format: markdown or json
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Manage decision proposals from reconciliation
    Decisions {
        #[command(subcommand)]
        action: commands::decisions::DecisionsAction,
    },
    /// Lint spec files for quality issues (superset of validate)
    Lint {
        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Compute drift scores for specs
    Score {
        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
        /// Score a single spec
        #[arg(long)]
        spec: Option<String>,
    },
    /// MCP server (stdio JSON-RPC 2.0 transport)
    Mcp,
    /// Update schema version across all specs in the project
    SchemaBump,
    /// Manage reconciliation state
    State {
        #[command(subcommand)]
        action: commands::state::StateAction,
    },
    /// Check for and install updates
    Update {
        /// Only check, don't install
        #[arg(long)]
        check: bool,
    },
}

#[derive(Subcommand)]
enum HookAction {
    /// Validate spec from Claude Code hook stdin
    Validate,
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Some(Commands::Validate { path }) => commands::validate::run(path),
        Some(Commands::Check {
            format,
            base_branch,
            strict,
        }) => commands::check::run(&format, &base_branch, strict),
        Some(Commands::Init { agents, agent }) => {
            let agents_raw: Option<Vec<String>> = match (agents, agent) {
                (Some(a), _) => Some(a),
                (None, Some(legacy)) => {
                    eprintln!(
                        "Warning: --agent is deprecated; use --agents. Forwarding '{legacy}'."
                    );
                    Some(vec![legacy])
                }
                (None, None) => None,
            };
            commands::init::run(None, agents_raw)
        }
        Some(Commands::Hook { action }) => match action {
            HookAction::Validate => commands::hook_validate::run(),
        },
        Some(Commands::Decisions { action }) => commands::decisions::run(action),
        Some(Commands::Lint { format }) => commands::lint::run(&format),
        Some(Commands::ExportContext {
            spec,
            all,
            bootstrap,
            base_branch,
            format,
        }) => commands::export_context::run(spec.as_deref(), all, bootstrap, &base_branch, &format),
        Some(Commands::Score { format, spec }) => commands::score::run(&format, spec.as_deref()),
        Some(Commands::Cache { action }) => commands::cache::run(action),
        Some(Commands::Mcp) => commands::mcp::run(),
        Some(Commands::SchemaBump) => commands::schema_bump::run(None),
        Some(Commands::State { action }) => commands::state::run(action),
        Some(Commands::Update { check }) => commands::update::run(check),
        None => {
            // Print help when no command given
            use clap::CommandFactory;
            Cli::command().print_help().ok();
            eprintln!();
            1
        }
    };

    process::exit(exit_code);
}
