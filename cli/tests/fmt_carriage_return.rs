//! A value holding a carriage return is written with the two-character escape
//! `\r` inside double quotes, so `fmt` and `set` accept it, a second pass is
//! byte-stable and `check` stays clean.

use std::fs;
use std::path::Path;

mod common;
use common::{living_docs, run_check, run_new, stdout_of, temp_bundle, write};

const RECORD: &str = "adr/0001-a.md";

fn adr(extra: &str, eol: &str) -> String {
    let text = format!(
        "---\ntimestamp: 2026-01-01T00:00:00Z\n{extra}status: Accepted\nowner: o\ndescription: d.\ntitle: A\ntype: ADR\n---\n\n# A\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n"
    );
    text.replace('\n', eol)
}

fn adr_with_description(description: &str, eol: &str) -> String {
    adr("", eol).replace("description: d.", &format!("description: {description}"))
}

fn fmt(bundle: &Path) {
    let out = living_docs()
        .args(["fmt", bundle.to_str().unwrap(), "--plain"])
        .output()
        .expect("failed to run living-docs fmt");
    assert!(out.status.success(), "{}", stdout_of(&out));
}

fn assert_clean(bundle: &Path) {
    let args = ["--docs-dir", bundle.to_str().unwrap(), "index"];
    assert!(living_docs().args(args).output().unwrap().status.success());
    let verdict = run_check(bundle);
    assert!(verdict.status.success(), "{}", stdout_of(&verdict));
    assert!(!stdout_of(&verdict).contains("non-canonical"));
}

/// Formats `extra` twice, requiring the second pass to change nothing and
/// `check` to stay clean, and returns the formatted record.
fn formatted(label: &str, record: &str) -> String {
    let bundle = temp_bundle("fmt-carriage-return", label);
    write(&bundle, RECORD, record);
    fmt(&bundle);
    let once = fs::read_to_string(bundle.join(RECORD)).unwrap();
    fmt(&bundle);
    assert_eq!(fs::read_to_string(bundle.join(RECORD)).unwrap(), once);
    assert_clean(&bundle);
    once
}

fn has_line(text: &str, wanted: &str) -> bool {
    text.lines().any(|l| l.trim_end_matches('\r') == wanted)
}

#[test]
fn a_double_quoted_description_with_a_carriage_return_is_kept() {
    let text = formatted("desc", &adr_with_description("\"a\\rb\"", "\n"));
    assert!(has_line(&text, "description: \"a\\rb\""), "{text}");
}

#[test]
fn a_tail_scalar_with_a_carriage_return_is_kept() {
    let text = formatted("tail", &adr("alias: \"a\\rb\"\n", "\n"));
    assert!(has_line(&text, "alias: \"a\\rb\""), "{text}");
}

#[test]
fn a_flow_item_with_a_carriage_return_is_kept() {
    let text = formatted("flow", &adr("tags: [\"a\\rb\", c]\n", "\n"));
    assert!(has_line(&text, "tags: [\"a\\rb\", c]"), "{text}");
}

#[test]
fn a_value_with_a_carriage_return_and_a_line_feed_is_kept() {
    let text = formatted("crlf-value", &adr("alias: \"a\\r\\nb\"\n", "\n"));
    assert!(has_line(&text, "alias: \"a\\r\\nb\""), "{text}");
}

#[test]
fn a_crlf_record_keeps_its_line_endings_and_the_escape() {
    let text = formatted("crlf", &adr_with_description("\"a\\rb\"", "\r\n"));
    assert!(has_line(&text, "description: \"a\\rb\""), "{text}");
    assert_eq!(text.matches('\n').count(), text.matches("\r\n").count());
    assert_eq!(text.matches('\r').count(), text.matches("\r\n").count());
}

#[test]
fn set_from_a_file_holding_a_carriage_return_writes_the_escape() {
    let docs = temp_bundle("fmt-carriage-return", "set");
    assert!(run_new(&docs, "adr", "T").status.success());
    let source = docs.join("value.txt");
    fs::write(&source, "a\rb").unwrap();
    let out = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "set", "adr/0001"])
        .args(["description", "--from", source.to_str().unwrap(), "--plain"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stdout_of(&out));
    let text = fs::read_to_string(docs.join("adr/0001-t.md")).unwrap();
    assert!(has_line(&text, "description: \"a\\rb\""), "{text}");
}
