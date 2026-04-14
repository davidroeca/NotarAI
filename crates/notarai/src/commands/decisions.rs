use crate::core::decisions::DecisionLog;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum DecisionsAction {
    /// List decision proposals
    List {
        /// Filter by status: proposed, accepted, rejected
        #[arg(long)]
        status: Option<String>,
    },
    /// Accept a proposed decision (appends to spec)
    Accept {
        /// Spec path (relative to project root)
        spec: String,
        /// Proposal index (from `decisions list`)
        index: usize,
    },
    /// Reject a proposed decision
    Reject {
        /// Spec path (relative to project root)
        spec: String,
        /// Proposal index (from `decisions list`)
        index: usize,
        /// Reason for rejection
        #[arg(long)]
        reason: Option<String>,
    },
}

pub fn run(action: DecisionsAction) -> i32 {
    let project_root = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: could not determine current directory: {e}");
            return 1;
        }
    };
    let notarai_dir = project_root.join(".notarai");
    if !notarai_dir.exists() {
        eprintln!("Error: .notarai/ directory not found. Run `notarai init` first.");
        return 2;
    }

    match action {
        DecisionsAction::List { status } => run_list(&project_root, status.as_deref()),
        DecisionsAction::Accept { spec, index } => run_accept(&project_root, &spec, index),
        DecisionsAction::Reject {
            spec,
            index,
            reason,
        } => run_reject(&project_root, &spec, index, reason),
    }
}

fn run_list(project_root: &std::path::Path, status: Option<&str>) -> i32 {
    let log = DecisionLog::load(project_root);
    let filtered = log.filter_by_status(status);

    if filtered.is_empty() {
        let qualifier = status
            .map(|s| format!(" with status '{s}'"))
            .unwrap_or_default();
        println!("No decisions{qualifier} found.");
        return 0;
    }

    println!("{:<5} {:<10} {:<40} Choice", "Index", "Status", "Spec");
    println!("{}", "-".repeat(80));
    for (idx, proposal) in &filtered {
        let choice_preview = if proposal.choice.len() > 40 {
            format!("{}...", &proposal.choice[..37])
        } else {
            proposal.choice.clone()
        };
        println!(
            "{:<5} {:<10} {:<40} {}",
            idx, proposal.status, proposal.spec_path, choice_preview
        );
    }
    println!("\n{} decision(s) shown.", filtered.len());
    0
}

fn run_accept(project_root: &std::path::Path, spec: &str, index: usize) -> i32 {
    let mut log = DecisionLog::load(project_root);

    // Verify the proposal matches the specified spec.
    if let Some(proposal) = log.proposals.get(index)
        && proposal.spec_path != spec
    {
        eprintln!(
            "Error: proposal at index {index} belongs to '{}', not '{spec}'",
            proposal.spec_path
        );
        return 1;
    }

    match log.accept(index, project_root) {
        Ok(accepted) => {
            if let Err(e) = log.save(project_root) {
                eprintln!("Error saving decision log: {e}");
                return 1;
            }
            println!(
                "Accepted decision for {}: {}",
                accepted.spec_path, accepted.choice
            );
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

fn run_reject(
    project_root: &std::path::Path,
    spec: &str,
    index: usize,
    reason: Option<String>,
) -> i32 {
    let mut log = DecisionLog::load(project_root);

    // Verify the proposal matches the specified spec.
    if let Some(proposal) = log.proposals.get(index)
        && proposal.spec_path != spec
    {
        eprintln!(
            "Error: proposal at index {index} belongs to '{}', not '{spec}'",
            proposal.spec_path
        );
        return 1;
    }

    match log.reject(index, reason) {
        Ok(rejected) => {
            if let Err(e) = log.save(project_root) {
                eprintln!("Error saving decision log: {e}");
                return 1;
            }
            println!(
                "Rejected decision for {}: {}",
                rejected.spec_path, rejected.choice
            );
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}
