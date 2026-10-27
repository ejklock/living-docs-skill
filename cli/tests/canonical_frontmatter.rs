//! ADR 0048 (issue 0041 defect #2): every CLI frontmatter-mutation verb
//! (`supersede`, `status`, `describe`, `owner`) routes through the canonical
//! serializer, so a record it touches — whether it inserts a
//! previously-absent key or edits an existing one — stays a canonical
//! round-trip fixed point. A dedicated file, mirroring the existing
//! per-verb split (`describe.rs`, `owner.rs`), keeps this coverage out of
//! `check_core.rs`/`index_supersede.rs`, both already at their file-size
//! ratchet baseline.

use std::fs;
use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, new_two_adrs_and_supersede, run_check, run_new, run_supersede};

fn temp_dir(label: &str) -> common::Scratch {
    common::temp_bundle("canonical-frontmatter", label)
}

fn run_set(docs: &Path, reference: &str, key: &str, value: &str) -> Output {
    living_docs()
        .args([
            "--docs-dir",
            docs.to_str().unwrap(),
            "set",
            reference,
            key,
            value,
        ])
        .output()
        .expect("failed to run living-docs set")
}

fn run_status(docs: &Path, reference: &str, new_status: &str) -> Output {
    run_set(docs, reference, "status", new_status)
}

fn run_describe(docs: &Path, reference: &str, description: &str) -> Output {
    run_set(docs, reference, "description", description)
}

fn run_owner_verb(docs: &Path, reference: &str, value: &str) -> Output {
    run_set(docs, reference, "owner", value)
}

fn body_of(path: &Path) -> String {
    let contents = fs::read_to_string(path).unwrap();
    let after_open = contents.strip_prefix("---\n").unwrap();
    let close = after_open.find("\n---\n\n").unwrap();
    after_open[close + "\n---\n\n".len()..].to_string()
}

/// The exact retired-record callout line a Superseded record must open
/// with, sourced from the same module `living-docs fmt` writes through.
fn superseded_callout(successor: &str) -> String {
    living_docs_core::callout::expected(Some("superseded"), Some(successor))
        .expect("a superseded status always yields a callout")
}

/// AC1: `supersede` inserting the previously-absent `supersedes`/
/// `superseded_by` keys leaves both records a canonical round-trip fixed
/// point — `check` reports no non-canonical-frontmatter finding.
#[test]
fn supersede_leaves_both_records_with_no_non_canonical_finding() {
    let docs = temp_dir("supersede-check-clean");
    new_two_adrs_and_supersede(&docs);

    let check = run_check(&docs);
    let stdout = String::from_utf8_lossy(&check.stdout);
    assert!(
        !stdout.contains("living-docs fmt"),
        "supersede left a non-canonical-frontmatter finding, stdout: {stdout}"
    );
}

/// AC2: `status`, `describe`, and `owner` each editing an already-present
/// key also leave the record canonical, with no `fmt` pass required.
#[test]
fn status_describe_and_owner_each_leave_the_record_with_no_non_canonical_finding() {
    let docs = temp_dir("mutation-check-clean");
    assert!(run_new(&docs, "adr", "A Decision").status.success());

    assert!(run_status(&docs, "0001", "Accepted").status.success());
    assert!(run_describe(&docs, "0001", "A refreshed decision.")
        .status
        .success());
    assert!(run_owner_verb(&docs, "0001", "alice").status.success());

    let check = run_check(&docs);
    let stdout = String::from_utf8_lossy(&check.stdout);
    assert!(
        !stdout.contains("living-docs fmt"),
        "a mutation verb left a non-canonical-frontmatter finding, stdout: {stdout}"
    );
}

/// For `status`, `describe`, and `owner`, the body below the frontmatter
/// stays byte-identical before and after the verb runs. For `supersede`,
/// the old record's body gains its retired-record callout above the
/// previously untouched body, while the new record's body stays
/// byte-identical.
#[test]
fn mutation_verbs_leave_the_body_below_the_frontmatter_byte_identical() {
    let docs = temp_dir("mutation-body-preserved");
    assert!(run_new(&docs, "adr", "Old Decision").status.success());
    assert!(run_new(&docs, "adr", "New Decision").status.success());
    let old_path = docs.join("adr/0001-old-decision.md");
    let new_path = docs.join("adr/0002-new-decision.md");
    let old_body_before = body_of(&old_path);
    let new_body_before = body_of(&new_path);

    assert!(run_supersede(&docs, "0001", "0002").status.success());
    assert!(run_status(&docs, "0002", "Accepted").status.success());
    assert!(run_describe(&docs, "0002", "A refreshed decision.")
        .status
        .success());
    assert!(run_owner_verb(&docs, "0002", "alice").status.success());

    let expected_old_body = format!(
        "{}\n\n{old_body_before}",
        superseded_callout("0002-new-decision.md")
    );
    assert_eq!(
        body_of(&old_path),
        expected_old_body,
        "supersede must prepend the retired-record callout to the old body"
    );
    assert_eq!(
        body_of(&new_path),
        new_body_before,
        "status/describe/owner must not touch the body"
    );
}
