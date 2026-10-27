//! `check` fails a supersede link the other record does not answer.

mod common;

use common::{index_bundle, living_docs, run_check, stdout_of, temp_bundle, write};

const BODY: &str = "## Context\n\nWhy.\n\n## Decision\n\nWhat.\n\n## Consequences\n\nSo.\n\n## Verification\n\n- Checked.\n";

/// An ADR `adr/<number>-d.md` carrying the given extra frontmatter lines.
fn adr(number: &str, extra: &str) -> (String, String) {
    let contents = format!(
        "---\ntype: ADR\ntitle: Decision {number}\ndescription: A record\nowner: Someone\nstatus: Accepted\n{extra}timestamp: 2026-01-01T00:00:00Z\n---\n\n# {number}. Decision {number}\n\n{BODY}"
    );
    (format!("adr/{number}-d.md"), contents)
}

fn bundle_of(label: &str, records: &[(String, String)]) -> common::Scratch {
    let bundle = temp_bundle("check-supersede-links", label);
    write(&bundle, "index.md", "# Docs\n\n* [adr](adr/)\n");
    for (rel, contents) in records {
        write(&bundle, rel, contents);
    }
    index_bundle(&bundle);
    bundle
}

fn link_lines(output: &std::process::Output) -> Vec<String> {
    stdout_of(output)
        .lines()
        .filter(|line| line.contains("does not name"))
        .map(str::to_owned)
        .collect()
}

fn expect_ok(label: &str, records: &[(String, String)]) {
    let output = run_check(&bundle_of(label, records));
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}

#[test]
fn a_supersedes_the_target_does_not_answer_fails_on_the_carrier() {
    let bundle = bundle_of(
        "supersedes",
        &[adr("0001", ""), adr("0003", "supersedes: 0001\n")],
    );
    let output = run_check(&bundle);
    let lines = link_lines(&output);
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    assert_eq!(lines.len(), 1, "{}", stdout_of(&output));
    assert!(lines[0].contains("0003-d.md"), "{}", lines[0]);
    assert!(lines[0].contains("0001-d.md"), "{}", lines[0]);
    assert!(lines[0].contains("supersedes"), "{}", lines[0]);
}

#[test]
fn a_superseded_by_the_target_does_not_answer_fails_on_the_carrier() {
    let bundle = bundle_of(
        "superseded-by",
        &[adr("0001", "superseded_by: 0003\n"), adr("0003", "")],
    );
    let output = run_check(&bundle);
    let lines = link_lines(&output);
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    assert_eq!(lines.len(), 1, "{}", stdout_of(&output));
    assert!(lines[0].contains("0001-d.md"), "{}", lines[0]);
    assert!(lines[0].contains("0003-d.md"), "{}", lines[0]);
    assert!(lines[0].contains("superseded_by"), "{}", lines[0]);
}

#[test]
fn a_pair_answered_by_a_third_record_gives_a_violation_per_carrier() {
    let bundle = bundle_of(
        "third",
        &[
            adr("0001", "superseded_by: 0002\n"),
            adr("0002", ""),
            adr("0003", "supersedes: 0001\n"),
        ],
    );
    let output = run_check(&bundle);
    let lines = link_lines(&output);
    assert_eq!(lines.len(), 2, "{}", stdout_of(&output));
    assert!(lines.iter().any(|l| l.contains("0003-d.md")));
    assert!(lines.iter().any(|l| l.contains("0001-d.md")));
}

#[test]
fn only_the_unanswered_entry_of_a_list_is_reported() {
    let bundle = bundle_of(
        "list",
        &[
            adr("0001", "superseded_by: 0003\n"),
            adr("0002", ""),
            adr("0003", "supersedes: [0001, 0002]\n"),
        ],
    );
    let output = run_check(&bundle);
    let lines = link_lines(&output);
    assert_eq!(lines.len(), 1, "{}", stdout_of(&output));
    assert!(lines[0].contains("0003-d.md"), "{}", lines[0]);
    assert!(lines[0].contains("0002-d.md"), "{}", lines[0]);
    assert!(!lines[0].contains("0001-d.md"), "{}", lines[0]);
}

#[test]
fn answered_links_pass() {
    expect_ok(
        "pair",
        &[
            adr("0001", "superseded_by: 0003\n"),
            adr("0003", "supersedes: 0001\n"),
        ],
    );
}

#[test]
fn a_list_answered_by_every_entry_passes() {
    expect_ok(
        "list-ok",
        &[
            adr("0001", "superseded_by: 0003\n"),
            adr("0002", "superseded_by: 0003\n"),
            adr("0003", "supersedes: [0001, 0002]\n"),
        ],
    );
}

#[test]
fn zero_padded_bare_and_quoted_spellings_are_one_number() {
    let bundle = bundle_of(
        "spellings",
        &[
            adr("0001", "superseded_by: \"0003\"\n"),
            adr("0002", "superseded_by: 3\n"),
            adr("0003", "supersedes: [1, \"0002\"]\n"),
        ],
    );
    assert!(link_lines(&run_check(&bundle)).is_empty());
}

#[test]
fn a_target_with_no_record_is_not_judged() {
    let bundle = bundle_of("missing", &[adr("0003", "supersedes: 0009\n")]);
    assert!(link_lines(&run_check(&bundle)).is_empty());
}

#[test]
fn a_target_of_another_type_is_not_judged() {
    let issue = "---\ntype: Issue\ntitle: Work\ndescription: A record\nowner: Someone\nstatus: Open\nsupersedes: 0001\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n# 0002. Work\n\n### Scope\n\nOne.\n\n### Acceptance\n\n- Works.\n";
    let bundle = bundle_of(
        "types",
        &[adr("0001", ""), ("issues/0002-w.md".into(), issue.into())],
    );
    assert!(link_lines(&run_check(&bundle)).is_empty());
}

#[test]
fn a_record_path_reports_its_own_violation_and_json_carries_it() {
    let bundle = bundle_of(
        "scoped",
        &[adr("0001", ""), adr("0003", "supersedes: 0001\n")],
    );
    let carrier = bundle.join("adr/0003-d.md");
    let scoped = living_docs()
        .args(["check", "--plain", carrier.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(scoped.status.code(), Some(1), "{}", stdout_of(&scoped));
    assert_eq!(link_lines(&scoped).len(), 1, "{}", stdout_of(&scoped));
    let json = living_docs()
        .args(["check", bundle.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    let violations = report["violations"].as_array().unwrap();
    assert_eq!(violations.len(), 1, "{report}");
    assert!(violations[0]["message"]
        .as_str()
        .unwrap()
        .contains("0001-d.md"));
}
