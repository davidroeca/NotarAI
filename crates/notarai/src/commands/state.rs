use clap::Subcommand;

#[derive(Subcommand)]
pub enum StateAction {
    /// Show the current reconciliation state
    Show,
    /// Reset reconciliation state (next run will be a full reconciliation)
    Reset,
    /// Snapshot current cache into reconciliation_state.json
    Snapshot,
}

pub fn run(action: StateAction) -> i32 {
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    match action {
        StateAction::Show => show(&root),
        StateAction::Reset => reset(&root),
        StateAction::Snapshot => snapshot(&root),
    }
}

fn show(root: &std::path::Path) -> i32 {
    match crate::core::state::load_state(root) {
        Ok(Some(state)) => {
            let meta = &state.last_reconciliation;
            println!("Last reconciliation: {}", meta.timestamp);
            if let Some(ref hash) = meta.git_hash {
                println!("Git hash:            {hash}");
            }
            if let Some(ref branch) = meta.branch {
                println!("Branch:              {branch}");
            }
            println!("Files:               {}", state.file_fingerprints.len());
            println!("Specs:               {}", state.spec_fingerprints.len());
            0
        }
        Ok(None) => {
            println!("No reconciliation state found.");
            0
        }
        Err(e) => {
            eprintln!("Error loading state: {e}");
            1
        }
    }
}

fn reset(root: &std::path::Path) -> i32 {
    let path = crate::core::state::state_path(root);
    if path.exists() {
        match std::fs::remove_file(&path) {
            Ok(()) => {
                println!("Reconciliation state reset.");
                0
            }
            Err(e) => {
                eprintln!("Error deleting state file: {e}");
                1
            }
        }
    } else {
        println!("No reconciliation state to reset.");
        0
    }
}

fn snapshot(root: &std::path::Path) -> i32 {
    match crate::core::state::snapshot_from_cache(root) {
        Ok(state) => match crate::core::state::save_state(root, &state) {
            Ok(()) => {
                let meta = &state.last_reconciliation;
                println!("Snapshot saved.");
                println!("Timestamp: {}", meta.timestamp);
                if let Some(ref hash) = meta.git_hash {
                    println!("Git hash:  {hash}");
                }
                println!("Files:     {}", state.file_fingerprints.len());
                println!("Specs:     {}", state.spec_fingerprints.len());
                0
            }
            Err(e) => {
                eprintln!("Error saving state: {e}");
                1
            }
        },
        Err(e) => {
            eprintln!("Error creating snapshot: {e}");
            1
        }
    }
}
