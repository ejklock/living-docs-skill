use std::fs;
use std::path::{Path, PathBuf};

mod common;
use common::{living_docs, run_check, stdout_of, write};

fn temp_bundle(label: &str) -> PathBuf {
    common::temp_bundle("size", label)
}

fn adr_with_body_lines(body_lines: usize) -> String {
    let headings = ["# Doc", "## Context", "## Decision", "## Consequences"].map(str::to_owned);
    let body = headings
        .into_iter()
        .chain((4..body_lines).map(|i| format!("line {i}")))
        .collect::<Vec<_>>()
        .join("\n");
    format!("---\ntype: ADR\ntitle: Doc\ndescription: A minimal record.\n---\n{body}\n")
}

fn assert_no_size_note(bundle: &Path) {
    let output = run_check(bundle);
    assert_eq!(output.status.code(), Some(0));
    assert!(!stdout_of(&output).contains("SIZE"));
    let _ = fs::remove_dir_all(bundle.parent().unwrap());
}

fn indexed_bundle_with(label: &str, record: &str) -> PathBuf {
    let bundle = temp_bundle(label);
    write(&bundle, "index.md", "# Docs\n\n- [ADRs](/adr/index.md)\n");
    write(
        &bundle,
        "adr/index.md",
        "# ADR Index\n\n- [Doc](/adr/0001-doc.md)\n",
    );
    write(&bundle, "adr/0001-doc.md", record);
    bundle
}

#[test]
fn an_over_target_record_gets_a_size_advisory_naming_both_numbers_and_still_exits_zero() {
    let bundle = indexed_bundle_with("over", &adr_with_body_lines(121));

    let output = run_check(&bundle);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout_of(&output);
    assert!(stdout.contains("SIZE"), "got: {stdout}");
    assert!(stdout.contains("0001-doc.md"), "got: {stdout}");
    assert!(stdout.contains("121 lines"), "got: {stdout}");
    assert!(stdout.contains("120-line"), "got: {stdout}");
    assert!(stdout.contains("aim ~100"), "got: {stdout}");
    assert!(stdout.contains("no invariant violations"), "got: {stdout}");

    let _ = fs::remove_dir_all(bundle.parent().unwrap());
}

#[test]
fn a_record_at_exactly_the_advisory_threshold_gets_no_size_note() {
    let bundle = indexed_bundle_with("at-threshold", &adr_with_body_lines(120));

    assert_no_size_note(&bundle);
}

#[test]
fn research_is_exempt_from_the_size_advisory() {
    let bundle = temp_bundle("research-exempt");
    write(
        &bundle,
        "index.md",
        "# Docs\n\n- [Research](/research/index.md)\n",
    );
    write(
        &bundle,
        "research/index.md",
        "# Research Index\n\n- [Note](/research/0001-note.md)\n",
    );
    let long_body = (0..300)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    write(
        &bundle,
        "research/0001-note.md",
        &format!(
            "---\ntype: Research\ntitle: Note\ndescription: A minimal record.\n---\n{long_body}\n"
        ),
    );

    assert_no_size_note(&bundle);
}

#[test]
fn a_real_violation_still_fails_and_the_size_advisory_still_prints_alongside_it() {
    let bundle = indexed_bundle_with("advisory-plus-violation", &adr_with_body_lines(121));
    write(
        &bundle,
        "adr/0002-orphan.md",
        "---\ntype: ADR\ntitle: Orphan\ndescription: An unreachable record.\n---\n# Orphan\n",
    );

    let output = run_check(&bundle);

    assert_eq!(output.status.code(), Some(1));
    let stdout = stdout_of(&output);
    assert!(stdout.contains("SIZE"), "got: {stdout}");
    assert!(stdout.contains("orphan"), "got: {stdout}");

    let _ = fs::remove_dir_all(bundle.parent().unwrap());
}

#[test]
fn the_size_targets_skill_topic_is_served_from_the_embedded_corpus() {
    let output = living_docs()
        .args(["guide", "size-targets", "--plain"])
        .output()
        .expect("failed to run living-docs guide");

    assert!(output.status.success());
    let stdout = stdout_of(&output);
    assert!(stdout.contains("~100"), "got: {stdout}");
    assert!(stdout.contains("120"), "got: {stdout}");
    assert!(stdout.contains("Advisory, never a gate"), "got: {stdout}");
}
