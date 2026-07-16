use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("living-docs-idx-sup-test-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_index(docs: &Path, doc_type: Option<&str>) -> Output {
    let mut args = vec!["--docs-dir", docs.to_str().unwrap(), "index"];
    if let Some(t) = doc_type {
        args.push(t);
    }
    living_docs()
        .args(args)
        .output()
        .expect("failed to run living-docs index")
}

fn run_supersede(docs: &Path, old: &str, new: &str) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "supersede", old, new])
        .output()
        .expect("failed to run living-docs supersede")
}

fn write_record(docs: &Path, dir: &str, filename: &str, title: &str, status: &str) {
    let type_dir = docs.join(dir);
    fs::create_dir_all(&type_dir).unwrap();
    let contents = format!(
        "---\ntype: ADR\ntitle: {title}\nstatus: {status}\nsupersedes:\nsuperseded_by:\ntags: []\ntimestamp: 2026-07-14T00:00:00Z\n---\n\n# {title}\n\n## Context\n\n<placeholder text>\n"
    );
    fs::write(type_dir.join(filename), contents).unwrap();
}

#[test]
fn index_produces_sorted_rows_in_the_locked_format() {
    let docs = temp_dir("sorted-rows");
    write_record(
        &docs,
        "adr",
        "0002-second.md",
        "Second Decision",
        "Proposed",
    );
    write_record(&docs, "adr", "0001-first.md", "First Decision", "Accepted");

    let output = run_index(&docs, Some("adr"));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = fs::read_to_string(docs.join("adr/index.md")).unwrap();
    let first_row = "* [0001 — First Decision](0001-first.md) - Accepted";
    let second_row = "* [0002 — Second Decision](0002-second.md) - Proposed";
    assert!(contents.contains(first_row), "got: {contents}");
    assert!(contents.contains(second_row), "got: {contents}");
    assert!(
        contents.find(first_row).unwrap() < contents.find(second_row).unwrap(),
        "rows are not sorted ascending by NNNN: {contents}"
    );

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn index_splits_adr_active_above_superseded_by_status() {
    let docs = temp_dir("adr-split");
    write_record(&docs, "adr", "0001-old.md", "Old Decision", "Superseded");
    write_record(
        &docs,
        "adr",
        "0002-current.md",
        "Current Decision",
        "Accepted",
    );

    let output = run_index(&docs, Some("adr"));
    assert!(output.status.success());

    let contents = fs::read_to_string(docs.join("adr/index.md")).unwrap();
    let active_heading = contents
        .find("## Active")
        .expect("missing ## Active heading");
    let superseded_heading = contents
        .find("## Superseded")
        .expect("missing ## Superseded heading");
    let active_row = contents.find("0002-current.md").unwrap();
    let superseded_row = contents.find("0001-old.md").unwrap();

    assert!(
        active_heading < superseded_heading,
        "Active must come before Superseded"
    );
    assert!(
        active_heading < active_row && active_row < superseded_heading,
        "got: {contents}"
    );
    assert!(superseded_row > superseded_heading, "got: {contents}");

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn index_preserves_the_preamble_above_the_first_managed_heading_verbatim() {
    let docs = temp_dir("preamble");
    let adr_dir = docs.join("adr");
    fs::create_dir_all(&adr_dir).unwrap();
    let custom_preamble = "# ADRs\n\nA hand-written intro paragraph that must survive.\n\n";
    fs::write(
        adr_dir.join("index.md"),
        format!("{custom_preamble}## Active\n\nstale content\n"),
    )
    .unwrap();
    write_record(&docs, "adr", "0001-first.md", "First Decision", "Proposed");

    let output = run_index(&docs, Some("adr"));
    assert!(output.status.success());

    let contents = fs::read_to_string(adr_dir.join("index.md")).unwrap();
    assert!(contents.starts_with(custom_preamble), "got: {contents}");
    assert!(!contents.contains("stale content"), "got: {contents}");
    assert!(
        contents.contains("* [0001 — First Decision](0001-first.md) - Proposed"),
        "got: {contents}"
    );

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn index_uses_a_minimal_title_preamble_on_a_fresh_file() {
    let docs = temp_dir("fresh-preamble");
    write_record(&docs, "prd", "0001-feature.md", "Feature", "Draft");

    let output = run_index(&docs, Some("prd"));
    assert!(output.status.success());

    let contents = fs::read_to_string(docs.join("prd/index.md")).unwrap();
    assert!(contents.starts_with("# PRDs\n\n"), "got: {contents}");

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn index_non_adr_type_is_a_single_listing_without_a_status_split() {
    let docs = temp_dir("non-adr-single");
    write_record(&docs, "prd", "0001-draft.md", "Draft Feature", "Draft");
    write_record(
        &docs,
        "prd",
        "0002-done.md",
        "Implemented Feature",
        "Implemented",
    );

    let output = run_index(&docs, Some("prd"));
    assert!(output.status.success());

    let contents = fs::read_to_string(docs.join("prd/index.md")).unwrap();
    assert!(!contents.contains("## Active"), "got: {contents}");
    assert!(!contents.contains("## Superseded"), "got: {contents}");
    assert!(
        contents.contains("* [0001 — Draft Feature](0001-draft.md) - Draft"),
        "got: {contents}"
    );
    assert!(
        contents.contains("* [0002 — Implemented Feature](0002-done.md) - Implemented"),
        "got: {contents}"
    );

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn index_is_byte_identical_on_a_second_run() {
    let docs = temp_dir("idempotent");
    write_record(&docs, "adr", "0001-old.md", "Old Decision", "Superseded");
    write_record(
        &docs,
        "adr",
        "0002-current.md",
        "Current Decision",
        "Accepted",
    );

    let first = run_index(&docs, Some("adr"));
    assert!(first.status.success());
    let first_bytes = fs::read(docs.join("adr/index.md")).unwrap();

    let second = run_index(&docs, Some("adr"));
    assert!(second.status.success());
    let second_bytes = fs::read(docs.join("adr/index.md")).unwrap();

    assert_eq!(
        first_bytes, second_bytes,
        "index generation is not idempotent"
    );

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn index_without_a_type_regenerates_every_supported_type_present() {
    let docs = temp_dir("all-types");
    write_record(&docs, "adr", "0001-a.md", "A Decision", "Proposed");
    write_record(&docs, "prd", "0001-p.md", "A Feature", "Draft");

    let output = run_index(&docs, None);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(docs.join("adr/index.md").exists());
    assert!(docs.join("prd/index.md").exists());

    let _ = fs::remove_dir_all(&docs);
}

fn run_new(docs: &Path, doc_type: &str, title: &str) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "new", doc_type, title])
        .output()
        .expect("failed to run living-docs new")
}

#[test]
fn supersede_wires_status_and_both_links_bidirectionally() {
    let docs = temp_dir("supersede-bidirectional");
    assert!(run_new(&docs, "adr", "Old Decision").status.success());
    assert!(run_new(&docs, "adr", "New Decision").status.success());

    let output = run_supersede(&docs, "0001", "0002");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let old_contents = fs::read_to_string(docs.join("adr/0001-old-decision.md")).unwrap();
    let new_contents = fs::read_to_string(docs.join("adr/0002-new-decision.md")).unwrap();

    assert!(
        old_contents.contains("status: Superseded"),
        "got: {old_contents}"
    );
    assert!(
        old_contents.contains("superseded_by: 0002"),
        "got: {old_contents}"
    );
    assert!(
        new_contents.contains("supersedes: 0001"),
        "got: {new_contents}"
    );

    assert!(
        old_contents.contains("We will <the choice, in active voice"),
        "body lost: {old_contents}"
    );
    assert!(
        new_contents.contains("We will <the choice, in active voice"),
        "body lost: {new_contents}"
    );
    assert!(
        old_contents.contains("# Proposed | Accepted | Superseded | Deprecated"),
        "comment lost: {old_contents}"
    );

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn supersede_inserts_a_missing_supersedes_key_into_the_frontmatter_block() {
    let docs = temp_dir("supersede-insert");
    assert!(run_new(&docs, "bdr", "Old Behavior").status.success());
    assert!(run_new(&docs, "bdr", "New Behavior").status.success());

    let output = run_supersede(&docs, "0001", "0002");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let new_contents = fs::read_to_string(docs.join("bdr/0002-new-behavior.md")).unwrap();
    assert!(
        new_contents.contains("supersedes: 0001"),
        "got: {new_contents}"
    );
    let fence_lines = new_contents.lines().filter(|line| *line == "---").count();
    assert_eq!(fence_lines, 2, "frontmatter fence broken: {new_contents}");
    assert_eq!(
        new_contents.matches("supersedes:").count(),
        1,
        "key inserted more than once: {new_contents}"
    );

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn supersede_fails_when_a_record_number_does_not_exist() {
    let docs = temp_dir("supersede-missing");
    assert!(run_new(&docs, "adr", "Only Decision").status.success());

    let output = run_supersede(&docs, "0001", "0099");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no record found"), "got: {stderr}");

    let _ = fs::remove_dir_all(&docs);
}
