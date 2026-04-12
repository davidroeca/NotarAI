use crate::core::check::{CheckFinding, CheckResult, CheckType, Severity};

pub fn run(format: &str, _base_branch: &str, strict: bool) -> i32 {
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

    let mut result = match crate::core::check::run_all_checks(&project_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };

    if strict {
        for f in &mut result.findings {
            if matches!(f.severity, Severity::Warning) {
                f.severity = Severity::Error;
            }
        }
    }

    match format {
        "json" => print_json(&result),
        _ => print_human(&result),
    }

    if has_errors(&result) { 1 } else { 0 }
}

fn has_errors(result: &CheckResult) -> bool {
    result
        .findings
        .iter()
        .any(|f| matches!(f.severity, Severity::Error))
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
        (CheckType::CircularRef, "Circular $ref Cycles"),
        (CheckType::BehaviorIncomplete, "Incomplete Behaviors"),
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
                .or(f.spec_path.as_deref())
                .unwrap_or("(unknown)");
            let prefix = match f.severity {
                Severity::Warning => "\x1b[33m  warning\x1b[0m",
                Severity::Error => "\x1b[31m  error  \x1b[0m",
            };
            println!("{prefix}: {detail}");
            if !matches!(check_type, CheckType::CircularRef)
                && let Some(spec) = &f.spec_path
            {
                println!("          in {spec}");
            }
            if matches!(
                check_type,
                CheckType::CircularRef | CheckType::BehaviorIncomplete
            ) {
                println!("          {}", f.message);
            }
        }
        println!();
    }

    let (errors, warnings) = count_severity(result);
    println!(
        "{} issue{} found ({} error{}, {} warning{}).",
        errors + warnings,
        if errors + warnings == 1 { "" } else { "s" },
        errors,
        if errors == 1 { "" } else { "s" },
        warnings,
        if warnings == 1 { "" } else { "s" },
    );
}

fn count_severity(result: &CheckResult) -> (usize, usize) {
    let mut errors = 0usize;
    let mut warnings = 0usize;
    for f in &result.findings {
        match f.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
        }
    }
    (errors, warnings)
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
                    CheckType::CircularRef => "circular_ref",
                    CheckType::BehaviorIncomplete => "behavior_incomplete",
                },
                "severity": match f.severity {
                    Severity::Warning => "warning",
                    Severity::Error => "error",
                },
                "spec_path": f.spec_path,
                "file_path": f.file_path,
                "glob_pattern": f.glob_pattern,
                "message": f.message,
            })
        })
        .collect();

    let (errors, warnings) = count_severity(result);

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
