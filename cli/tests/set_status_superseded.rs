mod common;

use common::{
    living_docs, new_two_adrs_and_supersede, run_new, run_supersede, stdout_of, temp_bundle,
};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

fn set(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "write", "set"])
        .args(args)
        .arg("--plain")
        .output()
        .unwrap()
}

fn set_section(docs: &Path, reference: &str, text: &str) -> Output {
    let mut child = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "write", "set"])
        .args([reference, "section", "Context", "--from", "-", "--plain"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn superseded_adrs(label: &str) -> common::Scratch {
    let docs = temp_bundle("set-status-superseded", label);
    new_two_adrs_and_supersede(&docs);
    docs
}

fn old_adr(docs: &Path) -> PathBuf {
    docs.join("adr/0001-old-decision.md")
}

fn plain_adr(docs: &Path) -> PathBuf {
    docs.join("adr/0001-plain.md")
}

fn snapshot(docs: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    collect(docs, &mut files);
    files.sort();
    files
}

fn collect(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, out);
        } else {
            let bytes = fs::read(&path).unwrap();
            out.push((path, bytes));
        }
    }
}

fn assert_refused(docs: &Path, reference: &str, status: &str, successor: &str) {
    let before = snapshot(docs);
    let out = set(docs, &[reference, "status", status]);
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert_eq!(out.status.code(), Some(2), "stderr: {stderr}");
    assert!(stderr.contains("living-docs write set:"), "{stderr}");
    assert!(stderr.contains("superseded_by"), "{stderr}");
    assert!(stderr.contains(successor), "{stderr}");
    assert!(stderr.contains("by hand"), "{stderr}");
    assert!(snapshot(docs) == before, "a refused call changed a file");
}

fn succeeded(out: &Output) {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn refuses_every_status_on_a_superseded_adr() {
    let docs = superseded_adrs("adr");
    for status in ["Accepted", "Deprecated", "Proposed"] {
        assert_refused(&docs, "adr/0001", status, "0002");
    }
}

#[test]
fn refuses_a_status_when_the_record_was_hand_edited_back_to_accepted() {
    let docs = superseded_adrs("hand-edit");
    let path = old_adr(&docs);
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace("status: Superseded", "status: Accepted"),
    )
    .unwrap();
    assert!(fs::read_to_string(&path)
        .unwrap()
        .contains("status: Accepted"));
    assert_refused(&docs, "adr/0001", "Deprecated", "0002");
}

#[test]
fn refuses_a_status_on_a_superseded_issue() {
    let docs = temp_bundle("set-status-superseded", "issue");
    assert!(run_new(&docs, "issue", "Old").status.success());
    assert!(run_new(&docs, "issue", "New").status.success());
    succeeded(&run_supersede(&docs, "issue/0001", "issue/0002"));
    assert_refused(&docs, "issue/0001", "closed", "0002");
}

#[test]
fn still_sets_a_record_without_a_successor() {
    let docs = temp_bundle("set-status-superseded", "plain");
    assert!(run_new(&docs, "adr", "Plain").status.success());
    let index = docs.join("adr/index.md");
    let before = fs::read(&index).unwrap();
    succeeded(&set(&docs, &["adr/0001", "status", "Accepted"]));
    let text = fs::read_to_string(plain_adr(&docs)).unwrap();
    assert!(text.contains("status: Accepted"), "{text}");
    assert!(fs::read(&index).unwrap() != before, "index row unchanged");
}

#[test]
fn still_sets_a_record_with_an_empty_successor_key() {
    let docs = temp_bundle("set-status-superseded", "empty");
    assert!(run_new(&docs, "adr", "Plain").status.success());
    let path = plain_adr(&docs);
    let text = fs::read_to_string(&path).unwrap();
    let keyed = text.replacen("\nstatus:", "\nsuperseded_by:\nstatus:", 1);
    assert!(keyed != text, "the empty key was not added");
    fs::write(&path, keyed).unwrap();
    succeeded(&set(&docs, &["adr/0001", "status", "Deprecated"]));
}

#[test]
fn still_sets_a_record_whose_successor_is_only_whitespace() {
    let docs = temp_bundle("set-status-superseded", "blank");
    assert!(run_new(&docs, "adr", "Plain").status.success());
    let path = plain_adr(&docs);
    let text = fs::read_to_string(&path).unwrap();
    let keyed = text.replacen("\nstatus:", "\nsuperseded_by: \"  \"\nstatus:", 1);
    assert!(keyed != text, "the blank key was not added");
    fs::write(&path, keyed).unwrap();
    succeeded(&set(&docs, &["adr/0001", "status", "Deprecated"]));
}

#[test]
fn still_sets_the_replacement_that_only_carries_supersedes() {
    let docs = superseded_adrs("replacement");
    let text = fs::read_to_string(docs.join("adr/0002-new-decision.md")).unwrap();
    assert!(text.contains("supersedes: 0001"), "{text}");
    succeeded(&set(&docs, &["adr/0002", "status", "Accepted"]));
}

#[test]
fn still_sets_superseded_with_the_pointer_to_supersede() {
    let docs = temp_bundle("set-status-superseded", "pointer");
    assert!(run_new(&docs, "adr", "Plain").status.success());
    let out = set(&docs, &["adr/0001", "status", "Superseded"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("write supersede"));
}

#[test]
fn other_keys_on_a_superseded_record_are_untouched_by_the_guard() {
    let docs = superseded_adrs("other-keys");
    succeeded(&set_section(&docs, "adr/0001", "Revised context.\n"));
    let text = fs::read_to_string(old_adr(&docs)).unwrap();
    assert!(text.contains("Revised context."), "{text}");
    assert!(text.contains("superseded_by: 0002"), "{text}");
    assert!(
        text.lines().any(|l| l.starts_with("> **SUPERSEDED")),
        "the callout is gone: {text}"
    );
    succeeded(&set(&docs, &["adr/0002", "owner", "x"]));
}

#[test]
fn help_says_a_status_change_is_refused_on_a_superseded_record() {
    let out = living_docs()
        .args(["write", "set", "--help", "--plain"])
        .output()
        .unwrap();
    let help = stdout_of(&out);
    assert!(
        help.contains("refused on a record that carries `superseded_by`"),
        "{help}"
    );
}
