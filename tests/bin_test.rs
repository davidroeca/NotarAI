use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn notarai() -> assert_cmd::Command {
    cargo_bin_cmd!("notarai")
}

#[test]
fn help_flag_exits_0_and_prints_usage() {
    notarai()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("notarai"));
}

#[test]
fn unknown_command_exits_2() {
    // clap exits with code 2 for parse errors
    notarai()
        .arg("nonexistent")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("notarai"));
}

#[test]
fn validate_exits_0_for_repo_notarai_dir() {
    notarai()
        .arg("validate")
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_exits_1_for_missing_path() {
    notarai()
        .args(["validate", "/nonexistent/path"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("path not found"));
}

#[test]
fn no_command_exits_1() {
    notarai()
        .assert()
        .code(1)
        .stdout(predicate::str::contains("notarai"));
}

const VALID_SPEC_YAML: &str = "\
schema_version: \"0.5\"
intent: \"Test spec\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/foo.ts\"
      role: \"test\"
";

const VALID_SPEC_V06: &str = "\
schema_version: \"0.6\"
intent: \"Test spec v0.6\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/foo.ts\"
      role: \"test\"
";

#[test]
fn hook_validate_exits_0_for_non_spec_file() {
    notarai()
        .args(["hook", "validate"])
        .write_stdin(r#"{"tool_input":{"file_path":"/tmp/foo.ts"}}"#)
        .assert()
        .success();
}

#[test]
fn hook_validate_exits_0_for_malformed_json() {
    notarai()
        .args(["hook", "validate"])
        .write_stdin("not json!")
        .assert()
        .success();
}

#[test]
fn hook_validate_exits_1_for_invalid_spec() {
    let tmp = TempDir::new().unwrap();
    let spec_dir = tmp.path().join(".notarai");
    fs::create_dir_all(&spec_dir).unwrap();
    let spec_path = spec_dir.join("test.spec.yaml");
    fs::write(&spec_path, "schema_version: \"0.5\"\n").unwrap();

    let input = serde_json::json!({
        "tool_input": { "file_path": spec_path.to_str().unwrap() }
    });
    notarai()
        .args(["hook", "validate"])
        .current_dir(tmp.path())
        .write_stdin(input.to_string())
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Spec validation failed"));
}

#[test]
fn hook_validate_exits_0_for_valid_spec() {
    let tmp = TempDir::new().unwrap();
    let spec_dir = tmp.path().join(".notarai");
    fs::create_dir_all(&spec_dir).unwrap();
    let spec_path = spec_dir.join("test.spec.yaml");
    fs::write(&spec_path, VALID_SPEC_YAML).unwrap();

    let input = serde_json::json!({
        "tool_input": { "file_path": spec_path.to_str().unwrap() }
    });
    notarai()
        .args(["hook", "validate"])
        .current_dir(tmp.path())
        .write_stdin(input.to_string())
        .assert()
        .success();
}

#[test]
fn validate_exits_0_with_warning_for_empty_spec_dir() {
    let tmp = TempDir::new().unwrap();
    let spec_dir = tmp.path().join(".notarai");
    fs::create_dir_all(&spec_dir).unwrap();

    notarai()
        .args(["validate", spec_dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("no .spec.yaml files"));
}

#[test]
fn validate_warns_when_local_schema_is_stale() {
    let tmp = TempDir::new().unwrap();

    let spec_dir = tmp.path().join(".notarai");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(spec_dir.join("test.spec.yaml"), VALID_SPEC_YAML).unwrap();
    // Write a stale schema at the new location (.notarai/notarai.spec.json)
    fs::write(
        spec_dir.join("notarai.spec.json"),
        r#"{"$id":"https://notarai.dev/schema/0.3/spec.schema.json"}"#,
    )
    .unwrap();

    notarai()
        .arg("validate")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("out of date"));
}

fn write_spec(tmp: &TempDir, content: &str) -> std::path::PathBuf {
    let spec_dir = tmp.path().join(".notarai");
    fs::create_dir_all(&spec_dir).unwrap();
    let path = spec_dir.join("test.spec.yaml");
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn validate_accepts_v05_spec() {
    let tmp = TempDir::new().unwrap();
    write_spec(&tmp, VALID_SPEC_YAML);
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_v06_spec() {
    let tmp = TempDir::new().unwrap();
    write_spec(&tmp, VALID_SPEC_V06);
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_derived_tier() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
tier: derived
intent: \"Generated output spec\"
artifacts:
  code:
    - path: \"dist/**\"
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_artifact_tier() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"Spec with per-artifact tier\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"dist/bundle.js\"
      tier: 4
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_output_block() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"Presentation spec\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
output:
  type: presentation
  format: pptx
  runtime: static-file
  entry_point: dist/presentation.pptx
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_content_block() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"Spec with content structure\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
content:
  structure: ordered
  sections:
    - id: intro
      type: slide
      intent: Hook the audience with the core problem
    - id: demo
      type: interactive
      content_ref: slides/02-demo.md
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_behavior_interaction() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"Interactive spec\"
behaviors:
  - name: click_button
    given: user clicks the submit button
    then: form is submitted
    interaction:
      trigger: user_action
      sequence:
        - validate form fields
        - submit to API
        - show confirmation
artifacts:
  code:
    - path: \"src/**\"
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_behavior_state_transition() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"State machine spec\"
behaviors:
  - name: submit_form
    given: user submits valid form
    then: transitions to confirmed state
    state_transition:
      from: editing
      to: confirmed
artifacts:
  code:
    - path: \"src/**\"
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_states_block() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"Spec with state machine\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
states:
  initial: idle
  definitions:
    - id: idle
      transitions:
        - to: running
          on: start
    - id: running
      transitions:
        - to: idle
          on: stop
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_design_block() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"Branded presentation\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
design:
  theme:
    palette:
      - \"#1a1a2e\"
      - \"#16213e\"
    typography:
      heading: Inter
      body: Roboto
  layout:
    type: slide-deck
    dimensions: \"16:9\"
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_audience_block() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"Audience-aware spec\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
audience:
  role: Series B investors
  tone: formal-but-engaging
  locale: en-US
  accessibility:
    - high-contrast
    - screen-reader-friendly
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_variants_block() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"Multi-variant spec\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
variants:
  - id: investor-deck
    description: Condensed version for investor meetings
    overrides:
      audience.role: Series B investors
      content.sections: []
  - id: technical-deep-dive
    description: Full technical version
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_pipeline_block() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"Spec with build pipeline\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
pipeline:
  steps:
    - name: compile
      tool: tsc
      input: \"src/**/*.ts\"
      output: dist/
    - name: bundle
      tool: esbuild
      command: esbuild dist/index.js --bundle --outfile=out.js
  preview:
    command: npx serve dist/
    url: http://localhost:3000
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_feedback_block() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"Spec with feedback loop\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
feedback:
  metrics:
    - name: avg_completion_rate
      source: analytics/completion.csv
      threshold: \">= 0.7\"
    - name: build_time
      threshold: \"< 5s\"
  reconciliation_trigger: when avg_completion_rate drops below threshold for 3 consecutive days
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_rejects_invalid_output_type() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"Bad output type\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
output:
  type: invalid
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .code(1);
}

#[test]
fn validate_rejects_invalid_layout_type() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"Bad layout type\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
design:
  layout:
    type: invalid
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .code(1);
}

#[test]
fn validate_rejects_invalid_content_structure() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.6\"
intent: \"Bad content structure\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
content:
  structure: invalid
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .code(1);
}

// -- v0.7 schema validation tests ---------------------------------------------

#[test]
fn validate_accepts_v07_spec_with_compliance_and_section_extensions() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.7\"
domain: legal
intent: \"Legal contract with compliance mapping\"
behaviors:
  - name: dpa_clause
    given: customer data is processed
    then: DPA clause governs processing per GDPR Article 28
artifacts:
  docs:
    - path: \"contracts/**\"
compliance:
  frameworks:
    - name: GDPR
      controls:
        - id: Art28
          satisfied_by:
            invariants:
              - 'DPA clause must never be removed'
  audit_trail: true
content:
  structure: ordered
  sections:
    - id: intro
      type: clause
      intent: Define terms
      duration:
        value: 5
        unit: minutes
      connections:
        - to: body
          label: next
      depends_on: []
      evidence:
        - type: citation
          ref: 'GDPR 2016/679'
          claim: 'Processing obligations per GDPR Article 28'
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_v07_spec_with_design_extensions() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.7\"
intent: \"Report with print and responsive design\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  docs:
    - path: \"report/**\"
design:
  theme:
    palette:
      - \"#1a1a2e\"
    modes:
      light:
        palette:
          - \"#ffffff\"
      dark:
        palette:
          - \"#1a1a2e\"
  layout:
    type: paginated
    dimensions: letter
  print:
    margins:
      top: '1in'
      bottom: '1in'
    headers: true
    footers: true
    page_numbers: true
    bleed: '0.125in'
  responsive:
    breakpoints:
      - name: mobile
        max_width: 768
        layout_override: scrolling
      - name: desktop
        min_width: 769
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_v07_spec_with_pipeline_extensions() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.7\"
intent: \"Spec with extended pipeline\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
pipeline:
  env:
    NODE_ENV: production
  steps:
    - name: compile
      tool: tsc
      input: \"src/**/*.ts\"
      output: dist/
      condition: \"output.format == 'web'\"
    - name: export_pdf
      command: pandoc input.md -o output.pdf
      condition: \"output.format == 'pdf'\"
      on_failure: skip
      depends_on:
        - compile
      env:
        PANDOC_DATA_DIR: ./templates
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_v07_spec_with_state_guard_and_action() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.7\"
intent: \"Spec with state guards and actions\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
states:
  initial: idle
  definitions:
    - id: idle
      transitions:
        - to: running
          on: start
          guard: 'all required fields are populated'
          action: 'initialize timer, log start event'
    - id: running
      transitions:
        - to: idle
          on: stop
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_v07_spec_with_feedback_triggers() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.7\"
intent: \"Spec with structured feedback triggers\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
feedback:
  metrics:
    - name: avg_completion_rate
      source: analytics/completion.csv
      threshold: \">= 0.7\"
  triggers:
    - condition:
        metric: avg_completion_rate
        operator: below_threshold
        duration:
          value: 3
          unit: days
      action: reconcile
      priority: high
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_v06_spec_unchanged_backward_compat() {
    // A valid v0.6 spec with no new fields must still pass under v0.7 schema.
    let tmp = TempDir::new().unwrap();
    write_spec(&tmp, VALID_SPEC_V06);
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_variants_resolved_flag() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.7\"
intent: \"Spec with variants_resolved\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/**\"
variants:
  - id: investor
    description: Investor version
variants_resolved: true
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_new_output_types() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.7\"
intent: \"Legal document spec\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  docs:
    - path: \"docs/**\"
output:
  type: document
  format: pdf
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_accepts_new_domain_values() {
    let tmp = TempDir::new().unwrap();
    write_spec(
        &tmp,
        "\
schema_version: \"0.7\"
domain: research
intent: \"Research report spec\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  docs:
    - path: \"sections/**\"
",
    );
    notarai()
        .args(["validate", tmp.path().join(".notarai").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}
