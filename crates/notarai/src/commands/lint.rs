use crate::core::lint::{LintConfig, LintFinding, LintRuleId, LintSeverity};

pub fn run(format: &str) -> i32 {
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

    let config = LintConfig::load(&project_root);

    let findings = match crate::core::lint::run_all_lints(&project_root, &config) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };

    match format {
        "json" => print_json(&findings),
        _ => print_human(&findings),
    }

    if has_errors(&findings) { 1 } else { 0 }
}

fn has_errors(findings: &[LintFinding]) -> bool {
    findings
        .iter()
        .any(|f| matches!(f.severity, LintSeverity::Error))
}

fn print_human(findings: &[LintFinding]) {
    if findings.is_empty() {
        println!("All lint checks passed.");
        return;
    }

    let rules: &[(LintRuleId, &str)] = &[
        (LintRuleId::L001, "L001: Tier 1 spec with no behaviors"),
        (LintRuleId::L002, "L002: Behavior missing given"),
        (LintRuleId::L003, "L003: Behavior missing then"),
        (LintRuleId::L004, "L004: $ref target missing"),
        (LintRuleId::L005, "L005: Circular $ref"),
        (LintRuleId::L006, "L006: Stale decision without rationale"),
        (LintRuleId::L007, "L007: Open questions"),
        (LintRuleId::L008, "L008: Overly broad glob"),
        (LintRuleId::L009, "L009: Schema version mismatch"),
        (LintRuleId::L010, "L010: Duplicate behavior names"),
    ];

    for (rule_id, label) in rules {
        let rule_findings: Vec<&LintFinding> =
            findings.iter().filter(|f| &f.rule_id == rule_id).collect();
        if rule_findings.is_empty() {
            continue;
        }

        println!(
            "\x1b[33m{label}\x1b[0m ({} finding{})",
            rule_findings.len(),
            if rule_findings.len() == 1 { "" } else { "s" }
        );
        for f in &rule_findings {
            let prefix = match f.severity {
                LintSeverity::Error => "\x1b[31m  error  \x1b[0m",
                LintSeverity::Warning => "\x1b[33m  warning\x1b[0m",
                LintSeverity::Info => "\x1b[36m  info   \x1b[0m",
            };
            println!("{prefix}: {}", f.message);
        }
        println!();
    }

    let (errors, warnings, infos) = count_severity(findings);
    println!(
        "{} finding{} ({} error{}, {} warning{}, {} info).",
        errors + warnings + infos,
        if errors + warnings + infos == 1 {
            ""
        } else {
            "s"
        },
        errors,
        if errors == 1 { "" } else { "s" },
        warnings,
        if warnings == 1 { "" } else { "s" },
        infos,
    );
}

fn count_severity(findings: &[LintFinding]) -> (usize, usize, usize) {
    let mut errors = 0usize;
    let mut warnings = 0usize;
    let mut infos = 0usize;
    for f in findings {
        match f.severity {
            LintSeverity::Error => errors += 1,
            LintSeverity::Warning => warnings += 1,
            LintSeverity::Info => infos += 1,
        }
    }
    (errors, warnings, infos)
}

fn print_json(findings: &[LintFinding]) {
    let json_findings: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| {
            serde_json::json!({
                "rule_id": f.rule_id.as_str(),
                "severity": f.severity.as_str(),
                "spec_path": f.spec_path,
                "message": f.message,
            })
        })
        .collect();

    let (errors, warnings, infos) = count_severity(findings);

    let output = serde_json::json!({
        "findings": json_findings,
        "summary": {
            "errors": errors,
            "warnings": warnings,
            "infos": infos,
        }
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    );
}
