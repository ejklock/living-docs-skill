//! `check <record.md>`: a record file named where a bundle root is expected
//! checks the enclosing bundle, reporting only that record's findings.

use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, write};

fn temp_bundle(label: &str) -> common::Scratch {
    common::temp_bundle("check-record-path", label)
}

/// A bundle with one clean record and one carrying an unfilled placeholder.
fn dirty_bundle(label: &str) -> common::Scratch {
    let bundle = temp_bundle(label);
    write(&bundle, "index.md", "# Docs\n\n* [ADRs](adr/)\n");
    write(
        &bundle,
        "adr/index.md",
        "# ADRs\n\n* [0001 — Clean](0001-clean.md) - Accepted\n* [0002 — Dirty](0002-dirty.md) - Accepted\n",
    );
    write(
        &bundle,
        "adr/0001-clean.md",
        "---\ntype: ADR\ntitle: Clean\ndescription: A clean record.\nowner: a@b.c\nstatus: Accepted\n---\n\n# 0001. Clean\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n",
    );
    write(
        &bundle,
        "adr/0002-dirty.md",
        "---\ntype: ADR\ntitle: Dirty\ndescription: A record with debt.\nowner: a@b.c\nstatus: Accepted\n---\n\n# 0002. Dirty\n\n{{UNFILLED: legacy debt}}\n",
    );
    bundle
}

/// The root-relative path of a record file inside the fixture bundle.
fn under_docs(name: &str) -> String {
    format!("docs/{}/{name}", "adr")
}

fn run_in(cwd: &Path, args: &[&str]) -> Output {
    living_docs()
        .current_dir(cwd)
        .arg("check")
        .arg("--plain")
        .args(args)
        .output()
        .expect("failed to run living-docs check")
}

fn repo_root(bundle: &Path) -> &Path {
    bundle.parent().unwrap()
}

#[test]
fn a_clean_record_path_from_the_repository_root_passes_despite_debt_elsewhere() {
    let bundle = dirty_bundle("clean");

    let output = run_in(repo_root(&bundle), &[&under_docs("0001-clean.md")]);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(0), "got:\n{stdout}");
    assert!(!stdout.contains("0002-dirty.md"), "got:\n{stdout}");
}

#[test]
fn a_broken_record_path_fails_naming_only_that_record() {
    let bundle = dirty_bundle("broken");

    let output = run_in(repo_root(&bundle), &[&under_docs("0002-dirty.md")]);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1), "got:\n{stdout}");
    assert!(stdout.contains("0002-dirty.md"), "got:\n{stdout}");
    assert!(stdout.contains("PLACEHOLDER"), "got:\n{stdout}");
    assert!(!stdout.contains("0001-clean.md"), "got:\n{stdout}");
}

#[test]
fn a_relative_path_from_a_subdirectory_behaves_like_the_root_relative_one() {
    let bundle = dirty_bundle("subdir");

    let output = run_in(&bundle.join("adr"), &["0002-dirty.md"]);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1), "got:\n{stdout}");
    assert!(stdout.contains("0002-dirty.md"), "got:\n{stdout}");
    assert!(!stdout.contains("0001-clean.md"), "got:\n{stdout}");
}

#[test]
fn an_absolute_record_path_behaves_like_the_relative_one() {
    let bundle = dirty_bundle("absolute");
    let clean = bundle.join("adr/0001-clean.md");
    let dirty = bundle.join("adr/0002-dirty.md");

    let ok = run_in(&std::env::temp_dir(), &[clean.to_str().unwrap()]);
    let bad = run_in(&std::env::temp_dir(), &[dirty.to_str().unwrap()]);
    let stdout = stdout_of(&bad);

    assert_eq!(ok.status.code(), Some(0), "got:\n{}", stdout_of(&ok));
    assert_eq!(bad.status.code(), Some(1), "got:\n{stdout}");
    assert!(stdout.contains("0002-dirty.md"), "got:\n{stdout}");
    assert!(!stdout.contains("0001-clean.md"), "got:\n{stdout}");
}

#[test]
fn a_bundle_root_still_checks_the_whole_bundle() {
    let bundle = dirty_bundle("whole");

    let output = run_in(repo_root(&bundle), &["docs"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout_of(&output).contains("0002-dirty.md"));
}

#[test]
fn a_record_outside_any_bundle_exits_two_with_the_bundle_root_message() {
    let bundle = temp_bundle("outside");
    write(repo_root(&bundle), "loose/note.md", "# Note\n");
    let note = repo_root(&bundle).join("loose/note.md");

    let output = run_in(repo_root(&bundle), &[note.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("bundle root not found"), "got:\n{stderr}");
}

#[test]
fn a_nonexistent_record_exits_two() {
    let bundle = dirty_bundle("missing");

    let output = run_in(repo_root(&bundle), &[&under_docs("0009-ghost.md")]);

    assert_eq!(output.status.code(), Some(2));
}

#[cfg(unix)]
#[test]
fn a_symlinked_record_resolves_to_its_targets_bundle() {
    let bundle = dirty_bundle("symlink");
    let link = repo_root(&bundle).join("link.md");
    std::os::unix::fs::symlink(bundle.join("adr/0002-dirty.md"), &link).unwrap();

    let output = run_in(repo_root(&bundle), &["link.md"]);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1), "got:\n{stdout}");
    assert!(stdout.contains("0002-dirty.md"), "got:\n{stdout}");
    assert!(!stdout.contains("0001-clean.md"), "got:\n{stdout}");
}

#[test]
fn json_for_a_record_path_has_the_shape_of_changed_files_json() {
    let bundle = dirty_bundle("json");
    let root = repo_root(&bundle);

    let by_path = living_docs()
        .current_dir(root)
        .args(["check", "--json", &under_docs("0002-dirty.md")])
        .output()
        .unwrap();
    let by_flag = living_docs()
        .current_dir(root)
        .args([
            "check",
            "--json",
            "--changed-files",
            &under_docs("0002-dirty.md"),
        ])
        .args(["--", "docs"])
        .output()
        .unwrap();

    assert_eq!(stdout_of(&by_path), stdout_of(&by_flag));
}

/// A bundle whose ADR links the issue record by a bundle-root-relative path,
/// so the link resolves only against the real bundle root.
fn linked_bundle(label: &str) -> common::Scratch {
    let bundle = temp_bundle(label);
    write(
        &bundle,
        "index.md",
        "# Docs\n\n* [ADRs](adr/)\n* [Issues](issues/)\n",
    );
    write(
        &bundle,
        "adr/index.md",
        "# ADRs\n\n* [0002 — Linked](0002-linked.md) - Accepted\n",
    );
    write(
        &bundle,
        "issues/index.md",
        "# Issues\n\n* [0001 — Work](0001-work.md) - open\n",
    );
    write(
        &bundle,
        "adr/0002-linked.md",
        "---\ntype: ADR\ntitle: Linked\ndescription: Links an issue.\nowner: a@b.c\nstatus: Accepted\n---\n\n# 0002. Linked\n\n## Context\n\nSee [the issue](/issues/0001-work.md).\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n",
    );
    write(
        &bundle,
        "issues/0001-work.md",
        "---\ntype: Issue\ntitle: Work\ndescription: Some work.\nowner: a@b.c\nstatus: open\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n## Work\n\nDo it.\n\n### Acceptance\n\n- It works.\n",
    );
    bundle
}

#[test]
fn an_index_above_the_bundle_does_not_move_the_bundle_root() {
    let bundle = linked_bundle("site-index");
    write(repo_root(&bundle), "index.md", "# Site\n");
    let record = under_docs("0002-linked.md");

    let by_path = run_in(repo_root(&bundle), &[&record]);
    let by_flag = run_in(
        repo_root(&bundle),
        &["--changed-files", &record, "--", "docs"],
    );

    assert_eq!(
        by_path.status.code(),
        Some(0),
        "got:\n{}",
        stdout_of(&by_path)
    );
    assert_eq!(stdout_of(&by_path), stdout_of(&by_flag));
}

/// Runs `living-docs <global args> check --json <record>` from `cwd`.
fn check_json(cwd: &Path, global: &[&str], record: &str) -> Output {
    living_docs()
        .current_dir(cwd)
        .args(global)
        .args(["check", "--json", record])
        .output()
        .unwrap()
}

fn names_docs_bundle(output: &Output) -> bool {
    stdout_of(output).contains("\"bundle\":\"docs\"")
}

#[test]
fn json_names_the_docs_directory_as_the_bundle_despite_an_index_above() {
    let bundle = linked_bundle("site-json");
    write(repo_root(&bundle), "index.md", "# Site\n");

    let output = check_json(repo_root(&bundle), &[], &under_docs("0002-linked.md"));

    assert!(names_docs_bundle(&output), "got:\n{}", stdout_of(&output));
}

#[test]
fn a_record_under_a_custom_docs_dir_resolves_to_it() {
    let bundle = dirty_bundle("custom");
    write(&bundle, "notes/free.md", "# Free\n");
    let root = repo_root(&bundle);

    let resolved = check_json(root, &["--docs-dir", "docs"], "docs/notes/free.md");
    let unresolved = check_json(root, &["--docs-dir", "elsewhere"], "docs/notes/free.md");

    assert!(
        names_docs_bundle(&resolved),
        "got:\n{}",
        stdout_of(&resolved)
    );
    assert_eq!(unresolved.status.code(), Some(2));
}

#[test]
fn a_record_in_a_research_session_directory_resolves_to_the_bundle() {
    let bundle = dirty_bundle("research");
    write(&bundle, "research/index.md", "# Research\n\n* [S](s1/)\n");
    write(&bundle, "research/s1/index.md", "# S\n");
    write(&bundle, "research/s1/part.md", "# Part\n");

    let output = check_json(repo_root(&bundle), &[], "docs/research/s1/part.md");

    assert!(names_docs_bundle(&output), "got:\n{}", stdout_of(&output));
}
