//! Which files `record-type` judges, how `status-vocabulary` words a
//! case-only difference, and how `--changed-files` scopes the advisories.

use serde_json::Value;
use std::path::Path;

mod common;
use common::{living_docs, write};

const STAMP: &str = "2026-11-20T10:00:00Z";

fn adr(extra: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: X\ndescription: A record.\nowner: a@b.c\n{extra}---\n\n# 0001. X\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n"
    )
}

fn issue(extra: &str) -> String {
    format!(
        "---\ntype: Issue\ntitle: X\ndescription: A record.\nowner: a@b.c\n{extra}---\n\n# 0001. X\n\n## Scope\n\ns.\n\n## Acceptance\n\na.\n"
    )
}

fn stamped(extra: &str) -> String {
    format!("{extra}timestamp: {STAMP}\n")
}

fn typed(declared: &str) -> String {
    format!("---\ntype: {declared}\ntitle: T\ndescription: T.\ntimestamp: {STAMP}\n---\n\n# T\n")
}

fn bundle_of(label: &str, records: &[(&str, String)]) -> common::Scratch {
    let bundle = common::temp_bundle("frontmatter-advice-scope", label);
    let mut dirs: Vec<&str> = records
        .iter()
        .filter_map(|(rel, _)| rel.split('/').next())
        .collect();
    dirs.sort_unstable();
    dirs.dedup();
    let links: String = dirs
        .iter()
        .map(|dir| format!("- [{dir}]({dir}/index.md)\n"))
        .collect();
    write(&bundle, "index.md", &format!("# Index\n\n{links}"));
    for (rel, contents) in records {
        write(&bundle, rel, contents);
    }
    common::index_bundle(&bundle);
    bundle
}

fn messages(json: &Value, kind: &str) -> Vec<String> {
    json["advisories"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["kind"] == kind)
        .map(|a| a["message"].as_str().unwrap().to_owned())
        .collect()
}

fn check_scoped(bundle: &Path, changed: &str) -> Value {
    let output = living_docs()
        .args(["check", "--json", "--changed-files"])
        .arg(bundle.join(changed))
        .arg("--")
        .arg(bundle)
        .output()
        .expect("failed to run living-docs check");
    serde_json::from_slice(&output.stdout).expect("check --json prints JSON")
}

fn advised(label: &str, rel: &str, contents: String, kind: &str) -> Vec<String> {
    let bundle = bundle_of(label, &[(rel, contents)]);
    messages(&check_scoped(&bundle, rel), kind)
}

#[test]
fn record_type_judges_only_the_files_a_type_index_lists() {
    let cases = [
        ("research/references.md", "Reference", 0),
        ("adr/notes.md", "Concept", 0),
        ("research/0004-x.md", "Reference", 1),
        ("architecture/runners.md", "Architecture", 1),
    ];
    for (rel, declared, expected) in cases {
        let found = advised("rt-listed", rel, typed(declared), "record-type");
        assert_eq!(found.len(), expected, "{rel}: {found:?}");
    }
}

#[test]
fn status_vocabulary_names_the_canonical_spelling_of_a_case_only_difference() {
    let cases = [
        (
            "issues/0001-x.md",
            issue(&stamped("status: Open\n")),
            "open",
        ),
        (
            "adr/0001-x.md",
            adr(&stamped("status: accepted\n")),
            "Accepted",
        ),
    ];
    for (rel, contents, canonical) in cases {
        let found = advised("sv-case", rel, contents, "status-vocabulary");
        assert_eq!(found.len(), 1, "{rel}: {found:?}");
        assert!(
            found[0].contains(&format!("write `{canonical}`")),
            "{rel}: {found:?}"
        );
    }
}

#[test]
fn status_vocabulary_names_no_single_spelling_for_an_unrelated_value() {
    let found = advised(
        "sv-unrelated",
        "issues/0001-x.md",
        issue(&stamped("status: done\n")),
        "status-vocabulary",
    );
    assert_eq!(found.len(), 1, "got: {found:?}");
    assert!(!found[0].contains("write `"), "got: {found:?}");
}

#[test]
fn gate_changed_files_reports_the_advisories_of_a_named_record_and_hides_the_rest() {
    let wrong_type = adr(&stamped("status: Accepted\n")).replace("type: ADR", "type: BDR");
    let bundle = bundle_of(
        "scope",
        &[
            ("adr/0001-x.md", wrong_type),
            ("adr/0002-x.md", adr(&stamped("status: Accepted\n"))),
        ],
    );
    let named = check_scoped(&bundle, "adr/0001-x.md");
    assert_eq!(messages(&named, "record-type").len(), 1, "{named}");
    let other = check_scoped(&bundle, "adr/0002-x.md");
    assert!(messages(&other, "record-type").is_empty(), "{other}");
}
