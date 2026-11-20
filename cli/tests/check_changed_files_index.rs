//! `check --changed-files`: a stale type index is in scope of a record of
//! that type, and of no other type's record. The staleness is a status the
//! index rows carry, so no membership check reports it instead.

use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, write};

const ADR: &str = "---\ntype: ADR\ntitle: TITLE\ndescription: A record.\nowner: a@b.c\nstatus: Accepted\n---\n\n# NUM. TITLE\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n";
const ISSUE: &str = "---\ntype: Issue\ntitle: X\ndescription: A filled description.\nstatus: open\ntimestamp: 2026-11-20T16:31:10Z\n---\n\n## 0001. X\n\n### Scope\n\nOne thing.\n\n### Acceptance\n\n- It works.\n";

fn adr(number: &str, title: &str) -> String {
    ADR.replace("NUM", number).replace("TITLE", title)
}

fn bundle_with_stale_adr_index(label: &str) -> common::Scratch {
    let bundle = common::temp_bundle("check-changed-files-index", label);
    write(
        &bundle,
        "index.md",
        "# Docs\n\n* [ADRs](adr/)\n* [Issues](issues/)\n",
    );
    write(&bundle, "adr/0001-a.md", &adr("0001", "A"));
    write(&bundle, "adr/0002-b.md", &adr("0002", "B"));
    write(&bundle, "adr/0003-with space.md", &adr("0003", "C"));
    write(&bundle, "issues/0001-x.md", ISSUE);
    common::index_bundle(&bundle);
    let index = bundle.join("adr/index.md");
    let current = std::fs::read_to_string(&index).unwrap();
    let stale = current.replace("Accepted", "Proposed");
    assert_ne!(stale, current, "the fixture must change the rows");
    std::fs::write(&index, stale).unwrap();
    bundle
}

fn run_scoped(bundle: &Path, changed: &[String]) -> Output {
    let mut command = living_docs();
    command.arg("check").arg("--plain");
    if !changed.is_empty() {
        command.arg("--changed-files").args(changed);
        command.arg("--");
    }
    command.arg(bundle).output().expect("failed to run check")
}

fn relative(path: &str) -> Vec<String> {
    vec![path.to_owned()]
}

fn assert_index_finding(output: &Output) {
    let stdout = stdout_of(output);
    assert_eq!(output.status.code(), Some(1), "got:\n{stdout}");
    assert!(stdout.contains("adr/index.md"), "got:\n{stdout}");
}

#[test]
fn a_record_in_scope_reports_its_types_stale_index() {
    let bundle = bundle_with_stale_adr_index("relative");

    assert_index_finding(&run_scoped(&bundle, &relative("adr/0002-b.md")));
}

#[test]
fn an_absolute_record_path_reports_its_types_stale_index() {
    let bundle = bundle_with_stale_adr_index("absolute");
    let absolute = bundle.join("adr/0002-b.md").display().to_string();

    assert_index_finding(&run_scoped(&bundle, &[absolute]));
}

#[test]
fn a_record_filename_with_a_space_reports_its_types_stale_index() {
    let bundle = bundle_with_stale_adr_index("space");

    assert_index_finding(&run_scoped(&bundle, &relative("adr/0003-with space.md")));
}

#[test]
fn a_record_of_another_type_does_not_report_the_stale_index() {
    let bundle = bundle_with_stale_adr_index("other-type");

    let output = run_scoped(&bundle, &relative("issues/0001-x.md"));

    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}

#[test]
fn the_index_itself_in_scope_reports_its_staleness() {
    let bundle = bundle_with_stale_adr_index("index");

    assert_index_finding(&run_scoped(&bundle, &relative("adr/index.md")));
}

#[test]
fn an_unscoped_check_reports_the_stale_index() {
    let bundle = bundle_with_stale_adr_index("unscoped");

    assert_index_finding(&run_scoped(&bundle, &[]));
}

const VIEW: &str = "---\ntype: Architecture View\ntitle: Context\ndescription: Who talks to the system.\nkind: context\ntimestamp: 2026-11-20T16:31:10Z\n---\n\n# Context\n\nThe actors around the system.\n\n```mermaid\nflowchart LR\n    A[User] --> B[System]\n```\n\n## Drift instrument\n\nDrift caught by: review of the diagram against the code in each release.\n";

#[test]
fn a_view_file_in_scope_reports_the_stale_architecture_index() {
    let bundle = common::temp_bundle("check-changed-files-index", "view");
    write(&bundle, "index.md", "# Docs\n\n* [Views](architecture/)\n");
    write(&bundle, "architecture/context.md", VIEW);
    common::index_bundle(&bundle);
    let index = bundle.join("architecture/index.md");
    let current = std::fs::read_to_string(&index).unwrap();
    std::fs::write(&index, current.replace("Context", "Outdated")).unwrap();
    let unscoped = run_scoped(&bundle, &[]);
    assert_eq!(unscoped.status.code(), Some(1), "{}", stdout_of(&unscoped));

    let output = run_scoped(&bundle, &relative("architecture/context.md"));
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1), "got:\n{stdout}");
    assert!(stdout.contains("architecture/index.md"), "got:\n{stdout}");
}

#[test]
fn a_current_index_passes_with_a_record_in_scope() {
    let bundle = bundle_with_stale_adr_index("current");
    common::index_bundle(&bundle);

    let output = run_scoped(&bundle, &relative("adr/0002-b.md"));

    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}
