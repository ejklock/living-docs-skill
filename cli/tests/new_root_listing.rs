//! `write new` lists the bundle-root `index.md` among the files it wrote
//! exactly when it created or changed it.

mod common;

use common::{living_docs, temp_bundle, write, Scratch};
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Output;

const MARKED_ROOT_WITH_LINK: &str = "---\nokf_version: \"0.1\"\n---\n\n* [ADRs](adr/)\n";
const UNMARKED_ROOT: &str = "* [ADRs](adr/)\n";

fn bundle(label: &str) -> Scratch {
    let docs = temp_bundle("new-root-listing", label);
    fs::create_dir_all(&docs).unwrap();
    docs
}

fn run_new_with(docs: &Path, flag: &str, doc_type: &str, title: &str) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap()])
        .args(["write", "new", doc_type, title, flag])
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("failed to run living-docs write new")
}

/// The `indexed <path>` lines the text output prints on stderr.
fn indexed_lines(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter_map(|line| {
            line.split_once(": indexed ")
                .map(|(_, path)| path.to_owned())
        })
        .collect()
}

fn lists_root(docs: &Path, output: &Output) -> bool {
    let root = docs.join("index.md").to_str().unwrap().to_owned();
    indexed_lines(output).contains(&root)
}

fn indexed_json(output: &Output) -> Vec<String> {
    let doc: Value = serde_json::from_slice(&output.stdout).expect("stdout is JSON");
    doc["indexed"]
        .as_array()
        .expect("indexed array")
        .iter()
        .map(|entry| entry.as_str().expect("path string").to_owned())
        .collect()
}

#[test]
fn the_first_adr_in_an_empty_docs_directory_lists_the_root_in_text() {
    let docs = bundle("first-text");

    let output = run_new_with(&docs, "--plain", "adr", "Alpha");

    assert!(output.status.success(), "{output:?}");
    assert!(lists_root(&docs, &output), "{output:?}");
}

#[test]
fn the_first_adr_in_an_empty_docs_directory_lists_the_root_in_json() {
    let docs = bundle("first-json");

    let output = run_new_with(&docs, "--json", "adr", "Alpha");

    assert!(output.status.success(), "{output:?}");
    let root = docs.join("index.md").to_str().unwrap().to_owned();
    assert!(indexed_json(&output).contains(&root), "{output:?}");
}

#[test]
fn a_root_without_the_marker_is_listed_when_it_gains_it() {
    let docs = bundle("unmarked");
    write(&docs, "index.md", UNMARKED_ROOT);

    let output = run_new_with(&docs, "--plain", "adr", "Alpha");

    assert!(output.status.success(), "{output:?}");
    assert!(lists_root(&docs, &output), "{output:?}");
}

#[test]
fn a_root_that_needs_no_change_is_not_listed() {
    let docs = bundle("unchanged");
    write(&docs, "index.md", MARKED_ROOT_WITH_LINK);
    assert!(run_new_with(&docs, "--plain", "adr", "Alpha")
        .status
        .success());

    let output = run_new_with(&docs, "--plain", "adr", "Beta");

    assert!(output.status.success(), "{output:?}");
    assert!(!lists_root(&docs, &output), "{output:?}");
    let type_index = docs.join("adr").join("index.md");
    assert_eq!(
        indexed_lines(&output),
        [type_index.to_str().unwrap().to_owned()]
    );
}

#[test]
fn a_constitution_in_an_empty_docs_directory_lists_the_linkless_root() {
    let docs = bundle("constitution");

    let output = run_new_with(&docs, "--plain", "constitution", "Rules");

    assert!(output.status.success(), "{output:?}");
    assert!(lists_root(&docs, &output), "{output:?}");
    assert!(docs.join("index.md").is_file());
}

#[test]
fn a_root_write_that_fails_is_a_failure_and_not_listed_as_written() {
    let docs = bundle("readonly");
    fs::create_dir_all(docs.join("adr")).unwrap();
    fs::set_permissions(&docs, fs::Permissions::from_mode(0o555)).unwrap();

    let output = run_new_with(&docs, "--plain", "adr", "Alpha");

    fs::set_permissions(&docs, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(!lists_root(&docs, &output), "{output:?}");
    assert!(!docs.join("index.md").exists());
}
