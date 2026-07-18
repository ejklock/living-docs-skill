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
    run_index_with_visibility(docs, doc_type, None)
}

fn run_index_with_visibility(
    docs: &Path,
    doc_type: Option<&str>,
    visibility: Option<&str>,
) -> Output {
    let mut args = vec!["--docs-dir", docs.to_str().unwrap(), "index"];
    if let Some(t) = doc_type {
        args.push(t);
    }
    if let Some(v) = visibility {
        args.push("--visibility");
        args.push(v);
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

fn write_record_with_visibility(
    docs: &Path,
    dir: &str,
    filename: &str,
    title: &str,
    status: &str,
    visibility: &str,
) {
    let type_dir = docs.join(dir);
    fs::create_dir_all(&type_dir).unwrap();
    let contents = format!(
        "---\ntype: ADR\ntitle: {title}\nstatus: {status}\nvisibility: {visibility}\nsupersedes:\nsuperseded_by:\ntags: []\ntimestamp: 2026-07-14T00:00:00Z\n---\n\n# {title}\n\n## Context\n\n<placeholder text>\n"
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
fn index_prd_and_bdr_split_active_above_superseded_like_adr() {
    let docs = temp_dir("prd-bdr-split");
    write_record(&docs, "prd", "0001-draft.md", "Draft Feature", "Draft");
    write_record(
        &docs,
        "prd",
        "0002-done.md",
        "Implemented Feature",
        "Implemented",
    );
    write_record(&docs, "prd", "0003-old.md", "Old Feature", "Superseded");
    write_record(
        &docs,
        "bdr",
        "0001-current.md",
        "Current Behavior",
        "Accepted",
    );
    write_record(
        &docs,
        "bdr",
        "0002-retired.md",
        "Retired Behavior",
        "Deprecated",
    );

    let prd_output = run_index(&docs, Some("prd"));
    assert!(prd_output.status.success());
    let bdr_output = run_index(&docs, Some("bdr"));
    assert!(bdr_output.status.success());

    let prd_contents = fs::read_to_string(docs.join("prd/index.md")).unwrap();
    let active_heading = prd_contents
        .find("## Active")
        .expect("missing ## Active heading");
    let superseded_heading = prd_contents
        .find("## Superseded")
        .expect("missing ## Superseded heading");
    assert!(active_heading < superseded_heading, "got: {prd_contents}");
    assert!(
        prd_contents.contains("* [0001 — Draft Feature](0001-draft.md) - Draft"),
        "got: {prd_contents}"
    );
    assert!(
        prd_contents.contains("* [0002 — Implemented Feature](0002-done.md) - Implemented"),
        "got: {prd_contents}"
    );
    let draft_row = prd_contents.find("0001-draft.md").unwrap();
    let implemented_row = prd_contents.find("0002-done.md").unwrap();
    let old_row = prd_contents.find("0003-old.md").unwrap();
    assert!(draft_row < superseded_heading && implemented_row < superseded_heading);
    assert!(old_row > superseded_heading, "got: {prd_contents}");

    let bdr_contents = fs::read_to_string(docs.join("bdr/index.md")).unwrap();
    let bdr_active_heading = bdr_contents
        .find("## Active")
        .expect("missing ## Active heading");
    let bdr_superseded_heading = bdr_contents
        .find("## Superseded")
        .expect("missing ## Superseded heading");
    let current_row = bdr_contents.find("0001-current.md").unwrap();
    let retired_row = bdr_contents.find("0002-retired.md").unwrap();
    assert!(bdr_active_heading < current_row && current_row < bdr_superseded_heading);
    assert!(retired_row > bdr_superseded_heading, "got: {bdr_contents}");

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn index_issue_splits_open_above_closed_with_case_insensitive_done() {
    let docs = temp_dir("issue-split");
    write_record(&docs, "issues", "0001-brewing.md", "Brewing Issue", "open");
    write_record(
        &docs,
        "issues",
        "0002-finished.md",
        "Finished Issue",
        "done",
    );
    write_record(
        &docs,
        "issues",
        "0003-also-finished.md",
        "Also Finished Issue",
        "Done",
    );

    let output = run_index(&docs, Some("issue"));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = fs::read_to_string(docs.join("issues/index.md")).unwrap();
    let open_heading = contents.find("## Open").expect("missing ## Open heading");
    let closed_heading = contents
        .find("## Closed")
        .expect("missing ## Closed heading");
    assert!(open_heading < closed_heading, "got: {contents}");

    let open_row = contents.find("0001-brewing.md").unwrap();
    let closed_row = contents.find("0002-finished.md").unwrap();
    let closed_row_case = contents.find("0003-also-finished.md").unwrap();
    assert!(
        open_heading < open_row && open_row < closed_heading,
        "got: {contents}"
    );
    assert!(closed_row > closed_heading, "got: {contents}");
    assert!(closed_row_case > closed_heading, "got: {contents}");

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn index_migrates_a_legacy_done_open_issues_index_to_open_closed() {
    let docs = temp_dir("issue-migration");
    let issue_dir = docs.join("issues");
    fs::create_dir_all(&issue_dir).unwrap();
    let legacy_preamble = "# Issues\n\nA hand-written intro paragraph that must survive.\n\n";
    fs::write(
        issue_dir.join("index.md"),
        format!(
            "{legacy_preamble}## Done\n\n* [0002 — Old Row](0002-old-row.md) - closed\n\n## Open\n\n* [0001 — Old Row](0001-old-row.md) - open\n"
        ),
    )
    .unwrap();
    write_record(&docs, "issues", "0001-active.md", "Active Issue", "open");
    write_record(
        &docs,
        "issues",
        "0002-finished.md",
        "Finished Issue",
        "done",
    );

    let output = run_index(&docs, Some("issue"));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = fs::read_to_string(issue_dir.join("index.md")).unwrap();
    assert!(contents.starts_with(legacy_preamble), "got: {contents}");
    assert!(!contents.contains("## Done"), "got: {contents}");
    assert!(!contents.contains("stale"), "got: {contents}");
    assert!(contents.contains("## Open"), "got: {contents}");
    assert!(contents.contains("## Closed"), "got: {contents}");
    assert!(
        contents.contains("* [0001 — Active Issue](0001-active.md) - open"),
        "got: {contents}"
    );
    assert!(
        contents.contains("* [0002 — Finished Issue](0002-finished.md) - done"),
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

#[test]
fn index_with_visibility_filter_lists_only_matching_visibility_records() {
    let docs = temp_dir("visibility-filter");
    write_record_with_visibility(
        &docs,
        "adr",
        "0001-public.md",
        "Public Decision",
        "Accepted",
        "public",
    );
    write_record_with_visibility(
        &docs,
        "adr",
        "0002-private.md",
        "Private Decision",
        "Accepted",
        "private",
    );
    write_record(
        &docs,
        "adr",
        "0003-absent.md",
        "Absent Decision",
        "Accepted",
    );

    let output = run_index_with_visibility(&docs, Some("adr"), Some("public,showcase"));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = fs::read_to_string(docs.join("adr/index.md")).unwrap();
    assert!(contents.contains("0001-public.md"), "got: {contents}");
    assert!(!contents.contains("0002-private.md"), "got: {contents}");
    assert!(!contents.contains("0003-absent.md"), "got: {contents}");

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn index_without_visibility_flag_lists_every_record_regardless_of_visibility() {
    let docs = temp_dir("visibility-unset");
    write_record_with_visibility(
        &docs,
        "adr",
        "0001-public.md",
        "Public Decision",
        "Accepted",
        "public",
    );
    write_record_with_visibility(
        &docs,
        "adr",
        "0002-private.md",
        "Private Decision",
        "Accepted",
        "private",
    );
    write_record(
        &docs,
        "adr",
        "0003-absent.md",
        "Absent Decision",
        "Accepted",
    );

    let output = run_index(&docs, Some("adr"));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = fs::read_to_string(docs.join("adr/index.md")).unwrap();
    assert!(contents.contains("0001-public.md"), "got: {contents}");
    assert!(contents.contains("0002-private.md"), "got: {contents}");
    assert!(contents.contains("0003-absent.md"), "got: {contents}");

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn index_visibility_private_filter_includes_absent_visibility_records() {
    let docs = temp_dir("visibility-default-deny");
    write_record_with_visibility(
        &docs,
        "adr",
        "0001-public.md",
        "Public Decision",
        "Accepted",
        "public",
    );
    write_record(
        &docs,
        "adr",
        "0002-absent.md",
        "Absent Decision",
        "Accepted",
    );

    let private_output = run_index_with_visibility(&docs, Some("adr"), Some("private"));
    assert!(private_output.status.success());
    let private_contents = fs::read_to_string(docs.join("adr/index.md")).unwrap();
    assert!(
        private_contents.contains("0002-absent.md"),
        "got: {private_contents}"
    );
    assert!(
        !private_contents.contains("0001-public.md"),
        "got: {private_contents}"
    );

    let public_output = run_index_with_visibility(&docs, Some("adr"), Some("public"));
    assert!(public_output.status.success());
    let public_contents = fs::read_to_string(docs.join("adr/index.md")).unwrap();
    assert!(
        !public_contents.contains("0002-absent.md"),
        "got: {public_contents}"
    );
    assert!(
        public_contents.contains("0001-public.md"),
        "got: {public_contents}"
    );

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
