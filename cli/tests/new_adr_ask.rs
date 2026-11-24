//! `write new adr` asks whether the decision contradicts or partly reverses a
//! record in force, with the count and how to list them, in text and JSON; any
//! other type prints what it always printed.

use serde_json::Value;
use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, write};

fn adr(number: &str, status: &str, extra: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: Rule {number}\ndescription: d\nstatus: {status}\n{extra}---\n\n# {number}. Rule {number}\n\nabout widgets\n"
    )
}

fn write_bundle(bundle: &Path) {
    write(bundle, "adr/0001-a.md", &adr("0001", "Accepted", ""));
    write(bundle, "adr/0002-b.md", &adr("0002", "Accepted", ""));
    write(bundle, "adr/0003-c.md", &adr("0003", "Proposed", ""));
    write(
        bundle,
        "adr/0004-d.md",
        &adr("0004", "Superseded", "superseded_by: \"0003\"\n"),
    );
    write(bundle, "adr/0005-e.md", &adr("0005", "Deprecated", ""));
}

fn new(bundle: &Path, flag: &str, doc_type: &str, title: &str) -> Output {
    living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap()])
        .args(["write", "new", doc_type, title, flag])
        .output()
        .expect("failed to run living-docs write new")
}

fn json_of(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

#[test]
fn a_new_adr_in_json_adds_the_count_and_the_question_after_the_existing_fields() {
    let bundle = common::temp_bundle("new-adr-ask", "json");
    write_bundle(&bundle);

    let output = new(&bundle, "--json", "adr", "X");

    assert!(output.status.success());
    let doc = json_of(&output);
    assert_eq!(doc["in_force"], 3);
    let ask = doc["ask"].as_str().expect("ask text");
    assert!(ask.contains("contradict"), "{ask}");
    assert!(ask.contains("living-docs read records"), "{ask}");
    assert_eq!(ask, living_docs_core::commands::effective::adr_ask());
    let keys: Vec<&String> = doc.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["path", "instruction", "indexed", "in_force", "ask"]);
}

#[test]
fn a_new_adr_in_text_prints_one_question_line_on_stderr() {
    let bundle = common::temp_bundle("new-adr-ask", "plain");
    write_bundle(&bundle);

    let output = new(&bundle, "--plain", "adr", "X");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("3 records in force"), "got:\n{stderr}");
    assert!(stderr.contains("contradict"), "got:\n{stderr}");
    assert!(stderr.contains("partly reverse"), "got:\n{stderr}");
    assert!(
        stderr.contains("`living-docs read records`"),
        "got:\n{stderr}"
    );
    assert!(!stderr.contains("Rule 000"), "names no record:\n{stderr}");
}

#[test]
fn quiet_silences_the_question_line() {
    let bundle = common::temp_bundle("new-adr-ask", "quiet");
    write_bundle(&bundle);

    let output = living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap(), "--quiet"])
        .args(["write", "new", "adr", "X", "--plain"])
        .output()
        .expect("failed to run living-docs write new");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("in force"), "got:\n{stderr}");
}

#[test]
fn a_new_issue_keeps_its_json_unchanged() {
    let bundle = common::temp_bundle("new-adr-ask", "issue");
    write_bundle(&bundle);

    let output = new(&bundle, "--json", "issue", "Y");

    assert!(output.status.success());
    let doc = json_of(&output);
    assert!(doc.get("in_force").is_none(), "{doc}");
    assert!(doc.get("ask").is_none(), "{doc}");
    let keys: Vec<&String> = doc.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["path", "instruction", "indexed"]);
}
