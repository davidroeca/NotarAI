use crate::core::scoring::{ScoringConfig, compute_scores};

pub fn run(format: &str, spec: Option<&str>) -> i32 {
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

    let config = ScoringConfig::load(&project_root);
    let result = match compute_scores(&project_root, &config, spec) {
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

    0 // Score is always informational.
}

fn print_human(result: &crate::core::scoring::OverallScore) {
    if result.specs.is_empty() {
        println!("No specs found.");
        return;
    }

    println!("{:<45} {:<7} Status", "Spec", "Score");
    println!("{}", "-".repeat(65));

    for s in &result.specs {
        let color = status_color(s.status);
        println!(
            "{:<45} {:<7.2} {color}{}\x1b[0m",
            s.spec_path, s.score, s.status
        );
    }

    println!();
    let color = status_color(result.status);
    println!(
        "Overall: {:.2} ({color}{}\x1b[0m)",
        result.overall, result.status
    );
}

fn status_color(status: &str) -> &'static str {
    match status {
        "healthy" => "\x1b[32m",
        "review" => "\x1b[33m",
        "overdue" => "\x1b[31m",
        _ => "",
    }
}

fn print_json(result: &crate::core::scoring::OverallScore) {
    let specs: Vec<serde_json::Value> = result
        .specs
        .iter()
        .map(|s| {
            serde_json::json!({
                "spec_path": s.spec_path,
                "score": (s.score * 100.0).round() / 100.0,
                "status": s.status,
            })
        })
        .collect();

    let output = serde_json::json!({
        "specs": specs,
        "overall": {
            "score": (result.overall * 100.0).round() / 100.0,
            "status": result.status,
        }
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    );
}
