mod common;

use common::{living_docs, run_new, temp_bundle, write};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

const SCOPE: &str = "### Scope\n\ns\n\n#### Child A\n\nold a\n\n#### Child B\n\nold b\n\n";

fn issue_with_children(label: &str) -> (common::Scratch, PathBuf) {
    issue_with_scope(label, SCOPE)
}

fn issue_with_scope(label: &str, scope: &str) -> (common::Scratch, PathBuf) {
    let docs = temp_bundle("set-children", label);
    assert!(run_new(&docs, "issue", "T").status.success());
    let path = docs.join("issues/0001-t.md");
    let text = fs::read_to_string(&path).unwrap();
    let (head, tail) = text.split_once("### Scope").unwrap();
    let rest = tail.split_once("### Decision").unwrap().1;
    fs::write(&path, format!("{head}{scope}### Decision{rest}")).unwrap();
    (docs, path)
}

fn set_section(docs: &Path, heading: &str, content: &str) -> Output {
    write(docs, "v.txt", content);
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "write", "set"])
        .args(["0001", "section", heading, "--plain", "--from"])
        .arg(docs.join("v.txt"))
        .output()
        .unwrap()
}

fn snapshot(docs: &Path) -> (String, String) {
    (
        fs::read_to_string(docs.join("issues/0001-t.md")).unwrap(),
        fs::read_to_string(docs.join("issues/index.md")).unwrap(),
    )
}

#[test]
fn a_missing_child_exits_two_names_each_child_and_writes_nothing() {
    let (docs, _) = issue_with_children("refuse");
    let before = snapshot(&docs);
    let out = set_section(&docs, "Scope", "new text\n");
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("#### Child A"), "{stderr}");
    assert!(stderr.contains("#### Child B"), "{stderr}");
    assert!(stderr.contains("include"), "{stderr}");
    assert_eq!(snapshot(&docs), before);
}

#[test]
fn every_child_carried_replaces_the_section_with_each_child_once() {
    let (docs, path) = issue_with_children("carried");
    let out = set_section(
        &docs,
        "Scope",
        "new text\n\n#### Child A\n\nnew a\n\n#### child b\n\nnew b\n",
    );
    assert!(out.status.success(), "{out:?}");
    let text = fs::read_to_string(path).unwrap();
    assert_eq!(text.matches("#### Child A").count(), 1, "{text}");
    assert_eq!(text.matches("#### child b").count(), 1, "{text}");
    assert!(!text.contains("old a") && !text.contains("old b"), "{text}");
}

#[test]
fn a_section_without_children_is_replaced() {
    let (docs, path) = issue_with_children("plain");
    let out = set_section(&docs, "Decision", "chosen\n");
    assert!(out.status.success(), "{out:?}");
    let text = fs::read_to_string(path).unwrap();
    assert!(text.contains("### Decision\n\nchosen\n"), "{text}");
}

#[test]
fn an_absent_section_is_appended() {
    let (docs, path) = issue_with_children("append");
    let out = set_section(&docs, "Outcome", "done\n");
    assert!(out.status.success(), "{out:?}");
    let text = fs::read_to_string(path).unwrap();
    assert!(text.ends_with("### Outcome\n\ndone\n"), "{text}");
}

#[test]
fn one_carried_copy_of_two_same_named_children_exits_two_and_names_the_other() {
    let scope =
        "### Scope\n\ns\n\n#### A\n\n##### Example\n\nex-a\n\n#### B\n\n##### Example\n\nex-b\n\n";
    let (docs, _) = issue_with_scope("twins", scope);
    let before = snapshot(&docs);
    let text = "new\n\n#### A\n\n##### Example\n\nz\n\n#### B\n";
    let out = set_section(&docs, "Scope", text);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("##### Example"), "{stderr}");
    assert_eq!(snapshot(&docs), before);
}
