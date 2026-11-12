use super::*;
use crate::check::Reporter;
use crate::test_support::MapStore;

fn adr(status: &str, body: &str) -> String {
    format!("---\ntype: ADR\ntitle: t\nstatus: {status}\n---\n\n{body}")
}

fn issue(status: &str, body: &str) -> String {
    format!("---\ntype: Issue\ntitle: t\nstatus: {status}\n---\n\n{body}")
}

fn stale_count(files: &[(&str, &str)]) -> usize {
    let (store, all_md) = MapStore::seeded(files);
    let bundle = Path::new("kb");
    let mut reporter = Reporter::new();
    check_liveness(&store, bundle, &all_md, &mut reporter);
    reporter.advisories.len()
}

const ADR_PATH: &str = "kb/adr/0001-x.md";
const DECLARE: &str = "Implements [the decision](/adr/0001-x.md).";

fn declared(status: &str, body: &str) -> usize {
    stale_count(&[
        (ADR_PATH, &adr("Proposed", "no links here")),
        ("kb/issues/0002-y.md", &issue(status, body)),
    ])
}

#[test]
fn an_adr_linking_a_closed_issue_that_does_not_declare_it_is_not_stale() {
    let n = stale_count(&[
        (
            ADR_PATH,
            &adr("Proposed", "See [issue](/issues/0002-y.md)."),
        ),
        ("kb/issues/0002-y.md", &issue("closed", "body")),
    ]);
    assert_eq!(n, 0);
}

#[test]
fn a_closed_issue_declaring_the_adr_makes_it_stale_without_the_adr_linking_back() {
    assert_eq!(declared("closed", DECLARE), 1);
}

#[test]
fn a_done_issue_declaring_the_adr_makes_it_stale() {
    assert_eq!(declared("done", DECLARE), 1);
}

#[test]
fn an_open_or_in_progress_declaring_issue_leaves_the_adr_active() {
    assert_eq!(declared("open", DECLARE), 0);
    assert_eq!(declared("in-progress", DECLARE), 0);
}

#[test]
fn a_superseded_declaring_issue_makes_the_adr_stale() {
    assert_eq!(declared("Superseded", DECLARE), 1);
}

#[test]
fn a_lower_case_implements_declaration_counts() {
    assert_eq!(
        declared("closed", "implements [the decision](/adr/0001-x.md)"),
        1
    );
}

#[test]
fn implements_without_a_link_does_not_count() {
    assert_eq!(declared("closed", "Implements the decision"), 0);
}

#[test]
fn a_link_without_the_implements_word_does_not_count() {
    assert_eq!(
        declared("closed", "Relates to [the decision](/adr/0001-x.md)"),
        0
    );
}

#[test]
fn implements_linking_another_adr_does_not_count() {
    let n = stale_count(&[
        (ADR_PATH, &adr("Proposed", "x")),
        ("kb/adr/0003-z.md", &adr("Accepted", "x")),
        (
            "kb/issues/0002-y.md",
            &issue("closed", "Implements [the decision](/adr/0003-z.md)"),
        ),
    ]);
    assert_eq!(n, 0);
}

#[test]
fn a_relative_declaration_link_resolving_to_the_adr_counts() {
    assert_eq!(
        declared("closed", "Implements [the decision](../adr/0001-x.md)"),
        1
    );
}

#[test]
fn a_declaration_inside_a_fenced_code_block_does_not_count() {
    let body = format!("```md\n{DECLARE}\n```");
    assert_eq!(declared("closed", &body), 0);
}

#[test]
fn two_closed_issues_declaring_the_same_adr_yield_one_advisory() {
    let n = stale_count(&[
        (ADR_PATH, &adr("Proposed", "x")),
        ("kb/issues/0002-y.md", &issue("closed", DECLARE)),
        ("kb/issues/0003-z.md", &issue("done", DECLARE)),
    ]);
    assert_eq!(n, 1);
}

#[test]
fn an_accepted_adr_is_never_stale_proposed() {
    let n = stale_count(&[
        (ADR_PATH, &adr("Accepted", "x")),
        ("kb/issues/0002-y.md", &issue("closed", DECLARE)),
    ]);
    assert_eq!(n, 0);
}

#[test]
fn liveness_is_scoped_to_adr_records() {
    let n = stale_count(&[
        (
            "kb/adr/0001-x.md",
            "---\ntype: PRD\ntitle: t\nstatus: Draft\n---\n\nbody",
        ),
        ("kb/issues/0002-y.md", &issue("closed", DECLARE)),
    ]);
    assert_eq!(n, 0);
}

#[test]
fn is_seed_status_reads_the_registry_seed_for_adr() {
    assert!(is_seed_status("ADR", "Proposed"));
    assert!(!is_seed_status("ADR", "Accepted"));
}

#[test]
fn a_word_ending_in_implements_does_not_count_as_a_declaration() {
    assert_eq!(
        declared("closed", "Reimplements [the decision](/adr/0001-x.md)."),
        0
    );
}

fn issue_with_description(description: &str, body: &str) -> String {
    format!("---\ntype: Issue\ntitle: t\nstatus: closed\ndescription: {description}\n---\n\n{body}")
}

fn stale_with_issue(contents: &str) -> usize {
    stale_count(&[
        (ADR_PATH, &adr("Proposed", "no links here")),
        ("kb/issues/0002-y.md", contents),
    ])
}

#[test]
fn a_declaration_in_the_frontmatter_does_not_count() {
    let contents = issue_with_description("Implements [the decision](/adr/0001-x.md)", "body");
    assert_eq!(stale_with_issue(&contents), 0);
}

#[test]
fn a_declaration_in_the_body_counts_when_the_frontmatter_has_one_too() {
    let contents = issue_with_description("Implements [the decision](/adr/0001-x.md)", DECLARE);
    assert_eq!(stale_with_issue(&contents), 1);
}

#[test]
fn a_declaration_on_the_first_body_line_counts() {
    let contents = "---\ntype: Issue\ntitle: t\nstatus: closed\n---\nImplements [the decision](/adr/0001-x.md).";
    assert_eq!(stale_with_issue(contents), 1);
}
