//! The output contract (ADR 0060): `check`/`read` print minified JSON of
//! their documented shape off a TTY, `--plain`/`NO_COLOR` never leak ANSI
//! escapes, `check` on a failing bundle exits 1, and an unknown verb exits
//! 2. `index`/`fmt`/`new`/`set`/`supersede` JSON coverage lives in
//! `cli/tests/output_contract_authoring.rs`, split out to stay under the
//! file-size cap.

use serde::Deserialize;
use std::process::{Command, Output};

mod common;
use common::{fixture, stdout_of, temp_bundle, write};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
        .args(args)
        .output()
        .unwrap_or_else(|_| panic!("failed to run living-docs {args:?}"))
}

#[derive(Deserialize)]
struct CheckJson {
    bundle: String,
    docs: usize,
    violations: Vec<ViolationJson>,
    #[allow(dead_code)]
    advisories: Vec<AdvisoryJson>,
    ok: bool,
}

#[derive(Deserialize)]
struct ViolationJson {
    #[allow(dead_code)]
    file: String,
    #[allow(dead_code)]
    message: String,
}

#[derive(Deserialize)]
struct AdvisoryJson {
    #[allow(dead_code)]
    file: String,
    #[allow(dead_code)]
    kind: String,
    #[allow(dead_code)]
    message: String,
}

#[test]
fn check_json_on_a_clean_bundle_matches_the_documented_shape() {
    let bundle = fixture("09-okf-canonical");
    let output = run(&["check", bundle.to_str().unwrap(), "--json"]);
    let stdout = stdout_of(&output);

    assert!(output.status.success(), "got:\n{stdout}");
    let parsed: CheckJson = serde_json::from_str(stdout.trim_end())
        .unwrap_or_else(|e| panic!("check --json emits valid JSON: {e}\ngot:\n{stdout}"));
    assert!(parsed.ok);
    assert!(parsed.violations.is_empty());
    assert!(parsed.docs > 0);
    assert!(!parsed.bundle.is_empty());
    assert_eq!(stdout.trim_end().lines().count(), 1);
}

#[test]
fn check_json_on_a_failing_bundle_reports_violations_and_exits_one() {
    let bundle = temp_bundle("output-contract", "check-fail");
    write(&bundle, "adr/0001-bad.md", "not a valid record at all");
    let output = run(&["check", bundle.to_str().unwrap(), "--json"]);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1), "got:\n{stdout}");
    let parsed: CheckJson = serde_json::from_str(stdout.trim_end())
        .unwrap_or_else(|e| panic!("check --json emits valid JSON: {e}\ngot:\n{stdout}"));
    assert!(!parsed.ok);
    assert!(!parsed.violations.is_empty());
}

#[test]
fn check_plain_never_leaks_ansi_escapes() {
    let bundle = fixture("09-okf-canonical");
    let output = run(&["check", bundle.to_str().unwrap(), "--plain"]);
    assert!(!stdout_of(&output).contains("\x1b["));
}

#[test]
fn check_no_color_env_never_leaks_ansi_escapes() {
    let bundle = fixture("09-okf-canonical");
    let output = Command::new(env!("CARGO_BIN_EXE_living-docs"))
        .args(["check", bundle.to_str().unwrap(), "--plain"])
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to run living-docs check");
    assert!(!stdout_of(&output).contains("\x1b["));
}

#[derive(Deserialize)]
struct ReadJson {
    withheld: usize,
    records: Vec<ReadRecordJson>,
}

#[derive(Deserialize)]
struct ReadRecordJson {
    #[serde(rename = "type")]
    doc_type: String,
    #[allow(dead_code)]
    number: Option<i32>,
    #[allow(dead_code)]
    title: String,
    #[allow(dead_code)]
    description: String,
    lineage: Option<String>,
}

#[test]
fn read_json_matches_the_documented_shape() {
    let bundle = temp_bundle("output-contract", "read");
    write(
        &bundle,
        "adr/0001-old.md",
        "---\ntype: ADR\ntitle: Old\ndescription: d\nstatus: Superseded\nsuperseded_by: \"0002\"\n---\n\n# 0001. Old\n\nbody\n",
    );
    write(
        &bundle,
        "adr/0002-new.md",
        "---\ntype: ADR\ntitle: New\ndescription: d\nstatus: Accepted\nsupersedes: \"0001\"\n---\n\n# 0002. New\n\nbody\n",
    );

    let output = run(&[
        "--docs-dir",
        bundle.to_str().unwrap(),
        "read",
        "records",
        "--json",
    ]);
    let stdout = stdout_of(&output);
    assert!(output.status.success(), "got:\n{stdout}");
    let parsed: ReadJson = serde_json::from_str(stdout.trim_end())
        .unwrap_or_else(|e| panic!("read --json emits valid JSON: {e}\ngot:\n{stdout}"));

    assert_eq!(parsed.withheld, 1);
    assert_eq!(parsed.records.len(), 1);
    assert_eq!(parsed.records[0].doc_type, "ADR");
    assert_eq!(
        parsed.records[0].lineage.as_deref(),
        Some("supersedes 0001")
    );
    assert_eq!(stdout.trim_end().lines().count(), 1);
}

#[test]
fn unknown_verb_exits_with_code_two() {
    let output = run(&["no-such-verb"]);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn explicit_color_always_with_plain_emits_ansi_escapes() {
    let bundle = fixture("09-okf-canonical");
    let output = run(&[
        "check",
        bundle.to_str().unwrap(),
        "--plain",
        "--color=always",
    ]);
    assert!(
        stdout_of(&output).contains("\x1b["),
        "got:\n{}",
        stdout_of(&output)
    );
}

#[test]
fn stdout_is_empty_when_a_verb_errors() {
    let output = run(&["check", "/no/such/docs/bundle"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stdout_of(&output).is_empty(),
        "got:\n{}",
        stdout_of(&output)
    );
}

#[test]
fn json_and_plain_together_exit_two() {
    let bundle = fixture("09-okf-canonical");
    let output = run(&["check", bundle.to_str().unwrap(), "--json", "--plain"]);
    assert_eq!(output.status.code(), Some(2));
}
