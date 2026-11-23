//! The record lifecycle seen through the binary: a refused transition exits
//! 2, names the rule and the records involved, and changes no byte of the
//! bundle; a legal one exits 0.

mod common;

use common::{living_docs, run_new, run_supersede, temp_bundle, Scratch};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

type Snapshot = Vec<(PathBuf, Vec<u8>)>;

fn bundle(label: &str, adrs: usize) -> Scratch {
    let docs = temp_bundle("lifecycle-refusals", label);
    for title in ["First", "Second", "Third", "Fourth"].iter().take(adrs) {
        assert!(run_new(&docs, "adr", title).status.success());
    }
    docs
}

fn set_status(docs: &Path, reference: &str, status: &str) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "write", "set"])
        .args([reference, "status", status, "--plain"])
        .output()
        .unwrap()
}

fn snapshot(root: &Path) -> Snapshot {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push((path.clone(), fs::read(&path).unwrap()));
            }
        }
    }
    files.sort();
    files
}

fn succeeded(output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
}

fn assert_refused(docs: &Path, before: &Snapshot, output: &Output, names: &[&str]) {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert_eq!(output.status.code(), Some(2), "stderr: {stderr}");
    for name in names {
        assert!(stderr.contains(name), "stderr names {name}: {stderr}");
    }
    assert!(&snapshot(docs) == before, "a refused call changed a file");
}

#[test]
fn a_record_with_a_successor_is_not_superseded_again() {
    let docs = bundle("re-supersede", 3);
    succeeded(&run_supersede(&docs, "adr/0001", "adr/0002"));
    let before = snapshot(&docs);
    let output = run_supersede(&docs, "adr/0001", "adr/0003");
    assert_refused(
        &docs,
        &before,
        &output,
        &["0001", "0002", "already superseded"],
    );
}

#[test]
fn a_supersede_that_closes_a_cycle_is_refused() {
    let docs = bundle("cycle", 2);
    succeeded(&run_supersede(&docs, "adr/0001", "adr/0002"));
    let before = snapshot(&docs);
    let output = run_supersede(&docs, "adr/0002", "adr/0001");
    assert_refused(&docs, &before, &output, &["0001", "0002", "cycle"]);
}

#[test]
fn a_cycle_through_a_longer_chain_is_refused() {
    let docs = bundle("long-cycle", 3);
    succeeded(&run_supersede(&docs, "adr/0001", "adr/0002"));
    succeeded(&run_supersede(&docs, "adr/0002", "adr/0003"));
    let before = snapshot(&docs);
    let output = run_supersede(&docs, "adr/0003", "adr/0001");
    assert_refused(&docs, &before, &output, &["0001", "0002", "0003", "cycle"]);
}

#[test]
fn a_deprecated_successor_is_refused() {
    let docs = bundle("deprecated-successor", 2);
    succeeded(&set_status(&docs, "adr/0002", "Deprecated"));
    let before = snapshot(&docs);
    let output = run_supersede(&docs, "adr/0001", "adr/0002");
    assert_refused(&docs, &before, &output, &["0001", "0002", "retired"]);
}

#[test]
fn a_superseded_successor_is_refused() {
    let docs = bundle("superseded-successor", 3);
    succeeded(&run_supersede(&docs, "adr/0002", "adr/0003"));
    let before = snapshot(&docs);
    let output = run_supersede(&docs, "adr/0001", "adr/0002");
    assert_refused(&docs, &before, &output, &["0001", "0002", "retired"]);
}

#[test]
fn a_proposed_successor_is_accepted() {
    let docs = bundle("proposed-successor", 2);
    succeeded(&run_supersede(&docs, "adr/0001", "adr/0002"));
}

#[test]
fn every_status_on_a_superseded_record_is_refused() {
    let docs = bundle("status-on-superseded", 2);
    succeeded(&run_supersede(&docs, "adr/0001", "adr/0002"));
    for status in ["Accepted", "Proposed", "Deprecated"] {
        let before = snapshot(&docs);
        let output = set_status(&docs, "adr/0001", status);
        assert_refused(&docs, &before, &output, &["0001", "0002", "superseded"]);
    }
}

#[test]
fn reopening_stays_legal_for_an_adr_and_an_issue() {
    let docs = bundle("reopen", 1);
    succeeded(&set_status(&docs, "adr/0001", "Deprecated"));
    succeeded(&set_status(&docs, "adr/0001", "Accepted"));
    assert!(run_new(&docs, "issue", "Bound the cache").status.success());
    succeeded(&set_status(&docs, "issue/0001", "closed"));
    succeeded(&set_status(&docs, "issue/0001", "open"));
}

#[test]
fn a_record_that_superseded_another_may_be_deprecated() {
    let docs = bundle("deprecate-successor", 2);
    succeeded(&run_supersede(&docs, "adr/0001", "adr/0002"));
    succeeded(&set_status(&docs, "adr/0002", "Deprecated"));
}

fn crlf_bundle(label: &str, adrs: usize) -> Scratch {
    let docs = bundle(label, adrs);
    for (path, bytes) in snapshot(&docs.join("adr")) {
        let text = String::from_utf8(bytes).unwrap();
        fs::write(path, text.replace('\n', "\r\n")).unwrap();
    }
    docs
}

#[test]
fn a_crlf_bundle_refuses_a_re_supersede() {
    let docs = crlf_bundle("crlf-re-supersede", 3);
    succeeded(&run_supersede(&docs, "adr/0001", "adr/0002"));
    let before = snapshot(&docs);
    let output = run_supersede(&docs, "adr/0001", "adr/0003");
    assert_refused(
        &docs,
        &before,
        &output,
        &["0001", "0002", "already superseded"],
    );
}

#[test]
fn a_crlf_bundle_refuses_a_cycle() {
    let docs = crlf_bundle("crlf-cycle", 2);
    succeeded(&run_supersede(&docs, "adr/0001", "adr/0002"));
    let before = snapshot(&docs);
    let output = run_supersede(&docs, "adr/0002", "adr/0001");
    assert_refused(&docs, &before, &output, &["0001", "0002", "cycle"]);
}

#[test]
fn repeating_the_same_supersede_exits_zero_and_changes_no_file() {
    let docs = bundle("identical-repeat", 2);
    succeeded(&run_supersede(&docs, "adr/0001", "adr/0002"));
    let before = snapshot(&docs);
    succeeded(&run_supersede(&docs, "adr/0001", "adr/0002"));
    assert!(
        snapshot(&docs) == before,
        "an identical repeat changed a file"
    );
}
