mod common;

use common::{living_docs, temp_bundle, write};
use serde_json::Value;
use std::fs;
use std::path::Path;

fn record(frontmatter_title: Option<&str>, body: &str) -> String {
    let title = frontmatter_title
        .map(|title| format!("title: {title}\n"))
        .unwrap_or_default();
    format!("---\ntype: ADR\n{title}status: Proposed\n---\n\n{body}")
}

fn index_row_title(bundle: &Path) -> String {
    let output = living_docs()
        .args([
            "--docs-dir",
            bundle.to_str().unwrap(),
            "manage",
            "index",
            "--plain",
        ])
        .output()
        .expect("failed to run index");
    assert!(output.status.success(), "index failed");
    let index = fs::read_to_string(bundle.join("adr/index.md")).unwrap();
    let row = index
        .lines()
        .find(|line| line.starts_with("* [0001"))
        .unwrap();
    let label = row[3..].split("](").next().unwrap();
    label.strip_prefix("0001 — ").unwrap().to_owned()
}

fn read_title(bundle: &Path) -> String {
    let output = living_docs()
        .args([
            "--docs-dir",
            bundle.to_str().unwrap(),
            "read",
            "records",
            "--json",
        ])
        .output()
        .expect("failed to run read");
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["records"][0]["title"].as_str().unwrap().to_owned()
}

fn assert_both_show(label: &str, contents: &str, expected: &str) {
    let bundle = temp_bundle("index-derived-title", label);
    write(&bundle, "adr/0001-the-stem.md", contents);
    assert_eq!(index_row_title(&bundle), expected, "index row");
    assert_eq!(read_title(&bundle), expected, "read");
    let before = fs::read_to_string(bundle.join("adr/index.md")).unwrap();
    index_row_title(&bundle);
    assert_eq!(
        before,
        fs::read_to_string(bundle.join("adr/index.md")).unwrap()
    );
}

#[test]
fn a_heading_only_inside_a_fenced_block_gives_the_filename_stem() {
    let contents = record(None, "```\n# Fake\n```\n");
    assert_both_show("fenced", &contents, "the-stem");
}

#[test]
fn a_record_with_no_heading_gives_the_filename_stem() {
    assert_both_show("no-heading", &record(None, "Just text.\n"), "the-stem");
}

#[test]
fn a_number_dot_heading_loses_its_number() {
    assert_both_show("dot", &record(None, "# 0001. Foo\n"), "Foo");
}

#[test]
fn an_adr_number_dash_heading_loses_its_prefix() {
    let heading = format!("# ADR {} — Foo\n", "0001");
    assert_both_show("adr-dash", &record(None, &heading), "Foo");
}

#[test]
fn a_number_dash_heading_loses_its_number() {
    assert_both_show("dash", &record(None, "# 0001 — Foo\n"), "Foo");
}

#[test]
fn a_frontmatter_title_wins_over_the_heading() {
    let contents = record(Some("Front"), "# 0001. Foo\n");
    assert_both_show("frontmatter", &contents, "Front");
}

#[test]
fn a_body_opening_with_a_rule_keeps_its_heading_in_both() {
    let contents = "---\ntype: ADR\nstatus: Proposed\n---\n---\n# Real\n\ntext\n---\n";
    assert_both_show("rule", contents, "Real");
}

#[test]
fn a_heading_that_is_only_a_dash_number_prefix_gives_the_filename_stem() {
    let heading = format!("# {} — \n", "0001");
    assert_both_show("only-dash", &record(None, &heading), "the-stem");
}

#[test]
fn a_heading_that_is_only_a_dot_number_prefix_gives_the_filename_stem() {
    let heading = format!("# {}. \n", "0001");
    assert_both_show("only-dot", &record(None, &heading), "the-stem");
}

#[test]
fn a_view_index_row_skips_a_heading_inside_a_fenced_block() {
    let bundle = temp_bundle("index-derived-title", "view");
    let view = "---\ntype: Architecture View\n---\n\n```\n# Fake\n```\n\n# Real View\n";
    write(&bundle, "architecture/context.md", view);
    let output = living_docs()
        .args([
            "--docs-dir",
            bundle.to_str().unwrap(),
            "manage",
            "index",
            "--plain",
        ])
        .output()
        .expect("failed to run index");
    assert!(output.status.success(), "index failed");
    let index = fs::read_to_string(bundle.join("architecture/index.md")).unwrap();
    assert!(index.contains("[Real View](context.md)"), "got: {index}");
    assert!(!index.contains("Fake"), "got: {index}");
}
