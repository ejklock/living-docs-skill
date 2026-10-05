//! `check` requires the sections a doc type's registry row marks required on
//! every live record of that type.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

mod common;
use common::{living_docs, run_check, stdout_of, write};

const SCOPE_AND_ACCEPTANCE: &str = "### Scope\n\nSome scope.\n\n### Acceptance\n\n- a criterion\n";

fn issue(status: &str, body: &str) -> String {
    format!(
        "---\ntype: Issue\ntitle: Work\ndescription: A work item.\nstatus: {status}\n---\n\n## Work\n\n{body}"
    )
}

fn bundle_with(label: &str, record: &str) -> PathBuf {
    let bundle = common::temp_bundle("sections", label);
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

fn check_record(label: &str, record: &str) -> (Output, String) {
    let bundle = bundle_with(label, record);
    let output = run_check(&bundle);
    let text = format!(
        "{}{}",
        stdout_of(&output),
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = fs::remove_dir_all(bundle.parent().unwrap());
    (output, text)
}

fn assert_passes(label: &str, record: &str) {
    let (output, text) = check_record(label, record);
    assert_eq!(output.status.code(), Some(0), "got: {text}");
}

fn assert_missing(label: &str, record: &str, missing: &[&str], present: &[&str]) {
    let (output, text) = check_record(label, record);
    assert_ne!(output.status.code(), Some(0), "got: {text}");
    assert!(text.contains("0001-work.md"), "got: {text}");
    for name in missing {
        assert!(text.contains(&format!("section '{name}'")), "got: {text}");
    }
    for name in present {
        assert!(!text.contains(&format!("section '{name}'")), "got: {text}");
    }
}

#[test]
fn an_open_issue_with_scope_and_acceptance_passes() {
    assert_passes("ok", &issue("open", SCOPE_AND_ACCEPTANCE));
}

#[test]
fn an_open_issue_without_acceptance_fails_naming_file_and_section() {
    assert_missing(
        "noacc",
        &issue("open", "### Scope\n\nSome scope.\n"),
        &["Acceptance"],
        &["Scope"],
    );
}

#[test]
fn an_in_progress_issue_without_acceptance_fails() {
    assert_missing(
        "inprog",
        &issue("in-progress", "### Scope\n\nSome scope.\n"),
        &["Acceptance"],
        &[],
    );
}

#[test]
fn an_issue_missing_both_sections_reports_both() {
    assert_missing(
        "both",
        &issue("open", "Just prose.\n"),
        &["Scope", "Acceptance"],
        &[],
    );
}

#[test]
fn retired_issues_without_the_sections_pass() {
    for status in ["closed", "done", "Closed"] {
        assert_passes(
            &format!("retired-{status}"),
            &issue(status, "Just prose.\n"),
        );
    }
}

#[test]
fn a_superseded_issue_gets_no_section_finding() {
    let (_, text) = check_record("superseded", &issue("Superseded", "Just prose.\n"));
    assert!(!text.contains("section '"), "got: {text}");
}

#[test]
fn heading_level_and_case_do_not_matter() {
    for acceptance in [
        "## Acceptance",
        "### acceptance",
        "#### ACCEPTANCE",
        "### Acceptance  ",
    ] {
        let body = format!("### Scope\n\ns\n\n{acceptance}\n\n- c\n");
        assert_passes("levels", &issue("open", &body));
    }
}

#[test]
fn a_longer_heading_does_not_satisfy_the_requirement() {
    let body = "### Scope\n\ns\n\n### Acceptance criteria\n\n- c\n";
    assert_missing("longer", &issue("open", body), &["Acceptance"], &[]);
}

#[test]
fn a_heading_inside_a_fenced_code_block_does_not_count() {
    let body = "### Scope\n\ns\n\n```\n### Acceptance\n```\n";
    assert_missing("fenced", &issue("open", body), &["Acceptance"], &[]);
}

#[test]
fn the_title_heading_never_counts_as_a_section() {
    let record = "---\ntype: Issue\ntitle: Acceptance\ndescription: d.\nstatus: open\n---\n\n## Acceptance\n\n### Scope\n\ns\n";
    assert_missing("title", record, &["Acceptance"], &[]);
}

#[test]
fn a_type_with_an_empty_schema_never_gets_the_finding() {
    let view = "---\ntype: Architecture View\ntitle: Doc\ndescription: d.\nkind: context\n---\n\n## Doc\n\nProse.\n";
    let (output, text) = check_files(
        "view",
        &[
            (
                "index.md",
                "# Docs\n\n- [Architecture](/architecture/index.md)\n",
            ),
            (
                "architecture/index.md",
                "# Architecture\n\n- [Doc](/architecture/doc.md)\n",
            ),
            ("architecture/doc.md", view),
        ],
    );
    assert_eq!(output.status.code(), Some(0), "got: {text}");
}

#[test]
fn a_file_without_a_recognized_type_is_skipped() {
    let record = "---\ntype: Note\ntitle: Work\ndescription: d.\nstatus: open\n---\n\nProse.\n";
    assert_passes("unknown-type", record);
}

fn check_changed(bundle: &Path, changed: &Path) -> Output {
    living_docs()
        .args(["check", "--plain", "--changed-files"])
        .arg(changed)
        .arg("--")
        .arg(bundle)
        .output()
        .expect("failed to run living-docs check")
}

#[test]
fn changed_files_scopes_the_finding_to_touched_records() {
    let bundle = bundle_with("changed", &issue("open", "Just prose.\n"));
    let untouched = check_changed(&bundle, &bundle.join("index.md"));
    let touched = check_changed(&bundle, &bundle.join("issues/0001-work.md"));
    assert_eq!(
        untouched.status.code(),
        Some(0),
        "got: {}",
        stdout_of(&untouched)
    );
    assert_ne!(
        touched.status.code(),
        Some(0),
        "got: {}",
        stdout_of(&touched)
    );
    let _ = fs::remove_dir_all(bundle.parent().unwrap());
}

fn adr(status: &str, body: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: Doc\ndescription: d.\nowner: me\nstatus: {status}\n---\n\n# 0001. Doc\n\n{body}"
    )
}

fn check_files(label: &str, files: &[(&str, &str)]) -> (Output, String) {
    let bundle = common::temp_bundle("sections", label);
    for (rel, contents) in files {
        write(&bundle, rel, contents);
    }
    let output = run_check(&bundle);
    let err = String::from_utf8_lossy(&output.stderr).into_owned();
    let text = stdout_of(&output) + &err;
    let _ = fs::remove_dir_all(bundle.parent().unwrap());
    (output, text)
}

fn check_numbered(dir: &str, label: &str, record: &str) -> (Output, String) {
    let root = format!("# Docs\n\n- [Index](/{dir}/index.md)\n");
    let index = format!("# Index\n\n- [Doc](/{dir}/0001-doc.md)\n");
    let (index_path, path) = (format!("{dir}/index.md"), format!("{dir}/0001-doc.md"));
    let files = [("index.md", &*root), (&index_path, &index), (&path, record)];
    check_files(label, &files)
}

fn assert_numbered_fails(dir: &str, label: &str, record: &str, section: &str) {
    let (output, text) = check_numbered(dir, label, record);
    assert_ne!(output.status.code(), Some(0), "got: {text}");
    let needle = format!("section '{section}'");
    assert!(text.contains(&needle), "got: {text}");
}

fn assert_numbered_passes(dir: &str, label: &str, record: &str) {
    let (output, text) = check_numbered(dir, label, record);
    assert_eq!(output.status.code(), Some(0), "got: {text}");
}

const CONTEXT: &str = "## Context\n\nc\n\n";
const DECISION: &str = "## Decision\n\nd\n\n";
const CONSEQUENCES: &str = "## Consequences\n\nq\n";

#[test]
fn an_accepted_adr_without_consequences_fails() {
    let record = adr("Accepted", &format!("{CONTEXT}{DECISION}"));
    assert_numbered_fails("adr", "adr-nocons", &record, "Consequences");
}

#[test]
fn a_proposed_adr_without_context_fails() {
    let record = adr("Proposed", &format!("{DECISION}{CONSEQUENCES}"));
    assert_numbered_fails("adr", "adr-noctx", &record, "Context");
}

#[test]
fn a_deprecated_adr_without_consequences_passes() {
    let callout = "> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read` for what is in force.\n\n";
    let record =
        adr("Deprecated", "Prose.\n").replace("# 0001. Doc", &format!("{callout}# 0001. Doc"));
    assert_numbered_passes("adr", "adr-dep", &record);
}

#[test]
fn an_adr_with_only_its_required_sections_passes() {
    let record = adr("Accepted", &format!("{CONTEXT}{DECISION}{CONSEQUENCES}"));
    assert_numbered_passes("adr", "adr-min", &record);
}

#[test]
fn a_draft_prd_without_non_goals_fails() {
    let record = "---\ntype: PRD\ntitle: Doc\ndescription: d.\nstatus: Draft\n---\n\n# 0001. Doc\n\n## Problem / Motivation\n\np\n";
    assert_numbered_fails("prd", "prd-nong", record, "Non-goals");
}

#[test]
fn research_with_none_of_its_template_sections_passes() {
    for status in ["Draft", "Accepted"] {
        let record = format!(
            "---\ntype: Research\ntitle: Doc\ndescription: d.\nstatus: {status}\n---\n\n# 0001. Doc\n\nFree-form notes.\n"
        );
        assert_numbered_passes("research", &format!("res-{status}"), &record);
    }
}

#[test]
fn a_constitution_without_non_negotiables_fails() {
    let constitution = "---\ntype: Constitution\ntitle: C\ndescription: d.\nstatus: Draft\n---\n\n# C\n\n## Product\n\np\n\n## Scope Boundaries\n\ns\n";
    let (output, text) = check_files(
        "const",
        &[
            ("index.md", "# Docs\n\n- [Constitution](/constitution.md)\n"),
            ("constitution.md", constitution),
        ],
    );
    assert_ne!(output.status.code(), Some(0), "got: {text}");
    assert!(text.contains("section 'Non-negotiables'"), "got: {text}");
}
