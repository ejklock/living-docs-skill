use std::fs;
use std::path::Path;

mod common;
use common::{living_docs, write};

fn temp_bundle(label: &str) -> common::Scratch {
    common::temp_bundle("retired-rows", label)
}

fn record(title: &str, status: &str, superseded_by: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: {title}\nstatus: {status}\nsupersedes:\nsuperseded_by: {superseded_by}\ntags: []\ntimestamp: 2026-07-14T00:00:00Z\n---\n\n# {title}\n\n## Context\n\n<placeholder text>\n"
    )
}

fn run_index(docs: &Path, doc_type: &str) -> std::process::Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "index", doc_type])
        .output()
        .expect("failed to run living-docs index")
}

/// Scaffolds one resolvable-successor, one unresolvable-successor, one
/// Deprecated, and one Accepted (non-retired) ADR record, runs `index`
/// over the bundle, and returns the regenerated `adr/index.md` contents.
fn indexed_adr_index_contents(label: &str) -> String {
    let docs = temp_bundle(label);
    write(
        &docs,
        "adr/0001-old.md",
        &record("Old Decision", "Superseded", "0002"),
    );
    write(
        &docs,
        "adr/0002-current.md",
        &record("Current Decision", "Accepted", ""),
    );
    write(
        &docs,
        "adr/0003-orphan.md",
        &record("Orphan Decision", "Superseded", "0009"),
    );
    write(
        &docs,
        "adr/0004-legacy.md",
        &record("Legacy Decision", "Deprecated", ""),
    );

    let output = run_index(&docs, "adr");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = fs::read_to_string(docs.join("adr/index.md")).unwrap();
    let _ = fs::remove_dir_all(docs.parent().unwrap());
    contents
}

#[test]
fn resolvable_successor_row_names_the_successor_file() {
    let contents = indexed_adr_index_contents("resolvable");
    assert!(
        contents.contains(
            "* [0001 — Old Decision](0001-old.md) - Superseded by [0002](0002-current.md)"
        ),
        "got: {contents}"
    );
}

#[test]
fn unresolvable_successor_row_names_the_bare_number() {
    let contents = indexed_adr_index_contents("unresolvable");
    assert!(
        contents.contains("* [0003 — Orphan Decision](0003-orphan.md) - Superseded by 0009"),
        "got: {contents}"
    );
}

#[test]
fn deprecated_row_has_no_successor() {
    let contents = indexed_adr_index_contents("deprecated");
    assert!(
        contents.contains("* [0004 — Legacy Decision](0004-legacy.md) - Deprecated (no successor)"),
        "got: {contents}"
    );
}

#[test]
fn history_note_sits_between_the_superseded_heading_and_the_first_retired_row() {
    let contents = indexed_adr_index_contents("history-note");
    let note = "_History only. Do not act on these records — run `living-docs read` for what is in force._";
    let note_offset = contents.find(note).expect("missing history note");
    let superseded_heading = contents.find("## Superseded").unwrap();
    let first_retired_row = contents.find("0001-old.md").unwrap();
    assert!(
        superseded_heading < note_offset && note_offset < first_retired_row,
        "got: {contents}"
    );
}

#[test]
fn open_closed_axis_never_carries_the_retired_section_note() {
    let docs = temp_bundle("open-closed");
    write(
        &docs,
        "issues/0001-open.md",
        &record("Open Issue", "open", ""),
    );
    write(
        &docs,
        "issues/0002-closed.md",
        &record("Closed Issue", "closed", ""),
    );

    let output = run_index(&docs, "issue");
    assert!(output.status.success());

    let contents = fs::read_to_string(docs.join("issues/index.md")).unwrap();
    assert!(
        !contents.contains("History only"),
        "the issue Open/Closed axis must never carry the retired-section note: {contents}"
    );

    let _ = fs::remove_dir_all(docs.parent().unwrap());
}
