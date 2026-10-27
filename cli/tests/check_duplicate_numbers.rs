mod common;

use common::{fixture, index_bundle, living_docs, run_check, stdout_of, temp_bundle, write};
use std::path::Path;
use std::process::Output;

const REPAIR: &str = "living-docs set <path> number next";

fn record(kind: &str, number: &str, slug: &str, status: &str) -> String {
    let (doc_type, title, body) = if kind == "adr" {
        (
            "ADR",
            "Decision",
            "## Context\n\nWhy.\n\n## Decision\n\nWhat.\n\n## Consequences\n\nSo.\n\n## Verification\n\n- Checked.\n",
        )
    } else {
        (
            "Issue",
            "Work",
            "### Scope\n\nOne thing.\n\n### Acceptance\n\n- It works.\n",
        )
    };
    format!(
        "---\ntype: {doc_type}\ntitle: {title} {slug}\ndescription: A record\nowner: Someone\nstatus: {status}\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n# {number}. {title} {slug}\n\n{body}"
    )
}

type Rec = (&'static str, &'static str, &'static str, &'static str);

fn issue(number: &'static str, slug: &'static str) -> Rec {
    ("issue", number, slug, "Open")
}

fn add(bundle: &Path, kind: &str, number: &str, slug: &str, status: &str) {
    let dir = if kind == "adr" { "adr" } else { "issues" };
    write(
        bundle,
        &format!("{dir}/{number}-{slug}.md"),
        &record(kind, number, slug, status),
    );
}

fn indexed(label: &str, records: &[Rec]) -> common::Scratch {
    let bundle = temp_bundle("check-duplicate-numbers", label);
    let mut root = String::from("# Docs\n\n");
    for (kind, number, slug, status) in records {
        add(&bundle, kind, number, slug, status);
    }
    for dir in ["issues", "adr"] {
        if bundle.join(dir).is_dir() {
            root.push_str(&format!("* [{dir}]({dir}/)\n"));
        }
    }
    write(&bundle, "index.md", &root);
    index_bundle(&bundle);
    bundle
}

fn duplicate_lines(output: &Output) -> Vec<String> {
    stdout_of(output)
        .lines()
        .filter(|line| line.contains(REPAIR))
        .map(str::to_owned)
        .collect()
}

#[test]
fn two_records_sharing_a_number_fail_with_both_paths_and_the_repair() {
    let bundle = indexed("pair", &[issue("0007", "a"), issue("0007", "b")]);
    let output = run_check(&bundle);
    let lines = duplicate_lines(&output);
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    assert_eq!(lines.len(), 1, "{}", stdout_of(&output));
    assert!(lines[0].contains("issues/0007-a.md"), "{}", lines[0]);
    assert!(lines[0].contains("issues/0007-b.md"), "{}", lines[0]);
    assert!(lines[0].contains("on each of the others"), "{}", lines[0]);
    let root = bundle.to_str().unwrap();
    assert!(
        !lines[0].contains(&format!("{root}/issues/0007")),
        "{}",
        lines[0]
    );
    assert!(stdout_of(&output).contains("1 violation(s)"));
}

#[test]
fn distinct_numbers_and_one_number_across_types_pass() {
    let adr = ("adr", "0007", "b", "Accepted");
    for (label, records) in [
        ("distinct", [issue("0007", "a"), issue("0008", "b")]),
        ("types", [issue("0007", "a"), adr]),
    ] {
        let output = run_check(&indexed(label, &records));
        assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
    }
}

#[test]
fn a_closed_record_and_an_open_one_still_collide() {
    let bundle = indexed(
        "status",
        &[("issue", "0007", "a", "Closed"), issue("0007", "b")],
    );
    let output = run_check(&bundle);
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    assert_eq!(duplicate_lines(&output).len(), 1);
}

#[test]
fn three_records_sharing_a_number_give_one_violation_naming_all() {
    let bundle = indexed(
        "triple",
        &[issue("0007", "a"), issue("0007", "b"), issue("0007", "c")],
    );
    let output = run_check(&bundle);
    let lines = duplicate_lines(&output);
    assert_eq!(lines.len(), 1, "{}", stdout_of(&output));
    for slug in ["a", "b", "c"] {
        assert!(
            lines[0].contains(&format!("issues/0007-{slug}.md")),
            "{}",
            lines[0]
        );
    }
}

#[test]
fn two_shared_numbers_give_two_violations() {
    let bundle = indexed(
        "two-numbers",
        &[
            issue("0007", "a"),
            issue("0007", "b"),
            issue("0009", "c"),
            issue("0009", "d"),
        ],
    );
    let output = run_check(&bundle);
    assert_eq!(duplicate_lines(&output).len(), 2, "{}", stdout_of(&output));
    assert!(stdout_of(&output).contains("2 violation(s)"));
}

#[test]
fn files_without_a_number_prefix_and_index_files_are_not_duplicates() {
    let bundle = indexed("unnumbered", &[issue("0007", "a")]);
    write(
        &bundle,
        "issues/notes.md",
        &record("issue", "0007", "notes", "Open"),
    );
    let output = run_check(&bundle);
    assert!(
        duplicate_lines(&output).is_empty(),
        "{}",
        stdout_of(&output)
    );
}

#[test]
fn a_concept_type_directory_with_number_prefixed_names_is_not_a_duplicate() {
    let bundle = indexed("concept", &[]);
    for slug in ["a", "b"] {
        write(
            &bundle,
            &format!("architecture/0001-{slug}.md"),
            &format!("---\ntype: Architecture\ntitle: {slug}\ndescription: A view\nowner: Someone\nstatus: Active\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n# {slug}\n"),
        );
    }
    let output = run_check(&bundle);
    assert!(
        duplicate_lines(&output).is_empty(),
        "{}",
        stdout_of(&output)
    );
}

#[test]
fn the_same_bundle_gives_byte_identical_output() {
    let bundle = indexed(
        "stable",
        &[
            issue("0009", "c"),
            issue("0007", "a"),
            issue("0007", "b"),
            issue("0009", "d"),
        ],
    );
    assert_eq!(run_check(&bundle).stdout, run_check(&bundle).stdout);
}

#[test]
fn the_violation_appears_in_json_under_the_shared_violation_shape() {
    let bundle = indexed("json", &[issue("0007", "a"), issue("0007", "b")]);
    let output = living_docs()
        .args(["check", bundle.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let violations = report["violations"].as_array().unwrap();
    assert_eq!(violations.len(), 1, "{report}");
    assert!(violations[0]["file"].is_string());
    let message = violations[0]["message"].as_str().unwrap();
    assert!(message.contains("0007-a.md") && message.contains("0007-b.md"));
    assert!(message.contains(REPAIR));
    assert_eq!(report["ok"], false);
    let mut keys: Vec<&String> = violations[0].as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(keys, ["file", "message"]);
}

#[test]
fn a_bundle_without_duplicates_prints_the_output_it_printed_before_the_check_existed() {
    let bundle = fixture("01-fence-link-clean");
    let shown = bundle.display();
    let plain = run_check(&bundle);
    assert_eq!(
        stdout_of(&plain),
        format!("Living Docs lint — bundle: {shown}\n\n\nOK — 2 docs, no invariant violations.\n")
    );
    let json = living_docs()
        .args(["check", bundle.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert_eq!(
        stdout_of(&json),
        format!("{{\"bundle\":\"{shown}\",\"docs\":2,\"violations\":[],\"advisories\":[],\"ok\":true}}\n")
    );
}

fn run_scoped(bundle: &Path, changed: &[&str]) -> Output {
    let files: Vec<String> = changed
        .iter()
        .map(|rel| bundle.join(rel).display().to_string())
        .collect();
    living_docs()
        .args(["check", "--require-owner", "--plain", "--changed-files"])
        .args(&files)
        .args(["--", bundle.to_str().unwrap()])
        .output()
        .unwrap()
}

fn pair_with_unrelated(label: &str) -> common::Scratch {
    indexed(
        label,
        &[issue("0007", "a"), issue("0007", "b"), issue("0009", "x")],
    )
}

#[test]
fn a_commit_touching_either_member_of_a_pair_fails_on_the_duplicate() {
    let bundle = pair_with_unrelated("scoped-pair");
    for changed in [
        vec!["issues/0007-a.md"],
        vec!["issues/0007-b.md"],
        vec!["issues/0007-a.md", "issues/0007-b.md"],
    ] {
        let output = run_scoped(&bundle, &changed);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{changed:?}: {}",
            stdout_of(&output)
        );
        assert_eq!(duplicate_lines(&output).len(), 1, "{changed:?}");
    }
}

#[test]
fn a_commit_touching_only_an_unrelated_record_passes() {
    let bundle = pair_with_unrelated("scoped-unrelated");
    let output = run_scoped(&bundle, &["issues/0009-x.md"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
    assert!(duplicate_lines(&output).is_empty());
}

#[test]
fn a_commit_touching_the_third_member_of_a_group_reports_it() {
    let bundle = indexed(
        "scoped-triple",
        &[issue("0007", "a"), issue("0007", "b"), issue("0007", "c")],
    );
    let output = run_scoped(&bundle, &["issues/0007-c.md"]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    assert_eq!(duplicate_lines(&output).len(), 1);
}

#[test]
fn checking_one_record_path_reports_the_duplicate_it_belongs_to() {
    let bundle = pair_with_unrelated("record-path");
    let target = bundle.join("issues/0007-b.md");
    let output = living_docs()
        .args(["check", target.to_str().unwrap(), "--plain"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    assert_eq!(duplicate_lines(&output).len(), 1);
}
