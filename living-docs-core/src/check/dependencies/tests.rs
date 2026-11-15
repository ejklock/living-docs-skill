use super::*;
use crate::test_support::MapStore;

fn issue_at(number: &str, blocked_by: &str) -> (String, String) {
    (
        format!("/bundle/issues/{number}-work.md"),
        format!(
            "---\ntype: Issue\ntitle: Work\nstatus: Open\nblocked_by: {blocked_by}\n---\n# Work\n"
        ),
    )
}

fn run(records: &[(String, String)]) -> Vec<(String, String)> {
    run_both(records).0
}

type Findings = Vec<(String, String)>;

fn run_both(records: &[(String, String)]) -> (Findings, Findings) {
    let seed: Vec<(&str, &str)> = records
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    let (store, all_md) = MapStore::seeded(&seed);
    let mut reporter = Reporter::new();
    check_dependencies(&store, Path::new("/bundle"), &all_md, &mut reporter);
    let violations = reporter
        .violations
        .into_iter()
        .map(|v| (v.file, v.message))
        .collect();
    let advice = reporter
        .advisories
        .into_iter()
        .map(|a| (a.file, a.message))
        .collect();
    (violations, advice)
}

#[test]
fn a_quoted_zero_padded_item_names_the_same_issue_as_its_bare_number() {
    let found = run(&[
        issue_at("0012", "[]"),
        issue_at("0013", "[\"0012\"]"),
        issue_at("0014", "[12]"),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_text_item_is_advised_with_the_item_and_the_issue_file_and_not_reported() {
    let (violations, advice) = run_both(&[issue_at("0001", "[\"see-nine\"]")]);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(advice.len(), 1, "{advice:?}");
    assert_eq!(advice[0].0, "/bundle/issues/0001-work.md");
    assert!(advice[0].1.contains("'see-nine'"), "{}", advice[0].1);
}

#[test]
fn a_scalar_comma_list_reads_as_several_items() {
    let found = run(&[issue_at("0001", "[]"), issue_at("0002", "0001, 0003")]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].1.contains("0003") && !found[0].1.contains("0001"),
        "{}",
        found[0].1
    );
}

#[test]
fn an_item_beyond_the_integer_range_names_no_issue() {
    let (violations, advice) = run_both(&[issue_at("0001", "[99999999999]")]);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert!(
        violations[0].1.contains("names no issue"),
        "{}",
        violations[0].1
    );
    assert!(advice.is_empty(), "{advice:?}");
}

#[test]
fn a_cycle_is_reported_once_with_its_members_and_not_the_issue_leading_into_it() {
    let found = run(&[
        issue_at("0004", "[2]"),
        issue_at("0001", "[2]"),
        issue_at("0002", "[3]"),
        issue_at("0003", "[1]"),
    ]);
    assert_eq!(found.len(), 1, "{found:?}");
    let message = &found[0].1;
    assert!(message.ends_with("issues/0001-work.md, issues/0002-work.md, issues/0003-work.md"));
    assert!(!message.contains("0004"), "{message}");
}

#[test]
fn two_separate_cycles_give_two_findings() {
    let found = run(&[
        issue_at("0001", "[2]"),
        issue_at("0002", "[1]"),
        issue_at("0003", "[4]"),
        issue_at("0004", "[3]"),
    ]);
    assert_eq!(found.len(), 2, "{found:?}");
}

#[test]
fn a_self_reference_is_one_finding_and_not_a_cycle() {
    let found = run(&[issue_at("0005", "[5]")]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].1.contains("itself"), "{}", found[0].1);
}
