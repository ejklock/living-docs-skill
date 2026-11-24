//! An empty `read records --topic` answer says how many records are in force
//! and that a word match is not a conflict check, in text and in JSON, while a
//! matching topic keeps its output unchanged.

use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, write};

fn adr(number: &str, status: &str, extra: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: Rule {number}\ndescription: d\nstatus: {status}\n{extra}---\n\n# {number}. Rule {number}\n\nabout widgets\n"
    )
}

fn write_bundle(bundle: &Path) {
    write(bundle, "adr/0001-a.md", &adr("0001", "Accepted", ""));
    write(bundle, "adr/0002-b.md", &adr("0002", "Accepted", ""));
    write(bundle, "adr/0003-c.md", &adr("0003", "Accepted", ""));
    write(
        bundle,
        "adr/0004-d.md",
        &adr("0004", "Superseded", "superseded_by: \"0003\"\n"),
    );
}

fn read(bundle: &Path, mode: &str, topic: &str) -> Output {
    living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap()])
        .args(["read", "records", mode, "--topic", topic])
        .output()
        .expect("failed to run living-docs read")
}

#[test]
fn an_empty_topic_names_the_records_in_force_on_stderr() {
    let bundle = common::temp_bundle("read-empty-topic", "plain");
    write_bundle(&bundle);

    let output = read(&bundle, "--plain", "zzqq");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no live record mentions 'zzqq' (1 retired withheld)"));
    assert!(stderr.contains("3 records in force"), "got:\n{stderr}");
    assert!(stderr.contains("not a conflict check"), "got:\n{stderr}");
    assert!(
        stderr.contains("`living-docs read records`"),
        "got:\n{stderr}"
    );
    assert!(stderr.contains("grep -ril -e 'zzqq'"), "got:\n{stderr}");
    assert_eq!(stderr.lines().count(), 1, "one hint line:\n{stderr}");
}

#[test]
fn an_empty_topic_in_json_adds_the_count_and_a_note_and_keeps_the_rest() {
    let bundle = common::temp_bundle("read-empty-topic", "json");
    write_bundle(&bundle);

    let output = read(&bundle, "--json", "zzqq");

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout_of(&output)).expect("json");
    assert_eq!(json["in_force"], 3);
    assert_eq!(json["withheld"], 1);
    assert_eq!(json["records"], serde_json::json!([]));
    let note = json["note"].as_str().expect("note string");
    assert!(note.contains("`living-docs read records`"), "note: {note}");
    assert!(note.contains("not a conflict check"), "note: {note}");
}

#[test]
fn an_empty_topic_under_all_still_counts_only_the_records_in_force() {
    let bundle = common::temp_bundle("read-empty-topic", "all");
    write_bundle(&bundle);

    let output = living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap()])
        .args(["read", "records", "--json", "--all", "--topic", "zzqq"])
        .output()
        .expect("failed to run living-docs read");

    let json: serde_json::Value = serde_json::from_str(&stdout_of(&output)).expect("json");
    assert_eq!(json["in_force"], 3);
    assert_eq!(json["withheld"], 0);
    assert_eq!(json["records"], serde_json::json!([]));
}

#[test]
fn the_note_is_the_one_the_core_writes_with_the_topic_and_path_shell_quoted() {
    let bundle = common::temp_bundle("read-empty-topic", "note");
    write_bundle(&bundle);

    let output = read(&bundle, "--json", "it's");

    let json: serde_json::Value = serde_json::from_str(&stdout_of(&output)).expect("json");
    let expected = living_docs_core::commands::effective::empty_topic_note(3, "it's", &bundle);
    assert_eq!(json["note"], expected);
    assert!(
        expected.contains("grep -ril -e 'it'\\''s' '"),
        "got: {expected}"
    );
}

#[test]
fn a_matching_topic_keeps_its_output_without_the_count_or_note() {
    let bundle = common::temp_bundle("read-empty-topic", "hit");
    write_bundle(&bundle);

    let output = read(&bundle, "--json", "widget");

    assert!(output.status.success());
    let stdout = stdout_of(&output);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert!(json.get("in_force").is_none(), "got: {stdout}");
    assert!(json.get("note").is_none(), "got: {stdout}");
    assert_eq!(json["records"].as_array().map(Vec::len), Some(3));
    assert!(output.stderr.is_empty(), "no hint on a match");
}
