use crate::core::check::{
    CheckConfig, CheckFinding, CheckResult, CheckType, Severity, SeverityTier, tier_for_check_type,
};
use crate::core::lint::{LintConfig, LintRuleId, LintSeverity};

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

    let check_config = CheckConfig::load(&project_root);

    let mut result = match crate::core::check::run_all_checks(&project_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };

    // Run lint rules and merge non-overlapping findings into check results.
    let lint_config = LintConfig::load(&project_root);
    if let Ok(lint_findings) = crate::core::lint::run_all_lints(&project_root, &lint_config) {
        for lf in lint_findings {
            // Skip L002/L003 (overlap with BehaviorIncomplete) and L005 (overlap with CircularRef).
            if matches!(
                lf.rule_id,
                LintRuleId::L002 | LintRuleId::L003 | LintRuleId::L005
            ) {
                continue;
            }
            let ct = CheckType::LintViolation(lf.rule_id);
            let tier = tier_for_check_type(&ct);
            result.findings.push(CheckFinding {
                check_type: ct,
                severity: match lf.severity {
                    LintSeverity::Error => Severity::Error,
                    LintSeverity::Warning => Severity::Warning,
                    // Info-level lint findings are mapped to warnings in check context.
                    LintSeverity::Info => Severity::Warning,
                },
                tier,
                spec_path: Some(lf.spec_path),
                file_path: None,
                glob_pattern: None,
                message: lf.message,
            });
        }
    }

    // Filter findings by warn_on threshold (suppress tiers below it).
    result
        .findings
        .retain(|f| (f.tier as u8) <= (check_config.warn_on as u8));

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

    should_fail(&result, &check_config, strict)
}

/// Determine exit code based on check config and findings.
fn should_fail(result: &CheckResult, config: &CheckConfig, strict: bool) -> i32 {
    if strict {
        // --strict: any finding = failure.
        return if result.findings.is_empty() { 0 } else { 1 };
    }
    if let Some(fail_tier) = config.fail_on {
        // fail_on config: fail if any finding at or above the configured tier.
        let has_failing = result
            .findings
            .iter()
            .any(|f| (f.tier as u8) <= (fail_tier as u8));
        return if has_failing { 1 } else { 0 };
    }
    // Default: fail only on error-severity findings.
    let has_errors = result
        .findings
        .iter()
        .any(|f| matches!(f.severity, Severity::Error));
    if has_errors { 1 } else { 0 }
}

fn print_human(result: &CheckResult) {
    if result.findings.is_empty() {
        println!("All checks passed.");
        return;
    }

    let tiers = [
        SeverityTier::Critical,
        SeverityTier::Drift,
        SeverityTier::Housekeeping,
    ];
    let tier_colors: [&str; 3] = ["\x1b[31m", "\x1b[33m", "\x1b[36m"];

    // Check type groups within each tier.
    let groups: &[(CheckType, &str)] = &[
        (CheckType::CircularRef, "Circular $ref Cycles"),
        (CheckType::OrphanedGlob, "Orphaned Globs"),
        (
            CheckType::ChangedSinceReconciliation,
            "Changed Since Last Reconciliation",
        ),
        (CheckType::CoverageGap, "Coverage Gaps"),
        (CheckType::OverlappingCoverage, "Overlapping Coverage"),
        (CheckType::BehaviorIncomplete, "Incomplete Behaviors"),
        (CheckType::TestPathMissing, "Test Paths Missing"),
        (CheckType::TestCoverageMissing, "Test Coverage Missing"),
    ];

    for (tier_idx, tier) in tiers.iter().enumerate() {
        let tier_findings: Vec<&CheckFinding> =
            result.findings.iter().filter(|f| &f.tier == tier).collect();
        if tier_findings.is_empty() {
            continue;
        }

        let color = tier_colors[tier_idx];
        println!(
            "{color}--- {} ({} findings) ---\x1b[0m",
            tier.label(),
            tier_findings.len()
        );

        // Print core check findings grouped by type within this tier.
        for (check_type, label) in groups {
            let findings: Vec<&&CheckFinding> = tier_findings
                .iter()
                .filter(|f| &f.check_type == check_type)
                .collect();
            if findings.is_empty() {
                continue;
            }

            println!("  {label} ({} findings)", findings.len());
            for f in &findings {
                let detail = f
                    .file_path
                    .as_deref()
                    .or(f.glob_pattern.as_deref())
                    .or(f.spec_path.as_deref())
                    .unwrap_or("(unknown)");
                let prefix = match f.severity {
                    Severity::Warning => "\x1b[33m    warning\x1b[0m",
                    Severity::Error => "\x1b[31m    error  \x1b[0m",
                };
                println!("{prefix}: {detail}");
                // Only show the `in {spec}` locator when it adds information:
                // skip it when the detail is already the spec path (would be
                // a redundant repeat) and when CircularRef formatting owns the
                // message line below.
                if !matches!(check_type, CheckType::CircularRef)
                    && let Some(spec) = &f.spec_path
                    && spec.as_str() != detail
                {
                    println!("            in {spec}");
                }
                // Surface the finding message when the detail alone doesn't
                // identify the problem (e.g. the behavior name in T001).
                if matches!(
                    check_type,
                    CheckType::CircularRef
                        | CheckType::BehaviorIncomplete
                        | CheckType::TestCoverageMissing
                ) {
                    println!("            {}", f.message);
                }
            }
        }

        // Print lint findings within this tier.
        let lint_in_tier: Vec<&&CheckFinding> = tier_findings
            .iter()
            .filter(|f| matches!(f.check_type, CheckType::LintViolation(_)))
            .collect();
        if !lint_in_tier.is_empty() {
            println!("  Lint Violations ({} findings)", lint_in_tier.len());
            for f in &lint_in_tier {
                let prefix = match f.severity {
                    Severity::Warning => "\x1b[33m    warning\x1b[0m",
                    Severity::Error => "\x1b[31m    error  \x1b[0m",
                };
                let rule = match &f.check_type {
                    CheckType::LintViolation(id) => id.as_str(),
                    _ => "",
                };
                println!("{prefix} [{rule}]: {}", f.message);
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

fn check_type_str(ct: &CheckType) -> String {
    match ct {
        CheckType::CoverageGap => "coverage_gap".to_string(),
        CheckType::OrphanedGlob => "orphaned_glob".to_string(),
        CheckType::ChangedSinceReconciliation => "changed_since_reconciliation".to_string(),
        CheckType::OverlappingCoverage => "overlapping_coverage".to_string(),
        CheckType::CircularRef => "circular_ref".to_string(),
        CheckType::BehaviorIncomplete => "behavior_incomplete".to_string(),
        CheckType::TestCoverageMissing => "test_coverage_missing".to_string(),
        CheckType::TestPathMissing => "test_path_missing".to_string(),
        CheckType::LintViolation(id) => format!("lint_{}", id.as_str().to_lowercase()),
    }
}

fn print_json(result: &CheckResult) {
    let findings: Vec<serde_json::Value> = result
        .findings
        .iter()
        .map(|f| {
            serde_json::json!({
                "type": check_type_str(&f.check_type),
                "severity": match f.severity {
                    Severity::Warning => "warning",
                    Severity::Error => "error",
                },
                "tier": f.tier.as_str(),
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
