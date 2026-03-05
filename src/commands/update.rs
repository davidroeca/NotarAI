use crate::core::update::{self, InstallMethod, UpdateStatus};

fn print_status(status: &UpdateStatus) {
    eprintln!("Current version: {}", status.current);
    eprintln!("Latest version:  {}", status.latest);
    if status.update_available {
        eprintln!("Update available!");
    } else {
        eprintln!("You are up to date.");
    }
}

fn print_install_instructions(method: &InstallMethod) {
    match method {
        InstallMethod::CargoInstall => {
            eprintln!();
            eprintln!("Run: cargo install notarai");
        }
        InstallMethod::DevBuild => {
            eprintln!();
            eprintln!("Run: cargo install --path .");
        }
        InstallMethod::GithubRelease => {}
    }
}

pub fn run(check_only: bool) -> i32 {
    let status = match update::check_for_update_no_cache() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error checking for updates: {e}");
            return 1;
        }
    };

    print_status(&status);

    if !status.update_available || check_only {
        return 0;
    }

    let method = update::detect_install_method();

    match method {
        InstallMethod::CargoInstall | InstallMethod::DevBuild => {
            print_install_instructions(&method);
            0
        }
        InstallMethod::GithubRelease => match update::download_and_replace(&status.latest) {
            Ok(()) => {
                eprintln!("Updated to v{}", status.latest);
                0
            }
            Err(e) => {
                eprintln!("Update failed: {e}");
                1
            }
        },
    }
}

pub fn passive_update_hint() {
    let status = match update::check_for_update() {
        Ok(s) => s,
        Err(_) => return,
    };

    if status.update_available {
        eprintln!(
            "hint: notarai v{} is available (current: v{}). Run `notarai update` to update.",
            status.latest, status.current
        );
    }
}
