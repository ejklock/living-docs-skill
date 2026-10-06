//! `check --changed-files` (ADR 0062): the gate's verdict narrows to the
//! records a commit touched, so a bundle carrying legacy debt can still gate
//! what it changes.

use std::path::{Path, PathBuf};
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, write};

fn temp_bundle(label: &str) -> PathBuf {
    common::temp_bundle("check-changed-files", label)
}

/// A bundle with one clean record and one carrying an unfilled placeholder —
/// the debt a brownfield adopter inherits.
fn dirty_bundle(label: &str) -> PathBuf {
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

fn run_scoped(bundle: &Path, changed: &[&str]) -> Output {
    let mut command = living_docs();
    command.arg("check").arg("--plain").arg("--changed-files");
    for rel in changed {
        command.arg(bundle.join(rel));
    }
    command
        .arg("--")
        .arg(bundle)
        .output()
        .expect("failed to run living-docs check")
}

#[test]
fn a_commit_touching_a_clean_record_passes_a_bundle_that_is_dirty_elsewhere() {
    let bundle = dirty_bundle("clean-record");

    let output = run_scoped(&bundle, &["adr/0001-clean.md"]);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(0), "got:\n{stdout}");
    assert!(!stdout.contains("0002-dirty.md"), "got:\n{stdout}");
}

#[test]
fn a_commit_touching_the_dirty_record_still_fails_and_names_only_that_record() {
    let bundle = dirty_bundle("dirty-record");

    let output = run_scoped(&bundle, &["adr/0002-dirty.md"]);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1), "got:\n{stdout}");
    assert!(stdout.contains("0002-dirty.md"), "got:\n{stdout}");
    assert!(stdout.contains("PLACEHOLDER"), "got:\n{stdout}");
    assert!(!stdout.contains("0001-clean.md"), "got:\n{stdout}");
}

#[test]
fn without_the_flag_the_same_bundle_still_fails_on_the_legacy_debt() {
    let bundle = dirty_bundle("unscoped");

    let output = living_docs()
        .args(["check", "--plain"])
        .arg(&bundle)
        .output()
        .expect("failed to run living-docs check");
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1), "got:\n{stdout}");
    assert!(stdout.contains("0002-dirty.md"), "got:\n{stdout}");
}
