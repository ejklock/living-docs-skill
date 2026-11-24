//! `write new adr` lists up to three in-force ADRs that share two or more
//! rare words with the new decision, after the question, in text and JSON;
//! it never lists a retired record and adds nothing when none qualifies.

use serde_json::Value;
use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, write};

const SHARING_BODY: &str = "Route through the shared state store.\n";

fn adr(number: &str, status: &str, title: &str, extra: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: {title}\ndescription: d\nstatus: {status}\n{extra}---\n\n# {number}. {title}\n\nabout widgets\n"
    )
}

fn write_bundle(bundle: &Path) {
    let shared = "Shared key-value store for login state";
    let old = "Shared state store of old";
    let records = [
        ("0001-a", adr("0001", "Accepted", "Rule one", "")),
        ("0002-b", adr("0002", "Accepted", "Rule two", "")),
        ("0003-c", adr("0003", "Accepted", shared, "")),
        (
            "0004-d",
            adr("0004", "Superseded", old, "superseded_by: \"0003\"\n"),
        ),
        ("0005-e", adr("0005", "Accepted", "Rule five", "")),
    ];
    for (name, text) in records {
        write(bundle, &format!("adr/{name}.md"), &text);
    }
}

fn new(bundle: &Path, body: &str, flag: &str, doc_type: &str) -> Output {
    let file = bundle.join("body.txt");
    std::fs::write(&file, body).unwrap();
    living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap()])
        .args(["write", "new", doc_type, "Sticky routing", flag])
        .args(["--body", file.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs write new")
}

fn json_of(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

#[test]
fn a_body_sharing_three_rare_words_lists_that_adr_after_the_question() {
    let bundle = common::temp_bundle("new-adr-candidates", "json");
    write_bundle(&bundle);

    let output = new(&bundle, SHARING_BODY, "--json", "adr");

    assert!(output.status.success());
    let doc = json_of(&output);
    let candidates = doc["candidates"].as_array().expect("candidates list");
    assert_eq!(candidates.len(), 1, "{doc}");
    assert_eq!(candidates[0]["type"], "ADR");
    assert_eq!(candidates[0]["number"], 3);
    let title = "Shared key-value store for login state";
    assert_eq!(candidates[0]["title"], title);
    let words = serde_json::json!(["shared", "state", "store"]);
    assert_eq!(candidates[0]["shared"], words);
    let keys: Vec<&String> = doc.as_object().unwrap().keys().collect();
    let expected = [
        "path",
        "instruction",
        "indexed",
        "in_force",
        "ask",
        "candidates",
    ];
    assert_eq!(keys, expected);
}

#[test]
fn text_mode_prints_one_stderr_line_per_candidate_naming_the_words() {
    let bundle = common::temp_bundle("new-adr-candidates", "plain");
    write_bundle(&bundle);

    let output = new(&bundle, SHARING_BODY, "--plain", "adr");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let marker = listed_line_marker();
    let lines: Vec<&str> = stderr.lines().filter(|l| l.contains(&marker)).collect();
    assert_eq!(lines.len(), 1, "got:\n{stderr}");
    assert!(lines[0].contains("shared, state, store"), "{}", lines[0]);
    assert!(!stderr.contains("conflict"), "got:\n{stderr}");
    let ask_at = stderr.find("in force").expect("ask line");
    assert!(ask_at < stderr.find(&marker).unwrap(), "got:\n{stderr}");
}

#[test]
fn a_body_sharing_one_rare_word_lists_nothing_and_adds_no_key() {
    let bundle = common::temp_bundle("new-adr-candidates", "one");
    write_bundle(&bundle);

    let output = new(
        &bundle,
        "Route through the shared cache.\n",
        "--json",
        "adr",
    );

    assert!(output.status.success());
    let doc = json_of(&output);
    assert!(doc.get("candidates").is_none(), "{doc}");
}

#[test]
fn a_superseded_record_is_never_listed() {
    let bundle = common::temp_bundle("new-adr-candidates", "retired");
    write_bundle(&bundle);

    let output = new(&bundle, SHARING_BODY, "--json", "adr");

    let doc = json_of(&output);
    let numbers: Vec<&Value> = doc["candidates"]
        .as_array()
        .expect("candidates list")
        .iter()
        .map(|c| &c["number"])
        .collect();
    assert_eq!(numbers, [3]);
}

#[test]
fn quiet_silences_the_candidate_lines() {
    let bundle = common::temp_bundle("new-adr-candidates", "quiet");
    write_bundle(&bundle);
    let file = bundle.join("body.txt");
    std::fs::write(&file, SHARING_BODY).unwrap();

    let output = living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap(), "--quiet"])
        .args(["write", "new", "adr", "Sticky routing", "--plain"])
        .args(["--body", file.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs write new");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains(&listed_line_marker()), "got:\n{stderr}");
}

/// How a candidate line names the shared-words record: type and padded number.
fn listed_line_marker() -> String {
    format!("{} {:04}", "ADR", 3)
}

#[test]
fn the_mcp_write_lists_the_same_candidates_as_the_cli() {
    let cli_bundle = common::temp_bundle("new-adr-candidates", "parity-cli");
    let mcp_bundle = common::temp_bundle("new-adr-candidates", "parity-mcp");
    write_bundle(&cli_bundle);
    write_bundle(&mcp_bundle);
    let from_cli = json_of(&new(&cli_bundle, SHARING_BODY, "--json", "adr"));

    let mut session = common::mcp_session::Session::start(&mcp_bundle);
    session.initialize();
    let arguments = serde_json::json!({
        "operation": "new", "type": "adr", "title": "Sticky routing",
        "owner": "Eve", "body": SHARING_BODY,
    });
    let result = session.call(2, "write", &arguments);
    let from_mcp = result["structuredContent"].clone();

    assert_eq!(from_mcp["candidates"], from_cli["candidates"]);
    let keys: Vec<&String> = from_mcp.as_object().unwrap().keys().collect();
    let expected = [
        "path",
        "instruction",
        "indexed",
        "in_force",
        "ask",
        "candidates",
    ];
    assert_eq!(keys, expected);
}

#[test]
fn a_new_issue_keeps_its_json_unchanged() {
    let bundle = common::temp_bundle("new-adr-candidates", "issue");
    write_bundle(&bundle);

    let output = new(&bundle, SHARING_BODY, "--json", "issue");

    assert!(output.status.success());
    let doc = json_of(&output);
    assert!(doc.get("candidates").is_none(), "{doc}");
    let keys: Vec<&String> = doc.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["path", "instruction", "indexed"]);
}
