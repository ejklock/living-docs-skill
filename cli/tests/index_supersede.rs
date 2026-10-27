use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod common;
use common::{new_two_adrs_and_supersede, Scratch};

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn temp_dir(label: &str) -> Scratch {
    Scratch::new(&format!("living-docs-idx-sup-test-{label}"))
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

fn run_check(docs: &Path) -> Output {
    living_docs()
        .args(["check", docs.to_str().unwrap(), "--plain"])
        .output()
        .expect("failed to run living-docs check")
}

/// `examples/linkly/docs` anchored at compile time via `CARGO_MANIFEST_DIR`,
/// independent of the working directory `cargo test` is invoked from — a
/// real bundle that passes `check` today and, notably, carries no
/// `research/` directory (the type-registry token this round adds).
fn linkly_fixture_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/..")).join("examples/linkly/docs")
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path);
        } else {
            fs::copy(entry.path(), &dst_path).unwrap();
        }
    }
}

fn run_supersede(docs: &Path, old: &str, new: &str) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "supersede", old, new])
        .output()
        .expect("failed to run living-docs supersede")
}

fn run_status(docs: &Path, number: &str, new_status: &str) -> Output {
    living_docs()
        .args([
            "--docs-dir",
            docs.to_str().unwrap(),
            "set",
            number,
            "status",
            new_status,
        ])
        .output()
        .expect("failed to run living-docs set")
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
}

#[test]
fn index_uses_a_minimal_title_preamble_on_a_fresh_file() {
    let docs = temp_dir("fresh-preamble");
    write_record(&docs, "prd", "0001-feature.md", "Feature", "Draft");

    let output = run_index(&docs, Some("prd"));
    assert!(output.status.success());

    let contents = fs::read_to_string(docs.join("prd/index.md")).unwrap();
    assert!(contents.starts_with("# PRDs\n\n"), "got: {contents}");
}

#[test]
#[allow(clippy::too_many_lines)]
fn index_prd_splits_active_above_superseded_like_adr() {
    let docs = temp_dir("prd-split");
    write_record(&docs, "prd", "0001-draft.md", "Draft Feature", "Draft");
    write_record(
        &docs,
        "prd",
        "0002-done.md",
        "Implemented Feature",
        "Implemented",
    );
    write_record(&docs, "prd", "0003-old.md", "Old Feature", "Superseded");

    let prd_output = run_index(&docs, Some("prd"));
    assert!(prd_output.status.success());

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
}

#[test]
#[allow(clippy::too_many_lines)]
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
}

#[test]
#[allow(clippy::too_many_lines)]
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
}

/// The regression this round fixes (ADR 0026 decision point 7): a bundle
/// that passes `check` must still pass `check` after a bare `index` run,
/// for every registry token — not just the ones the bundle happens to use.
/// `regenerate` used to call `fs::create_dir_all` unconditionally, so a bare
/// sweep materialized an empty `index.md` (and its directory) for every
/// registry token regardless of whether the bundle carried that type,
/// producing a directory index unreachable from the bundle root (invariant
/// 3). This is registry-blind by construction — it exercises every token in
/// `all_type_tokens()` through the real bare-`index` sweep against a
/// fixture that only uses a subset of them, so it keeps holding without
/// editing as new types are registered.
#[test]
fn index_without_a_type_leaves_a_check_clean_bundle_check_clean() {
    let docs = temp_dir("linkly-check-clean");
    copy_dir_recursive(&linkly_fixture_dir(), &docs);

    let before = run_check(&docs);
    assert!(
        before.status.success(),
        "fixture must be check-clean before index: stdout: {}",
        String::from_utf8_lossy(&before.stdout)
    );

    let index_output = run_index(&docs, None);
    assert!(
        index_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&index_output.stderr)
    );

    let after = run_check(&docs);
    assert!(
        after.status.success(),
        "bundle regressed from check-clean to failing after a bare index run: stdout: {}",
        String::from_utf8_lossy(&after.stdout)
    );
}

fn write_raw(docs: &Path, dir: &str, filename: &str, contents: &str) {
    let type_dir = docs.join(dir);
    fs::create_dir_all(&type_dir).unwrap();
    fs::write(type_dir.join(filename), contents).unwrap();
}

#[test]
#[allow(clippy::too_many_lines)]
fn index_migrates_a_hand_maintained_table_format_index_into_the_canonical_bullet_listing() {
    let docs = temp_dir("table-migration");
    let adr_dir = docs.join("adr");
    fs::create_dir_all(&adr_dir).unwrap();
    let preamble = "# ADRs\n\nA hand-written intro paragraph that must survive.\n\n";
    fs::write(
        adr_dir.join("index.md"),
        format!(
            "{preamble}| # | Decision | Status |\n|---|---|---|\n| [0001](0001-old.md) | Old Decision | Superseded |\n"
        ),
    )
    .unwrap();
    write_record(&docs, "adr", "0001-old.md", "Old Decision", "Superseded");
    write_record(
        &docs,
        "adr",
        "0002-current.md",
        "Current Decision",
        "Accepted",
    );

    let first = run_index(&docs, Some("adr"));
    assert!(
        first.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let first_contents = fs::read_to_string(adr_dir.join("index.md")).unwrap();
    assert!(
        first_contents.starts_with(preamble),
        "got: {first_contents}"
    );
    assert!(
        !first_contents.contains('|'),
        "table not migrated away: {first_contents}"
    );
    assert_eq!(
        first_contents.matches("0001-old.md").count(),
        1,
        "duplicated row: {first_contents}"
    );
    assert!(
        first_contents.contains("* [0002 — Current Decision](0002-current.md) - Accepted"),
        "got: {first_contents}"
    );
    assert!(
        first_contents.contains("* [0001 — Old Decision](0001-old.md) - Superseded"),
        "got: {first_contents}"
    );

    let second = run_index(&docs, Some("adr"));
    assert!(second.status.success());
    let second_contents = fs::read_to_string(adr_dir.join("index.md")).unwrap();
    assert_eq!(
        first_contents, second_contents,
        "migrated table-format index is not idempotent on a second run"
    );
}

#[test]
fn index_falls_back_to_the_h1_heading_and_warns_when_title_frontmatter_is_absent() {
    let docs = temp_dir("missing-title");
    write_raw(
        &docs,
        "adr",
        "0007-legacy.md",
        "---\ntype: ADR\nstatus: Accepted\nsupersedes:\nsuperseded_by:\ntags: []\n---\n\n# ADR 0007 — Legacy Decision\n\nBody.\n",
    );

    let output = run_index(&docs, Some("adr"));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = fs::read_to_string(docs.join("adr/index.md")).unwrap();
    assert!(
        contents.contains("* [0007 — Legacy Decision](0007-legacy.md) - Accepted"),
        "got: {contents}"
    );
    assert!(
        !contents.contains("[0007 — ]"),
        "title silently blanked: {contents}"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("0007-legacy.md"), "got: {stderr}");
    assert!(stderr.contains("title"), "got: {stderr}");
}

#[test]
fn index_renders_the_full_title_when_frontmatter_title_is_an_unquoted_scalar_with_special_characters(
) {
    let docs = temp_dir("special-char-title");
    write_raw(
        &docs,
        "adr",
        "0007-special.md",
        "---\ntype: ADR\ntitle: Provenance audit: vs Matt Pocock's skills; remove derivative\nstatus: Accepted\nsupersedes:\nsuperseded_by:\ntags: []\n---\n\n# Special\n\nBody.\n",
    );

    let output = run_index(&docs, Some("adr"));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = fs::read_to_string(docs.join("adr/index.md")).unwrap();
    assert!(
        contents.contains(
            "* [0007 — Provenance audit: vs Matt Pocock's skills; remove derivative](0007-special.md) - Accepted"
        ),
        "got: {contents}"
    );
}

fn run_new(docs: &Path, doc_type: &str, title: &str) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "new", doc_type, title])
        .output()
        .expect("failed to run living-docs new")
}

#[test]
#[allow(clippy::too_many_lines)]
fn supersede_wires_status_and_both_links_bidirectionally() {
    let docs = temp_dir("supersede-bidirectional");
    new_two_adrs_and_supersede(&docs);

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
        old_contents.contains("We will {{DECISION}}"),
        "body lost: {old_contents}"
    );
    assert!(
        new_contents.contains("We will {{DECISION}}"),
        "body lost: {new_contents}"
    );
    assert!(
        old_contents.contains("{{REJECTED_ALTERNATIVES:"),
        "hinted slot lost: {old_contents}"
    );
}

#[test]
fn supersede_inserts_a_missing_supersedes_key_into_the_frontmatter_block() {
    let docs = temp_dir("supersede-insert");
    assert!(run_new(&docs, "prd", "Old Behavior").status.success());
    assert!(run_new(&docs, "prd", "New Behavior").status.success());

    let output = run_supersede(&docs, "0001", "0002");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let new_contents = fs::read_to_string(docs.join("prd/0002-new-behavior.md")).unwrap();
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
}

#[test]
fn supersede_fails_when_a_record_number_does_not_exist() {
    let docs = temp_dir("supersede-missing");
    assert!(run_new(&docs, "adr", "Only Decision").status.success());

    let output = run_supersede(&docs, "0001", "0099");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no record found"), "got: {stderr}");
}

#[test]
fn status_sets_the_field_and_preserves_body_and_other_frontmatter() {
    let docs = temp_dir("status-set");
    assert!(run_new(&docs, "adr", "A Decision").status.success());

    let output = run_status(&docs, "0001", "Accepted");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = fs::read_to_string(docs.join("adr/0001-a-decision.md")).unwrap();
    assert!(contents.contains("status: Accepted"), "got: {contents}");
    assert!(
        contents.contains("We will {{DECISION}}"),
        "body lost: {contents}"
    );
    assert!(
        contents.contains("{{REJECTED_ALTERNATIVES:"),
        "hinted slot lost: {contents}"
    );
}

#[test]
fn status_rejects_superseded_and_points_to_the_supersede_command_leaving_the_file_unchanged() {
    let docs = temp_dir("status-reject-superseded");
    assert!(run_new(&docs, "adr", "A Decision").status.success());
    let before = fs::read_to_string(docs.join("adr/0001-a-decision.md")).unwrap();

    let output = run_status(&docs, "0001", "Superseded");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("living-docs supersede"), "got: {stderr}");

    let after = fs::read_to_string(docs.join("adr/0001-a-decision.md")).unwrap();
    assert_eq!(before, after, "file must be left unchanged");
}

#[test]
fn status_rejects_an_unknown_value_leaving_the_file_unchanged() {
    let docs = temp_dir("status-reject-unknown");
    assert!(run_new(&docs, "adr", "A Decision").status.success());
    let before = fs::read_to_string(docs.join("adr/0001-a-decision.md")).unwrap();

    let output = run_status(&docs, "0001", "Acepted");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Proposed"), "got: {stderr}");
    assert!(stderr.contains("Accepted"), "got: {stderr}");
    assert!(stderr.contains("Deprecated"), "got: {stderr}");

    let after = fs::read_to_string(docs.join("adr/0001-a-decision.md")).unwrap();
    assert_eq!(before, after, "file must be left unchanged");
}

#[test]
fn status_setting_the_current_status_again_is_a_byte_identical_no_op() {
    let docs = temp_dir("status-idempotent");
    assert!(run_new(&docs, "adr", "A Decision").status.success());
    let before = fs::read(docs.join("adr/0001-a-decision.md")).unwrap();

    let output = run_status(&docs, "0001", "Proposed");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let after = fs::read(docs.join("adr/0001-a-decision.md")).unwrap();
    assert_eq!(
        before, after,
        "re-setting the same status must be byte-identical"
    );
}
