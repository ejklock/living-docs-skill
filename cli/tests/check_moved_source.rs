use std::fs;
use std::path::{Path, PathBuf};

mod common;
use common::{run_check, stdout_of, write};

fn temp_bundle(label: &str) -> PathBuf {
    common::temp_bundle("moved-source", label)
}

fn assert_clean(bundle: &Path) {
    let output = run_check(bundle);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected clean, got:\n{stdout}"
    );
    assert!(!stdout.contains("MOVED-SOURCE"), "got:\n{stdout}");

    let _ = fs::remove_dir_all(bundle);
}

/// The exact retired-record callout line a fixture record must open with,
/// sourced from the same module `living-docs fmt` writes through.
fn superseded_callout(successor: &str) -> String {
    living_docs_core::callout::expected(Some("superseded"), Some(successor))
        .expect("a superseded status always yields a callout")
}

fn deprecated_callout() -> String {
    living_docs_core::callout::expected(Some("deprecated"), None)
        .expect("a deprecated status always yields a callout")
}

fn write_moved_source_tree(bundle: &Path, dependent_body: &str) {
    write(
        bundle,
        "index.md",
        "# Index\n\n- [A](a.md)\n- [B](b.md)\n- [C](c.md)\n",
    );
    write(
        bundle,
        "a.md",
        &format!("---\ntype: Reference\ntitle: A\ndescription: Dependent record.\n---\n# A\n\n{dependent_body}\n"),
    );
    write(
        bundle,
        "b.md",
        &format!(
            "---\ntype: Reference\ntitle: B\ndescription: Moved source.\nstatus: Superseded\nsuperseded_by: c\n---\n{}\n\n# B\n",
            superseded_callout("c.md")
        ),
    );
    write(
        bundle,
        "c.md",
        "---\ntype: Reference\ntitle: C\ndescription: Successor.\n---\n# C\n",
    );
}

#[test]
fn moved_source_finding_names_dependent_source_status_and_successor_and_stays_exit_zero() {
    let bundle = temp_bundle("finding");
    write_moved_source_tree(&bundle, "[b](./b.md)");

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "advisory-only finding must not change the exit code, got:\n{stdout}"
    );
    assert!(stdout.contains("MOVED-SOURCE"), "got:\n{stdout}");
    assert!(stdout.contains("a.md"), "got:\n{stdout}");
    assert!(stdout.contains("b.md"), "got:\n{stdout}");
    assert!(stdout.contains("Superseded"), "got:\n{stdout}");
    assert!(stdout.contains("superseded by c"), "got:\n{stdout}");

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn linking_the_successor_clears_the_moved_source_finding() {
    let bundle = temp_bundle("cleared");
    write_moved_source_tree(&bundle, "[b](./b.md) [c](./c.md)");

    assert_clean(&bundle);
}

#[test]
fn a_done_issue_linking_a_superseded_record_is_clean() {
    let bundle = temp_bundle("done-issue");
    write(&bundle, "index.md", "# Index\n\n- [A](a.md)\n- [B](b.md)\n");
    write(
        &bundle,
        "a.md",
        "---\ntype: Issue\ntitle: A\ndescription: Dependent record.\nstatus: done\n---\n# A\n\n[b](./b.md)\n",
    );
    write(
        &bundle,
        "b.md",
        &format!(
            "---\ntype: Issue\ntitle: B\ndescription: Moved source.\nstatus: Deprecated\n---\n{}\n\n# B\n\n### Scope\n\n### Acceptance\n",
            deprecated_callout()
        ),
    );

    assert_clean(&bundle);
}

#[test]
fn a_record_linking_its_own_predecessor_is_clean() {
    let bundle = temp_bundle("self-successor");
    write(
        &bundle,
        "index.md",
        "# Index\n\n- [New](new.md)\n- [Old](old.md)\n",
    );
    write(
        &bundle,
        "new.md",
        "---\ntype: ADR\ntitle: New\ndescription: Supersedes Old.\nstatus: Accepted\nsupersedes: old\n---\n# New\n\n[old](./old.md)\n",
    );
    write(
        &bundle,
        "old.md",
        &format!(
            "---\ntype: ADR\ntitle: Old\ndescription: Superseded by New.\nstatus: Superseded\nsuperseded_by: new\n---\n{}\n\n# Old\n",
            superseded_callout("new.md")
        ),
    );

    assert_clean(&bundle);
}

#[test]
fn a_broken_link_alongside_a_moved_source_candidate_keeps_its_own_error_class_and_exit_code() {
    let bundle = temp_bundle("broken");
    write_moved_source_tree(&bundle, "[b](./b.md)\n[missing](./no-such.md)");

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1), "got:\n{stdout}");
    assert!(
        stdout.contains("broken link"),
        "expected the existing broken-link error class, got:\n{stdout}"
    );
    assert!(stdout.contains("MOVED-SOURCE"), "got:\n{stdout}");
    for line in stdout.lines().filter(|line| line.contains("broken link")) {
        assert!(
            !line.contains("MOVED-SOURCE"),
            "a broken link must never be labelled MOVED-SOURCE, got:\n{line}"
        );
    }

    let _ = fs::remove_dir_all(&bundle);
}
