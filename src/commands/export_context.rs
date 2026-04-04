use crate::core::export;

pub fn run(spec: Option<&str>, all: bool, bootstrap: bool, base_branch: &str, format: &str) -> i32 {
    let project_root = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: could not determine working directory: {e}");
            return 1;
        }
    };

    if bootstrap {
        // Warn if specs already exist; bootstrap is a pre-init command.
        let notarai_dir = project_root.join(".notarai");
        if notarai_dir.exists() {
            let has_specs = std::fs::read_dir(&notarai_dir)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .any(|e| e.file_name().to_string_lossy().ends_with(".spec.yaml"))
                })
                .unwrap_or(false);
            if has_specs {
                eprintln!(
                    "Warning: .notarai/ already contains spec files. \
                     Use export-context --all to reconcile existing specs instead."
                );
            }
        }
        print!("{}", export::render_bootstrap());
        return 0;
    }

    if !project_root.join(".notarai").exists() {
        eprintln!("Error: .notarai/ not found. Run `notarai init` first.");
        return 2;
    }

    match (spec, all) {
        (Some(_), true) | (None, false) => {
            eprintln!("Error: specify exactly one of --spec <path>, --all, or --bootstrap.");
            return 1;
        }
        _ => {}
    }

    let contexts = if let Some(spec_path) = spec {
        if !project_root.join(spec_path).exists() {
            eprintln!("Error: spec file not found: {spec_path}");
            return 1;
        }
        match export::build_context(spec_path, base_branch, &project_root) {
            Ok(ctx) => vec![ctx],
            Err(e) => {
                eprintln!("Error: {e}");
                return 1;
            }
        }
    } else {
        match export::build_all_contexts(base_branch, &project_root) {
            Ok(ctxs) => ctxs,
            Err(e) => {
                eprintln!("Error: {e}");
                return 1;
            }
        }
    };

    match format {
        "json" => print_json(&contexts),
        _ => print_markdown(&contexts),
    }

    0
}

fn print_markdown(contexts: &[export::ExportContext]) {
    for (i, ctx) in contexts.iter().enumerate() {
        if i > 0 {
            println!("\n---\n");
        }
        println!("{}", export::render_markdown(ctx));
    }
}

fn print_json(contexts: &[export::ExportContext]) {
    let json: Vec<serde_json::Value> = contexts.iter().map(export::render_json).collect();
    if json.len() == 1 {
        println!(
            "{}",
            serde_json::to_string_pretty(&json[0]).expect("JSON serialization failed")
        );
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&json).expect("JSON serialization failed")
        );
    }
}
