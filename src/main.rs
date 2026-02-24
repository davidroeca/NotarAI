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
    /// Set up NotarAI in a project (hook, slash commands, schema, CLAUDE.md context)
    Init,
    /// Internal hook commands
    Hook {
        #[command(subcommand)]
        action: HookAction,
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
        Some(Commands::Init) => commands::init::run(None),
        Some(Commands::Hook { action }) => match action {
            HookAction::Validate => commands::hook_validate::run(),
        },
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
