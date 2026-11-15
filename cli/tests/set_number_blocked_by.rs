//! `set <issue> number next` repoints the renumbered issue in every other
//! issue's `blocked_by`, and leaves `blocked_by` alone when an ADR renumbers.

use std::fs;
use std::path::Path;
use std::process::Output;

mod common;
use common::{index_bundle, living_docs, run_check, temp_bundle, write};

fn issue(number: &str, title: &str, extra: &str) -> String {
    format!(
        "---\ntype: Issue\ntitle: {title}\ndescription: d\nstatus: open\n{extra}timestamp: 2026-10-24T10:00:00Z\n---\n\n## {number}. {title}\n\nBody.\n\n### Scope\n\ns\n\n### Acceptance\n\n- a\n"
    )
}

const ADR: &str = "---\ntype: ADR\ntitle: Cache\ndescription: d\nowner: o\nstatus: Accepted\ntimestamp: 2026-10-24T10:00:00Z\n---\n\n# 0001. Cache\n\nBody.\n\n## Context\n\nc\n\n## Decision\n\nd\n\n## Consequences\n\nq\n";

fn bundle(label: &str) -> common::Scratch {
    let docs = temp_bundle("set-number-blocked-by", label);
    let blocked = |number: &str, title: &str, list: &str| {
        let extra = format!("blocked_by: {list}\n");
        let path = format!("issues/{number}-{title}.md");
        write(&docs, &path, &issue(number, title, &extra));
    };
    write(&docs, "adr/0001-cache.md", ADR);
    write(&docs, "issues/0005-a.md", &issue("0005", "a", ""));
    write(&docs, "issues/0007-b.md", &issue("0007", "b", ""));
    write(&docs, "issues/0009-x.md", &issue("0009", "x", ""));
    blocked("0003", "c", "[9, 7, 5]");
    blocked("0004", "d", "[\"0007\"]");
    blocked("0006", "e", "[9, 5]");
    index_bundle(&docs);
    docs
}

fn number(docs: &Path, reference: &str) -> Output {
    living_docs()
        .arg("--docs-dir")
        .arg(docs)
        .args(["write", "set", reference, "number", "next", "--plain"])
        .output()
        .unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn read(docs: &Path, rel: &str) -> String {
    fs::read_to_string(docs.join(rel)).unwrap()
}

#[test]
fn renumbering_an_issue_rewrites_its_number_in_every_blocked_by_list() {
    let docs = bundle("issue");
    let untouched = read(&docs, "issues/0006-e.md");
    let output = number(&docs, "issues/0007-b.md");
    assert!(output.status.success(), "{}", stderr(&output));

    let first = read(&docs, "issues/0003-c.md");
    assert!(first.contains("blocked_by: [9, 10, 5]\n"), "{first}");
    let second = read(&docs, "issues/0004-d.md");
    assert!(second.contains("blocked_by: [10]\n"), "{second}");
    assert_eq!(read(&docs, "issues/0006-e.md"), untouched);

    let listed = stdout(&output);
    assert!(listed.contains("0003-c.md"), "{listed}");
    assert!(listed.contains("0004-d.md"), "{listed}");
    assert!(!listed.contains("0006-e.md"), "{listed}");
    let check = run_check(&docs);
    assert!(check.status.success(), "{check:?}");
}

#[test]
fn renumbering_an_adr_leaves_every_blocked_by_untouched() {
    let docs = bundle("adr");
    write(
        &docs,
        "issues/0008-f.md",
        &issue("0008", "f", "blocked_by: [1]\n"),
    );
    index_bundle(&docs);
    let before = read(&docs, "issues/0008-f.md");
    let output = number(&docs, "adr/0001-cache.md");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(read(&docs, "issues/0008-f.md"), before);
    assert!(read(&docs, "issues/0003-c.md").contains("blocked_by: [9, 7, 5]\n"));
}

fn renumbered_dependent(label: &str, written: &str) -> String {
    let docs = temp_bundle("set-number-blocked-by", label);
    write(&docs, "issues/0007-b.md", &issue("0007", "b", ""));
    write(&docs, "issues/0009-x.md", &issue("0009", "x", ""));
    let extra = format!("blocked_by: {written}\n");
    write(&docs, "issues/0003-c.md", &issue("0003", "c", &extra));
    index_bundle(&docs);
    let output = number(&docs, "issues/0007-b.md");
    assert!(output.status.success(), "{}", stderr(&output));
    read(&docs, "issues/0003-c.md")
}

#[test]
fn renumbering_keeps_each_untouched_blocked_by_item_with_its_type() {
    let cases = [
        ("[7, ~, 9]", "blocked_by: [10, ~, 9]\n"),
        ("[7, \"\"]", "blocked_by: [10, \"\"]\n"),
        ("[9, \"true\", 7]", "blocked_by: [9, \"true\", 10]\n"),
        ("[0007, \"0005\"]", "blocked_by: [10, 0005]\n"),
        ("[0007]", "blocked_by: [10]\n"),
        ("0007", "blocked_by: [10]\n"),
    ];
    for (index, (written, expected)) in cases.iter().enumerate() {
        let after = renumbered_dependent(&format!("typed-{index}"), written);
        assert!(after.contains(expected), "{written} -> {after}");
    }
}

#[test]
fn renumbering_changes_nothing_when_a_dependent_cannot_be_rewritten() {
    let docs = bundle("abort");
    write(
        &docs,
        "issues/0008-f.md",
        &issue("0008", "f", "blocked_by:\n  - 7\n  - 9\n"),
    );
    index_bundle(&docs);
    let names = [
        "issues/0007-b.md",
        "issues/0003-c.md",
        "issues/0004-d.md",
        "issues/0008-f.md",
        "issues/0009-x.md",
    ];
    let before: Vec<String> = names.iter().map(|name| read(&docs, name)).collect();
    let index_before = read(&docs, "issues/index.md");

    let output = number(&docs, "issues/0007-b.md");
    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(docs.join("issues/0007-b.md").exists(), "renamed anyway");
    let after: Vec<String> = names.iter().map(|name| read(&docs, name)).collect();
    assert_eq!(before, after);
    assert_eq!(read(&docs, "issues/index.md"), index_before);
    let check = stdout(&run_check(&docs));
    assert!(!check.contains("index.md"), "{check}");
}
