//! `check` reports a record whose `description` is still a doc-type
//! template's placeholder sentence, the same class as an unfilled body slot.

use std::process::Output;

mod common;
use common::{living_docs, run_check, stdout_of, write};

const ISSUE_PLACEHOLDER: &str = "<One sentence — the change and its motivation.>";
const ADR_PLACEHOLDER: &str = "<One sentence — the decision and its scope.>";
const BODY: &str = "### Scope\n\nSome scope.\n\n### Acceptance\n\n- a criterion\n";

fn issue_with(description_line: Option<&str>) -> String {
    let description = description_line
        .map(|line| format!("description: {line}\n"))
        .unwrap_or_default();
    format!("---\ntype: Issue\ntitle: Work\n{description}status: open\ntimestamp: 2026-11-16T02:45:18Z\n---\n\n## 0001. Work\n\n{BODY}")
}

fn adr_with(description: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: Pick\ndescription: {description}\nowner: Dev\nstatus: Accepted\ntimestamp: 2026-11-16T02:45:18Z\n---\n\n# 0001. Pick\n\n## Context\n\nc\n\n## Decision\n\nd\n\n## Consequences\n\ne\n"
    )
}

fn scaffold(label: &str, record_path: &str, record: &str) -> common::Scratch {
    let bundle = common::temp_bundle("placeholder-description", label);
    let (issues, adrs) = if record_path.starts_with("adr/") {
        (
            "# Issue Index\n",
            "# ADR Index\n\n- [Pick](/adr/0001-pick.md)\n",
        )
    } else {
        (
            "# Issue Index\n\n- [Work](/issues/0001-work.md)\n",
            "# ADR Index\n",
        )
    };
    write(
        &bundle,
        "index.md",
        "# Docs\n\n- [Issues](/issues/index.md)\n- [ADRs](/adr/index.md)\n",
    );
    write(&bundle, "issues/index.md", issues);
    write(&bundle, "adr/index.md", adrs);
    write(&bundle, record_path, record);
    common::index_bundle(&bundle);
    bundle
}

fn check_text(label: &str, record_path: &str, record: &str) -> (Output, String) {
    let bundle = scaffold(label, record_path, record);
    let output = run_check(&bundle);
    let text = format!(
        "{}{}",
        stdout_of(&output),
        String::from_utf8_lossy(&output.stderr)
    );
    (output, text)
}

fn assert_flagged(label: &str, record_path: &str, record: &str) {
    let (output, text) = check_text(label, record_path, record);
    assert_eq!(output.status.code(), Some(1), "got: {text}");
    assert_eq!(
        text.matches("description --from -").count(),
        1,
        "got: {text}"
    );
    let fix_line = text
        .lines()
        .find(|line| line.contains("description --from -"))
        .unwrap();
    assert!(fix_line.contains(record_path), "got: {text}");
    assert!(fix_line.contains("living-docs write set"), "got: {text}");
}

fn assert_clean(label: &str, record_path: &str, record: &str) {
    let (output, text) = check_text(label, record_path, record);
    assert_eq!(output.status.code(), Some(0), "got: {text}");
}

#[test]
fn an_issue_whose_description_is_its_template_placeholder_fails() {
    let record = issue_with(Some(ISSUE_PLACEHOLDER));
    assert_flagged("exact", "issues/0001-work.md", &record);
}

#[test]
fn the_violation_names_the_record_reference_in_the_fix() {
    let record = issue_with(Some(ISSUE_PLACEHOLDER));
    let (_, text) = check_text("reference", "issues/0001-work.md", &record);
    assert!(
        text.contains("write set issue/0001 description --from -"),
        "got: {text}"
    );
}

#[test]
fn a_quoted_placeholder_description_fails() {
    let record = issue_with(Some(&format!("\"{ISSUE_PLACEHOLDER}\"")));
    assert_flagged("quoted", "issues/0001-work.md", &record);
}

#[test]
fn a_placeholder_description_with_surrounding_spaces_fails() {
    let record = issue_with(Some(&format!("   {ISSUE_PLACEHOLDER}   ")));
    assert_flagged("spaced", "issues/0001-work.md", &record);
}

#[test]
fn an_adr_whose_description_is_its_template_placeholder_fails() {
    assert_flagged("adr", "adr/0001-pick.md", &adr_with(ADR_PLACEHOLDER));
}

#[test]
fn another_types_placeholder_on_an_issue_fails() {
    let record = issue_with(Some(ADR_PLACEHOLDER));
    assert_flagged("other-type", "issues/0001-work.md", &record);
}

#[test]
fn a_filled_description_passes() {
    let record = issue_with(Some("A real sentence."));
    assert_clean("filled", "issues/0001-work.md", &record);
}

#[test]
fn a_description_quoting_the_placeholder_inside_a_longer_sentence_passes() {
    let record = issue_with(Some(&format!(
        "The scaffold writes {ISSUE_PLACEHOLDER} until filled."
    )));
    assert_clean("quoting", "issues/0001-work.md", &record);
}

#[test]
fn a_quoted_placeholder_description_with_inner_padding_fails() {
    let record = issue_with(Some(&format!("\"   {ISSUE_PLACEHOLDER}   \"")));
    assert_flagged("padded", "issues/0001-work.md", &record);
}

fn scaffold_with_placeholder(label: &str, args: &[&str]) -> common::Scratch {
    let bundle = common::temp_bundle("placeholder-description", label);
    let created = living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap(), "write", "new"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        created.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&created.stderr)
    );
    bundle
}

fn printed_fix_args(text: &str) -> Vec<String> {
    let line = text
        .lines()
        .find(|line| line.contains("description --from -"))
        .unwrap_or_else(|| panic!("no description violation in: {text}"));
    let command = line.split('`').nth(1).unwrap();
    command
        .split_whitespace()
        .skip(1)
        .map(str::to_string)
        .collect()
}

fn run_printed_fix(bundle: &common::Scratch) {
    let text = stdout_of(&run_check(bundle));
    let mut child = living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap()])
        .args(printed_fix_args(&text))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(&mut child.stdin.take().unwrap(), b"A real sentence.\n").unwrap();
    let fixed = child.wait_with_output().unwrap();
    assert!(
        fixed.status.success(),
        "printed fix refused: {} (from: {text})",
        String::from_utf8_lossy(&fixed.stderr)
    );
    let after = stdout_of(&run_check(bundle));
    assert!(!after.contains("description --from -"), "got: {after}");
}

#[test]
fn the_printed_fix_for_an_issue_is_accepted_and_clears_the_violation() {
    let bundle = scaffold_with_placeholder("fix-issue", &["issue", "Work"]);
    run_printed_fix(&bundle);
}

#[test]
fn the_printed_fix_for_a_view_is_accepted_and_clears_the_violation() {
    let bundle = scaffold_with_placeholder("fix-view", &["view", "Rings", "--kind", "component"]);
    run_printed_fix(&bundle);
}

/// A view `write new` never names that way, or one nested below its type
/// directory, has no `view/<slug>` reference: offering one would be refused,
/// or would set the description of the top-level view of the same stem.
#[test]
fn a_view_outside_the_named_form_is_not_offered_a_view_reference() {
    let bundle = scaffold_with_placeholder("odd-views", &["view", "Rings", "--kind", "component"]);
    let rings = std::fs::read_to_string(bundle.join("architecture/rings.md")).unwrap();
    write(&bundle, "architecture/sub/rings.md", &rings);
    write(&bundle, "architecture/My_View.md", &rings);

    let text = stdout_of(&run_check(&bundle));

    assert_eq!(text.matches("write set view/rings ").count(), 1, "{text}");
    assert!(!text.contains("view/My_View"), "{text}");
    assert_eq!(
        text.matches("no `write set` reference reaches it").count(),
        2,
        "{text}"
    );
}

#[test]
fn a_record_of_no_registered_type_is_told_to_set_one() {
    let record = issue_with(Some(ISSUE_PLACEHOLDER)).replace("type: Issue", "type: Memo");
    let (_, text) = check_text("unknown-type", "notes/0001-memo.md", &record);

    assert!(text.contains("has no registered `type`"), "{text}");
}

/// A numbered record nested below its type directory gets no `TYPE/NNNN`
/// reference, which would reach the top-level record of the same number.
#[test]
fn a_nested_numbered_record_is_not_offered_its_number() {
    let bundle = scaffold_with_placeholder("nested-adr", &["adr", "Store"]);
    let adr = std::fs::read_to_string(bundle.join("adr/0001-store.md")).unwrap();
    write(&bundle, "adr/old/0001-store.md", &adr);

    let text = stdout_of(&run_check(&bundle));

    assert_eq!(text.matches("write set adr/0001 ").count(), 1, "{text}");
    assert_eq!(
        text.matches("no `write set` reference reaches it").count(),
        1,
        "{text}"
    );
}

#[test]
fn the_printed_fix_for_the_constitution_is_accepted_and_clears_the_violation() {
    let bundle = scaffold_with_placeholder("fix-constitution", &["constitution", "P"]);
    run_printed_fix(&bundle);
}

#[test]
fn a_record_with_no_description_key_gets_no_violation_from_this_rule() {
    let (_, text) = check_text("missing", "issues/0001-work.md", &issue_with(None));
    assert!(!text.contains("description --from -"), "got: {text}");
}

#[test]
fn a_new_issue_given_a_description_and_a_filled_body_passes_check() {
    let bundle = common::temp_bundle("placeholder-description", "new");
    let body_dir = common::temp_bundle("placeholder-description", "new-body");
    let body = body_dir.join("body.md");
    std::fs::write(&body, BODY).unwrap();
    let created = living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap()])
        .args(["write", "new", "issue", "Fresh work"])
        .args(["--description", "A real sentence."])
        .args(["--body", body.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        created.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&created.stderr)
    );
    let output = run_check(&bundle);
    assert_eq!(output.status.code(), Some(0), "got: {}", stdout_of(&output));
}
