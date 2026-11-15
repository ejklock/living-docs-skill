mod common;

use common::{index_bundle, living_docs, run_check, stdout_of, temp_bundle, write};
use std::path::Path;
use std::process::Output;

struct Issue {
    number: &'static str,
    status: &'static str,
    blocked_by: Option<&'static str>,
}

fn issue(number: &'static str, blocked_by: &'static str) -> Issue {
    Issue {
        number,
        status: "Open",
        blocked_by: Some(blocked_by),
    }
}

fn contents(doc_type: &str, issue: &Issue) -> String {
    let number = issue.number;
    let blocked = issue
        .blocked_by
        .map(|value| format!("blocked_by: {value}\n"))
        .unwrap_or_default();
    let body = if doc_type == "ADR" {
        "## Context\n\nWhy.\n\n## Decision\n\nWhat.\n\n## Consequences\n\nSo.\n\n## Verification\n\n- Checked.\n"
    } else {
        "### Scope\n\nOne thing.\n\n### Acceptance\n\n- It works.\n"
    };
    format!(
        "---\ntype: {doc_type}\ntitle: Work {number}\ndescription: A record\nowner: Someone\nstatus: {}\n{blocked}timestamp: 2026-01-01T00:00:00Z\n---\n\n# {number}. Work {number}\n\n{body}",
        issue.status
    )
}

fn bundle_of(label: &str, issues: &[Issue]) -> common::Scratch {
    let bundle = temp_bundle("check-dependencies", label);
    for issue in issues {
        write(
            &bundle,
            &format!("issues/{}-work.md", issue.number),
            &contents("Issue", issue),
        );
    }
    write(&bundle, "index.md", "# Docs\n\n* [issues](issues/)\n");
    index_bundle(&bundle);
    bundle
}

fn file(number: &str) -> String {
    format!("issues/{number}-work.md")
}

fn findings(output: &Output) -> Vec<String> {
    stdout_of(output)
        .lines()
        .filter(|line| line.contains("blocked_by"))
        .map(str::to_owned)
        .collect()
}

fn scoped(bundle: &Path, changed: &[&str]) -> Output {
    let mut command = living_docs();
    command.args(["check", "--plain", "--changed-files"]);
    for number in changed {
        command.arg(bundle.join(file(number)));
    }
    command.arg("--").arg(bundle).output().unwrap()
}

#[test]
fn an_item_naming_no_issue_fails_naming_the_file_and_the_item() {
    let bundle = bundle_of("dangling", &[issue("0001", "[7]"), issue("0002", "[]")]);
    let output = run_check(&bundle);
    let lines = findings(&output);
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    assert_eq!(lines.len(), 1, "{}", stdout_of(&output));
    assert!(lines[0].contains("0001-work.md"), "{}", lines[0]);
    assert!(lines[0].contains('7'), "{}", lines[0]);
}

#[test]
fn an_item_that_is_not_a_number_fails_naming_the_item() {
    let bundle = bundle_of("tracker", &[issue("0001", "[see-nine]")]);
    let output = run_check(&bundle);
    let lines = findings(&output);
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    assert_eq!(lines.len(), 1, "{}", stdout_of(&output));
    assert!(lines[0].contains("see-nine"), "{}", lines[0]);
}

#[test]
fn an_issue_listing_itself_fails() {
    let bundle = bundle_of("self", &[issue("0003", "[3]")]);
    let output = run_check(&bundle);
    let lines = findings(&output);
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    assert_eq!(lines.len(), 1, "{}", stdout_of(&output));
    assert!(lines[0].contains("0003-work.md"), "{}", lines[0]);
}

#[test]
fn a_closed_blocker_and_either_spelling_of_a_number_resolve() {
    let closed = Issue {
        number: "0012",
        status: "Closed",
        blocked_by: None,
    };
    let bundle = bundle_of(
        "resolve",
        &[
            closed,
            issue("0013", "[12]"),
            issue("0014", "[0012]"),
            issue("0015", "0012"),
            issue("0016", "[]"),
        ],
    );
    let output = run_check(&bundle);
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}

#[test]
fn a_record_that_is_not_an_issue_is_never_judged() {
    let bundle = bundle_of("adr", &[issue("0001", "[]")]);
    let adr = Issue {
        number: "0001",
        status: "Accepted",
        blocked_by: Some("[see-nine]"),
    };
    write(&bundle, "adr/0001-work.md", &contents("ADR", &adr));
    write(
        &bundle,
        "index.md",
        "# Docs\n\n* [issues](issues/)\n* [adr](adr/)\n",
    );
    index_bundle(&bundle);
    let output = run_check(&bundle);
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}

#[test]
fn a_cycle_gives_one_finding_naming_every_member_in_ascending_order() {
    let three = bundle_of(
        "cycle3",
        &[
            issue("0003", "[1]"),
            issue("0001", "[2]"),
            issue("0002", "[3]"),
        ],
    );
    let output = run_check(&three);
    let lines = findings(&output);
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    assert_eq!(lines.len(), 1, "{}", stdout_of(&output));
    let at = |n: &str| lines[0].find(&format!("{n}-work.md")).unwrap();
    assert!(
        at("0001") < at("0002") && at("0002") < at("0003"),
        "{}",
        lines[0]
    );

    let two = bundle_of("cycle2", &[issue("0001", "[2]"), issue("0002", "[1]")]);
    let lines = findings(&run_check(&two));
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("0001-work.md") && lines[0].contains("0002-work.md"));
}

#[test]
fn a_chain_without_a_cycle_passes() {
    let bundle = bundle_of(
        "chain",
        &[
            issue("0001", "[2, 3]"),
            issue("0002", "[3]"),
            issue("0003", "[]"),
        ],
    );
    let output = run_check(&bundle);
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}

#[test]
fn a_scoped_check_judges_only_the_named_issue_but_resolves_against_all() {
    let bundle = bundle_of(
        "scoped-dangling",
        &[
            issue("0001", "[2]"),
            issue("0002", "[]"),
            issue("0003", "[9]"),
        ],
    );
    let output = scoped(&bundle, &["0001"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
    let output = scoped(&bundle, &["0002"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
    let output = scoped(&bundle, &["0003"]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    assert!(stdout_of(&output).contains("0003-work.md"));
}

#[test]
fn a_scoped_check_reports_a_cycle_when_any_member_is_named() {
    let bundle = bundle_of(
        "scoped-cycle",
        &[
            issue("0001", "[2]"),
            issue("0002", "[1]"),
            issue("0003", "[]"),
        ],
    );
    for member in ["0001", "0002"] {
        let output = scoped(&bundle, &[member]);
        assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
        assert_eq!(findings(&output).len(), 1);
    }
    assert_eq!(scoped(&bundle, &["0003"]).status.code(), Some(0));
}
