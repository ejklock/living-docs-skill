//! Integration coverage for record liveness (ADR 0049, trimmed by ADR 0057):
//! `check` emits a `stale-proposed` advisory for a `Proposed` ADR whose linked
//! issue is terminal, and never changes the exit code.

use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, write};

fn liveness_bundle(label: &str) -> std::path::PathBuf {
    common::temp_bundle("liveness", label)
}

fn run_check(bundle: &Path) -> Output {
    living_docs()
        .args(["check", bundle.to_str().unwrap(), "--plain"])
        .output()
        .expect("failed to run living-docs check")
}

fn write_stale_proposed_bundle(bundle: &Path) {
    write(
        bundle,
        "index.md",
        "---\nokf_version: \"1.0\"\n---\n# Docs\n\n* [ADRs](adr/index.md)\n* [Issues](issues/index.md)\n",
    );
    write(
        bundle,
        "adr/index.md",
        "# ADRs\n\n## Active\n\n* [0001 — Proposed](0001-proposed.md) - Proposed\n",
    );
    write(
        bundle,
        "adr/0001-proposed.md",
        "---\ntype: ADR\ntitle: Proposed\ndescription: d\nowner: x\nstatus: Proposed\n---\n\n# 0001. Proposed\n\n## Context\n\nSee [issue](/issues/0001-task.md).\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n",
    );
    write(
        bundle,
        "issues/index.md",
        "# Issues\n\n## Closed\n\n* [0001 — Task](0001-task.md) - closed\n",
    );
    write(
        bundle,
        "issues/0001-task.md",
        "---\ntype: Issue\ntitle: Task\ndescription: d\nstatus: closed\n---\n\n# 0001. Task\n",
    );
}

/// A `Proposed` ADR linking a `closed` issue prints a `stale-proposed`
/// advisory, and the bundle still exits 0 — liveness advises, never gates.
#[test]
fn check_flags_stale_proposed_without_gating() {
    let bundle = liveness_bundle("stale-proposed");
    write_stale_proposed_bundle(&bundle);

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "liveness must not gate; got:\n{stdout}"
    );
    assert!(
        stdout.contains("0001-proposed.md") && stdout.contains("stale-proposed"),
        "got:\n{stdout}"
    );
}
