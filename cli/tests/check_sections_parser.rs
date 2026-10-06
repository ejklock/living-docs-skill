//! The required-section pass reads headings the way a Markdown reader does:
//! indented code is not a heading and only a title-matching first heading is
//! the record's title.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

mod common;
use common::{living_docs, run_check, stdout_of, temp_bundle, write};

const SCOPE: &str = "### Scope\n\nSome scope.\n\n";
const ACCEPTANCE: &str = "### Acceptance\n\n- x\n";

fn record(title: &str, body: &str) -> String {
    format!("---\ntype: Issue\ntitle: {title}\ndescription: d.\nstatus: open\n---\n\n{body}")
}

fn bundle_with(label: &str, record: &str) -> PathBuf {
    let bundle = temp_bundle("sections-parser", label);
    write(
        &bundle,
        "index.md",
        "# Docs\n\n- [Issues](/issues/index.md)\n",
    );
    write(
        &bundle,
        "issues/index.md",
        "# Issue Index\n\n- [Work](/issues/0001-work.md)\n",
    );
    write(&bundle, "issues/0001-work.md", record);
    bundle
}

fn check_bundle(bundle: &Path) -> (Output, String) {
    let output = run_check(bundle);
    let text = stdout_of(&output) + &String::from_utf8_lossy(&output.stderr);
    let _ = fs::remove_dir_all(bundle.parent().unwrap());
    (output, text)
}

fn assert_passes(label: &str, record: &str) -> String {
    let (output, text) = check_bundle(&bundle_with(label, record));
    assert_eq!(output.status.code(), Some(0), "got: {text}");
    text
}

fn assert_lacks_acceptance(label: &str, record: &str) {
    let (output, text) = check_bundle(&bundle_with(label, record));
    assert_ne!(output.status.code(), Some(0), "got: {text}");
    assert!(text.contains("section 'Acceptance'"), "got: {text}");
}

#[test]
fn an_acceptance_heading_indented_four_spaces_does_not_count() {
    let body = format!("## Work\n\n{SCOPE}    ### Acceptance\n\n- x\n");
    assert_lacks_acceptance("four-spaces", &record("Work", &body));
}

#[test]
fn an_acceptance_heading_indented_with_a_tab_does_not_count() {
    let body = format!("## Work\n\n{SCOPE}\t### Acceptance\n\n- x\n");
    assert_lacks_acceptance("tab", &record("Work", &body));
}

#[test]
fn an_acceptance_heading_indented_three_spaces_counts() {
    let body = format!("## Work\n\n{SCOPE}   ### Acceptance\n\n- x\n");
    assert_passes("three-spaces", &record("Work", &body));
}

#[test]
fn read_contract_keeps_an_indented_plan_line_inside_scope() {
    let body = "## Work\n\n### Scope\n\nhead\n\n    ### Plan\n\ntail\n";
    let bundle = bundle_with("contract", &record("Work", body));
    let output = living_docs()
        .arg("--docs-dir")
        .arg(&bundle)
        .args(["read", "--contract", "--plain"])
        .output()
        .expect("failed to run read");
    let text = stdout_of(&output);
    let _ = fs::remove_dir_all(bundle.parent().unwrap());
    assert!(text.contains("### Plan"), "got: {text}");
    assert!(text.contains("tail"), "got: {text}");
}

#[test]
fn a_body_with_no_title_heading_keeps_its_first_section() {
    assert_passes("untitled", &record("Work", &format!("{SCOPE}{ACCEPTANCE}")));
}

#[test]
fn a_title_heading_matching_the_frontmatter_title_is_not_a_section() {
    let body = format!("## 0001. My title\n\n{SCOPE}{ACCEPTANCE}");
    assert_passes("title-match", &record("My title", &body));
}

#[test]
fn a_disagreeing_title_heading_still_passes_and_keeps_the_advisory() {
    let body = format!("## Something else\n\n{SCOPE}{ACCEPTANCE}");
    let text = assert_passes("title-differs", &record("My title", &body));
    assert!(text.contains("HEADING"), "got: {text}");
}

#[test]
fn a_title_heading_is_not_a_section_even_when_the_title_is_acceptance() {
    let body = format!("## Acceptance\n\n{SCOPE}");
    assert_lacks_acceptance("title-is-acceptance", &record("Acceptance", &body));
}

#[test]
fn a_draft_prd_without_problem_motivation_fails_naming_it() {
    let prd = "---\ntype: PRD\ntitle: Doc\ndescription: d.\nstatus: Draft\n---\n\n# 0001. Doc\n\n## Non-goals\n\nn\n";
    let bundle = temp_bundle("sections-parser", "prd");
    write(&bundle, "index.md", "# Docs\n\n- [Prd](/prd/index.md)\n");
    write(
        &bundle,
        "prd/index.md",
        "# Index\n\n- [Doc](/prd/0001-doc.md)\n",
    );
    write(&bundle, "prd/0001-doc.md", prd);
    let (output, text) = check_bundle(&bundle);
    assert_ne!(output.status.code(), Some(0), "got: {text}");
    assert!(
        text.contains("section 'Problem / Motivation'"),
        "got: {text}"
    );
}
