//! `check <record.md>` bundle-root choice: which directory the report names
//! as the bundle for each step of the root rule.

use std::fs;
use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, write};

fn scratch(label: &str) -> common::Scratch {
    let root = common::Scratch::new(&format!("living-docs-check-record-roots-test-{label}"));
    fs::create_dir_all(root.join("docs")).unwrap();
    root
}

/// Runs `living-docs <global args> check --json <record>` from `cwd`.
fn check_json(cwd: &Path, global: &[&str], record: &str) -> Output {
    living_docs()
        .current_dir(cwd)
        .args(global)
        .args(["check", "--json", record])
        .output()
        .unwrap()
}

/// The `bundle` the JSON report names for `record`, `None` when no report
/// came back.
fn bundle_of(cwd: &Path, record: &str) -> Option<String> {
    let output = check_json(cwd, &[], record);
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    report["bundle"].as_str().map(String::from)
}

#[test]
fn a_record_under_a_type_directory_with_no_index_anywhere_exits_two() {
    let root = scratch("no-index");
    write(&root, "research/n.md", "# N\n");

    let output = check_json(&root.join("research"), &[], "n.md");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stdout_of(&output).is_empty(),
        "got:\n{}",
        stdout_of(&output)
    );
}

#[test]
fn an_index_in_the_type_directory_or_its_parent_makes_the_parent_the_bundle() {
    let in_type = scratch("index-in-type");
    write(&in_type, "research/n.md", "# N\n");
    write(&in_type, "research/index.md", "# R\n");
    let in_parent = scratch("index-in-parent");
    write(&in_parent, "research/n.md", "# N\n");
    write(&in_parent, "index.md", "# Root\n");

    assert_eq!(bundle_of(&in_type, "research/n.md").as_deref(), Some("."));
    assert_eq!(bundle_of(&in_parent, "research/n.md").as_deref(), Some("."));
}

#[test]
fn the_nearest_type_directory_wins_over_a_farther_one() {
    let root = scratch("nearest");
    write(&root, "index.md", "# Root\n");
    write(&root, "research/s1/index.md", "# S\n");
    write(&root, "research/s1/adr/x.md", "# X\n");

    let bundle = bundle_of(&root, "research/s1/adr/x.md");

    assert_eq!(bundle.as_deref(), Some("research/s1"));
}

#[test]
fn a_constitution_outside_the_docs_dir_resolves_to_its_own_directory() {
    let root = scratch("singleton");
    write(&root, "docs/constitution.md", "# Constitution\n");

    let output = check_json(&root, &["--docs-dir", "nope"], "docs/constitution.md");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();

    assert_eq!(report["bundle"].as_str(), Some("docs"));
}
