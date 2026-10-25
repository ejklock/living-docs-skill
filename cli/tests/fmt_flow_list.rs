//! The flow lists the canonical pass writes read back as the same items
//! with the same YAML types: an item that would read differently when plain
//! is quoted, and a plain number stays a number.

use std::fs;
use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, run_check, stdout_of, temp_bundle, write};

fn adr(extra: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: A\ndescription: d.\nowner: o\nstatus: Accepted\n{extra}timestamp: 2026-01-01T00:00:00Z\n---\n\n# A\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n"
    )
}

fn fmt(bundle: &Path) -> Output {
    living_docs()
        .args(["fmt", bundle.to_str().unwrap(), "--plain"])
        .output()
        .expect("failed to run living-docs fmt")
}

fn line(bundle: &Path, key: &str) -> String {
    let text = fs::read_to_string(bundle.join("adr/0001-a.md")).unwrap();
    text.lines()
        .find(|l| l.starts_with(&format!("{key}:")))
        .unwrap_or_default()
        .to_owned()
}

/// Writes one record carrying `extra`, formats it twice, and returns the line
/// for `key`, requiring the second pass to change nothing.
fn formatted(label: &str, extra: &str, key: &str) -> String {
    let bundle = temp_bundle("fmt-flow-list", label);
    write(&bundle, "adr/0001-a.md", &adr(extra));
    assert!(fmt(&bundle).status.success(), "{extra}");
    let once = fs::read_to_string(bundle.join("adr/0001-a.md")).unwrap();
    assert!(fmt(&bundle).status.success(), "{extra}");
    assert_eq!(
        fs::read_to_string(bundle.join("adr/0001-a.md")).unwrap(),
        once
    );
    assert!(living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap(), "index"])
        .output()
        .unwrap()
        .status
        .success());
    let verdict = stdout_of(&run_check(&bundle));
    assert!(!verdict.contains("non-canonical"), "{extra}: {verdict}");
    line(&bundle, key)
}

#[test]
fn tags_that_hold_a_flow_indicator_keep_their_quotes_and_their_count() {
    assert_eq!(
        formatted("indicators", "tags: [\"a, b\", \"x]\"]\n", "tags"),
        "tags: [\"a, b\", \"x]\"]"
    );
}

#[test]
fn tags_that_read_as_another_type_when_plain_stay_strings() {
    let got = formatted(
        "typed",
        "tags: [\"~\", \"null\", \"true\", \"1.5\", \"{k}\"]\n",
        "tags",
    );
    assert_eq!(got, "tags: [\"1.5\", \"null\", \"true\", \"{k}\", \"~\"]");
}

#[test]
fn plain_tags_and_a_plain_number_tag_are_not_quoted() {
    assert_eq!(formatted("plain", "tags: [a, b]\n", "tags"), "tags: [a, b]");
    assert_eq!(
        formatted("number", "tags: [2024]\n", "tags"),
        "tags: [2024]"
    );
}

#[test]
fn a_tail_sequence_gets_the_same_quoting() {
    let got = formatted("tail", "aliases: [\"a, b\", \"~\"]\n", "aliases");
    assert_eq!(got, "aliases: [\"a, b\", \"~\"]");
}
