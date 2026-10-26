//! A frontmatter key outside the typed columns whose value is the empty
//! string survives `fmt` as `key: ""`, byte-stable on a second pass. A
//! `timestamp` that is blank, null or a mapping is reseeded with an RFC 3339
//! stamp, also byte-stable on a second pass.

use std::fs;
use std::path::Path;

mod common;
use common::{living_docs, run_check, stdout_of, temp_bundle, write};

const RECORD: &str = "adr/0001-a.md";

fn adr(extra: &str, eol: &str) -> String {
    let text = format!(
        "---\ntimestamp: 2026-01-01T00:00:00Z\n{extra}status: Accepted\nowner: o\ndescription: d.\ntitle: A\ntype: ADR\n---\n\n# A\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n"
    );
    text.replace('\n', eol)
}

fn fmt(bundle: &Path) {
    let out = living_docs()
        .args(["fmt", bundle.to_str().unwrap(), "--plain"])
        .output()
        .expect("failed to run living-docs fmt");
    assert!(out.status.success(), "{}", stdout_of(&out));
}

/// Formats `extra` twice, requiring the second pass to change nothing and
/// `check` to stay clean, and returns the formatted record.
fn formatted(label: &str, extra: &str, eol: &str) -> String {
    formatted_record(label, &adr(extra, eol))
}

fn formatted_record(label: &str, record: &str) -> String {
    let bundle = temp_bundle("fmt-empty-tail", label);
    write(&bundle, RECORD, record);
    fmt(&bundle);
    let once = fs::read_to_string(bundle.join(RECORD)).unwrap();
    fmt(&bundle);
    assert_eq!(fs::read_to_string(bundle.join(RECORD)).unwrap(), once);
    assert!(living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap(), "index"])
        .output()
        .unwrap()
        .status
        .success());
    let verdict = stdout_of(&run_check(&bundle));
    assert!(!verdict.contains("non-canonical"), "{verdict}");
    once
}

fn has_line(text: &str, wanted: &str) -> bool {
    text.lines().any(|l| l == wanted)
}

#[test]
fn an_empty_double_quoted_tail_value_is_kept() {
    let text = formatted("double", "alias: \"\"\n", "\n");
    assert!(has_line(&text, "alias: \"\""), "{text}");
}

#[test]
fn an_empty_single_quoted_tail_value_is_written_double_quoted() {
    let text = formatted("single", "alias: ''\n", "\n");
    assert!(has_line(&text, "alias: \"\""), "{text}");
}

#[test]
fn a_tail_key_with_no_value_is_dropped() {
    let text = formatted("null", "alias:\n", "\n");
    assert!(!text.contains("alias"), "{text}");
}

#[test]
fn an_empty_item_in_a_tail_sequence_is_kept_as_an_empty_item() {
    let text = formatted("seq", "labels: [\"\", a]\n", "\n");
    assert!(has_line(&text, "labels: [\"\", a]"), "{text}");
}

#[test]
fn a_crlf_record_keeps_its_line_endings_and_the_empty_value() {
    let text = formatted("crlf", "alias: \"\"\n", "\r\n");
    assert!(
        has_line(&text.replace("\r\n", "\n"), "alias: \"\""),
        "{text}"
    );
    assert_eq!(text.matches('\n').count(), text.matches("\r\n").count());
}

fn assert_timestamp_seeded(label: &str, blank: &str) {
    let record = adr("", "\n").replace("timestamp: 2026-01-01T00:00:00Z", blank);
    let text = formatted_record(label, &record);
    let line = text
        .lines()
        .find_map(|l| l.strip_prefix("timestamp: "))
        .unwrap_or_else(|| panic!("no timestamp in {text}"));
    let stamp = line.trim_matches('"');
    assert!(
        stamp.len() >= 20 && stamp.contains('T') && stamp.ends_with('Z'),
        "{text}"
    );
}

#[test]
fn an_empty_timestamp_is_seeded() {
    assert_timestamp_seeded("ts-empty", "timestamp: \"\"");
}

#[test]
fn a_whitespace_only_timestamp_is_seeded() {
    assert_timestamp_seeded("ts-blank", "timestamp: \"  \"");
}

#[test]
fn a_null_timestamp_is_seeded() {
    assert_timestamp_seeded("ts-null", "timestamp:");
}

#[test]
fn a_tilde_timestamp_is_seeded() {
    assert_timestamp_seeded("ts-tilde", "timestamp: ~");
}

#[test]
fn a_mapping_timestamp_is_seeded() {
    assert_timestamp_seeded("ts-map", "timestamp: {a: b}");
}

#[test]
fn an_empty_tail_value_leaves_check_with_a_zero_exit() {
    let bundle = temp_bundle("fmt-empty-tail", "check-exit");
    write(&bundle, RECORD, &adr("alias: \"\"\n", "\n"));
    fmt(&bundle);
    let text = fs::read_to_string(bundle.join(RECORD)).unwrap();
    assert!(has_line(&text, "alias: \"\""), "{text}");
    assert_check_exits_zero(&bundle);
}

fn assert_check_exits_zero(bundle: &Path) {
    let args = ["--docs-dir", bundle.to_str().unwrap(), "index"];
    assert!(living_docs().args(args).output().unwrap().status.success());
    let verdict = run_check(bundle);
    assert!(verdict.status.success(), "{}", stdout_of(&verdict));
}

#[test]
fn a_scalar_empty_supersedes_is_dropped_with_a_zero_exit_and_a_clean_check() {
    let bundle = temp_bundle("fmt-empty-tail", "supersedes-check");
    write(&bundle, RECORD, &adr("supersedes: \"\"\n", "\n"));
    fmt(&bundle);
    let text = fs::read_to_string(bundle.join(RECORD)).unwrap();
    let expected = "---\ntype: ADR\ntitle: A\ndescription: d.\nowner: o\nstatus: Accepted\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n# A\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n";
    assert_eq!(text, expected);
    assert_check_exits_zero(&bundle);
}
