use crate::core::check::{CheckFinding, CheckResult, CheckType, Severity};

pub fn run(format: &str, _base_branch: &str) -> i32 {
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

    let result = match crate::core::check::run_all_checks(&project_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };

    match format {
        "json" => print_json(&result),
        _ => print_human(&result),
    }

    if has_errors(&result) { 1 } else { 0 }
}

fn has_errors(_result: &CheckResult) -> bool {
    // All current checks are warning-severity. When error-severity checks are
    // added (e.g. via a future lint integration), filter for them here.
    false
}

fn print_human(result: &CheckResult) {
    if result.findings.is_empty() {
        println!("All checks passed.");
        return;
    }

    let groups: &[(CheckType, &str)] = &[
        (CheckType::CoverageGap, "Coverage Gaps"),
        (CheckType::OrphanedGlob, "Orphaned Globs"),
        (
            CheckType::ChangedSinceReconciliation,
            "Changed Since Last Reconciliation",
        ),
        (CheckType::OverlappingCoverage, "Overlapping Coverage"),
    ];

    for (check_type, label) in groups {
        let findings: Vec<&CheckFinding> = result
            .findings
            .iter()
            .filter(|f| &f.check_type == check_type)
            .collect();
        if findings.is_empty() {
            continue;
        }

        println!("\x1b[33m{label}\x1b[0m ({} findings)", findings.len());
        for f in &findings {
            let detail = f
                .file_path
                .as_deref()
                .or(f.glob_pattern.as_deref())
                .unwrap_or("(unknown)");
            let prefix = match f.severity {
                Severity::Warning => "\x1b[33m  warning\x1b[0m",
            };
            println!("{prefix}: {detail}");
            if let Some(spec) = &f.spec_path {
                println!("          in {spec}");
            }
        }
        println!();
    }

    let warnings = result.findings.len();
    println!(
        "{warnings} finding{} total.",
        if warnings == 1 { "" } else { "s" }
    );
}

fn print_json(result: &CheckResult) {
    let findings: Vec<serde_json::Value> = result
        .findings
        .iter()
        .map(|f| {
            serde_json::json!({
                "type": match f.check_type {
                    CheckType::CoverageGap => "coverage_gap",
                    CheckType::OrphanedGlob => "orphaned_glob",
                    CheckType::ChangedSinceReconciliation => "changed_since_reconciliation",
                    CheckType::OverlappingCoverage => "overlapping_coverage",
                },
                "severity": match f.severity {
                    Severity::Warning => "warning",
                },
                "spec_path": f.spec_path,
                "file_path": f.file_path,
                "glob_pattern": f.glob_pattern,
                "message": f.message,
            })
        })
        .collect();

    let errors = 0; // No error-severity findings yet.
    let warnings = result.findings.len();

    let output = serde_json::json!({
        "findings": findings,
        "summary": {
            "errors": errors,
            "warnings": warnings,
        }
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    );
}
