//! The four frontmatter advisories (`status-vocabulary`, `record-type`,
//! `timestamp`, `stale-successor`): each fires on a state the CLI never
//! writes, and none of them fails the gate.

use serde_json::Value;
use std::path::Path;

mod common;
use common::{living_docs, write};

const STAMP: &str = "2026-11-20T10:00:00Z";
const KINDS: [&str; 4] = [
    "status-vocabulary",
    "record-type",
    "timestamp",
    "stale-successor",
];

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

fn bundle_of(label: &str, records: &[(&str, String)]) -> common::Scratch {
    let bundle = common::temp_bundle("frontmatter-advice", label);
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

fn check_json(bundle: &Path) -> (Option<i32>, Value) {
    let output = living_docs()
        .args(["check", bundle.to_str().unwrap(), "--json"])
        .output()
        .expect("failed to run living-docs check");
    let json = serde_json::from_slice(&output.stdout).expect("check --json prints JSON");
    (output.status.code(), json)
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

fn advised(label: &str, rel: &str, contents: String, kind: &str) -> Vec<String> {
    let bundle = bundle_of(label, &[(rel, contents)]);
    messages(&check_json(&bundle).1, kind)
}

#[test]
fn status_vocabulary_names_the_value_and_the_vocabulary_of_an_issue() {
    let found = advised(
        "sv-issue",
        "issues/0001-x.md",
        issue(&stamped("status: done\n")),
        "status-vocabulary",
    );
    assert_eq!(found.len(), 1, "got: {found:?}");
    assert!(found[0].contains("done") && found[0].contains("open, in-progress, closed"));
}

#[test]
fn status_vocabulary_flags_an_adr_status_outside_the_vocabulary() {
    for value in ["Reverted", "active"] {
        let found = advised(
            "sv-adr",
            "adr/0001-x.md",
            adr(&stamped(&format!("status: {value}\n"))),
            "status-vocabulary",
        );
        assert_eq!(found.len(), 1, "{value}: {found:?}");
        assert!(found[0].contains(value), "{value}: {found:?}");
    }
}

#[test]
fn status_vocabulary_is_silent_for_a_member_superseded_in_any_case_or_no_status() {
    for status in [
        "status: Accepted\n",
        "status: Superseded\n",
        "status: superseded\n",
        "",
    ] {
        let found = advised(
            "sv-silent",
            "adr/0001-x.md",
            adr(&stamped(status)),
            "status-vocabulary",
        );
        assert!(found.is_empty(), "{status:?}: {found:?}");
    }
}

#[test]
fn status_vocabulary_is_silent_for_a_type_with_no_vocabulary() {
    let constitution = format!(
        "---\ntype: Constitution\ntitle: C\ndescription: C.\ntimestamp: {STAMP}\nstatus: whatever\n---\n\n# C\n\n## Product\n\np.\n\n## Scope Boundaries\n\ns.\n\n## Non-negotiables\n\nn.\n"
    );
    let bundle = bundle_of("sv-constitution", &[("adr/0001-x.md", adr(&stamped("")))]);
    write(&bundle, "constitution.md", &constitution);
    let found = messages(&check_json(&bundle).1, "status-vocabulary");
    assert!(found.is_empty(), "got: {found:?}");
}

#[test]
fn record_type_names_the_declared_and_the_directory_type() {
    let wrong = adr(&stamped("status: Accepted\n")).replace("type: ADR", "type: BDR");
    let found = advised("rt-bdr", "adr/0001-x.md", wrong, "record-type");
    assert_eq!(found.len(), 1, "got: {found:?}");
    assert!(found[0].contains("BDR") && found[0].contains("ADR"));
}

#[test]
fn record_type_flags_a_lowercase_type() {
    let research = format!(
        "---\ntype: research\ntitle: R\ndescription: R.\ntimestamp: {STAMP}\n---\n\n# 0001. R\n"
    );
    let found = advised("rt-lower", "research/0001-x.md", research, "record-type");
    assert_eq!(found.len(), 1, "got: {found:?}");
}

#[test]
fn record_type_is_silent_for_a_matching_type_an_index_and_an_untyped_directory() {
    let concept =
        format!("---\ntype: Concept\ntitle: N\ndescription: N.\ntimestamp: {STAMP}\n---\n\n# N\n");
    let bundle = bundle_of(
        "rt-silent",
        &[
            ("adr/0001-x.md", adr(&stamped("status: Accepted\n"))),
            ("notes/0001-x.md", concept),
        ],
    );
    let found = messages(&check_json(&bundle).1, "record-type");
    assert!(found.is_empty(), "got: {found:?}");
}

#[test]
fn timestamp_names_the_fix_for_a_missing_or_empty_timestamp() {
    for extra in ["status: Accepted\n", "status: Accepted\ntimestamp:\n"] {
        let found = advised("ts-missing", "adr/0001-x.md", adr(extra), "timestamp");
        assert_eq!(found.len(), 1, "{extra:?}: {found:?}");
        assert!(
            found[0].contains("living-docs manage fmt"),
            "got: {found:?}"
        );
    }
}

#[test]
fn timestamp_is_silent_when_present_and_gone_after_manage_fmt() {
    let present = advised(
        "ts-present",
        "adr/0001-x.md",
        adr(&stamped("status: Accepted\n")),
        "timestamp",
    );
    assert!(present.is_empty(), "got: {present:?}");

    let bundle = bundle_of("ts-fmt", &[("adr/0001-x.md", adr("status: Accepted\n"))]);
    assert_eq!(messages(&check_json(&bundle).1, "timestamp").len(), 1);
    let fmt = living_docs()
        .args(["manage", "fmt", bundle.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs manage fmt");
    assert!(fmt.status.success(), "fmt failed");
    let after = messages(&check_json(&bundle).1, "timestamp");
    assert!(after.is_empty(), "got: {after:?}");
}

#[test]
fn stale_successor_names_the_successor_of_a_record_still_in_force() {
    for status in ["Accepted", "Deprecated"] {
        let contents = adr(&stamped(&format!(
            "status: {status}\nsuperseded_by: 0002\n"
        )));
        let found = advised("ss-live", "adr/0001-x.md", contents, "stale-successor");
        assert_eq!(found.len(), 1, "{status}: {found:?}");
        assert!(found[0].contains("0002"), "{status}: {found:?}");
    }
}

#[test]
fn stale_successor_is_silent_for_a_superseded_record_or_an_empty_link() {
    for extra in [
        "status: Superseded\nsuperseded_by: 0002\n",
        "status: Accepted\nsuperseded_by:\n",
    ] {
        let found = advised(
            "ss-silent",
            "adr/0001-x.md",
            adr(&stamped(extra)),
            "stale-successor",
        );
        assert!(found.is_empty(), "{extra:?}: {found:?}");
    }
}

fn all_four_states() -> common::Scratch {
    let wrong_type = adr(&stamped("status: Accepted\n")).replace("type: ADR", "type: BDR");
    bundle_of(
        "gate",
        &[
            ("issues/0001-x.md", issue(&stamped("status: done\n"))),
            ("adr/0001-x.md", adr("status: Accepted\n")),
            (
                "adr/0002-x.md",
                adr(&stamped("status: Accepted\nsuperseded_by: 0003\n")),
            ),
            ("adr/0003-x.md", wrong_type),
        ],
    )
}

#[test]
fn gate_stays_green_with_all_four_advisories_carrying_their_kind() {
    let bundle = all_four_states();
    let (code, json) = check_json(&bundle);
    assert_eq!(code, Some(0), "advisories never fail check: {json}");
    assert_eq!(json["ok"], true, "{json}");
    for kind in KINDS {
        assert!(!messages(&json, kind).is_empty(), "no {kind}: {json}");
    }
}

#[test]
fn gate_changed_files_naming_another_record_reports_none_of_the_four() {
    let bundle = all_four_states();
    write(
        &bundle,
        "adr/0004-x.md",
        &adr(&stamped("status: Accepted\n")),
    );
    common::index_bundle(&bundle);
    let output = living_docs()
        .args(["check", "--json", "--changed-files"])
        .arg(bundle.join("adr/0004-x.md"))
        .arg("--")
        .arg(&*bundle)
        .output()
        .expect("failed to run living-docs check");
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    for kind in KINDS {
        assert!(messages(&json, kind).is_empty(), "{kind} leaked: {json}");
    }
}

#[test]
fn gate_help_lists_the_four_kinds_with_their_meanings() {
    let output = living_docs()
        .args(["check", "--help", "--plain"])
        .output()
        .expect("failed to run living-docs check --help");
    let help = String::from_utf8_lossy(&output.stdout);
    for name in KINDS {
        let line = help
            .lines()
            .find(|l| l.trim_start().starts_with(name))
            .unwrap_or_else(|| panic!("no help line for {name}"));
        assert!(line.trim_start()[name.len()..].trim().len() > 10, "{line}");
    }
}
