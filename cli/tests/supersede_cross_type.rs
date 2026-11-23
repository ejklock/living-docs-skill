use std::fs;
use std::path::Path;

mod common;
use common::{run_new, run_supersede, Scratch};

fn bundle(label: &str) -> Scratch {
    let docs = Scratch::new(&format!("living-docs-sup-cross-{label}"));
    for title in ["First Decision", "Second Decision", "Third Decision"] {
        assert!(run_new(&docs, "adr", title).status.success());
    }
    assert!(run_new(&docs, "issue", "First Issue").status.success());
    assert!(run_new(&docs, "research", "First Research")
        .status
        .success());
    docs
}

fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push((path.display().to_string(), fs::read(&path).unwrap()));
            }
        }
    }
    files.sort();
    files
}

fn assert_refused(label: &str, old: &str, new: &str, names: [&str; 2]) {
    let docs = bundle(label);
    let before = snapshot(&docs);
    let output = run_supersede(&docs, old, new);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert_eq!(output.status.code(), Some(2), "stderr: {stderr}");
    for name in names {
        assert!(stderr.contains(name), "stderr names {name}: {stderr}");
    }
    assert_message_shape(&stderr, old, new);
    assert_eq!(
        snapshot(&docs),
        before,
        "a refused supersede writes nothing"
    );
}

fn assert_message_shape(stderr: &str, old: &str, new: &str) {
    for record in [old, new] {
        assert!(stderr.contains(record), "stderr names {record}: {stderr}");
    }
    assert!(
        stderr.contains("supersede links records of one type"),
        "{stderr}"
    );
    assert_eq!(
        stderr.trim_end().lines().count(),
        1,
        "the refusal is one line: {stderr}"
    );
    for article in [" a ADR", " a Issue", " a Research", " a PRD"] {
        assert!(!stderr.contains(article), "no article {article}: {stderr}");
    }
}

fn adr_text(docs: &Path, number: &str) -> String {
    let name = fs::read_dir(docs.join("adr"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .find(|name| name.starts_with(number))
        .unwrap();
    fs::read_to_string(docs.join("adr").join(name)).unwrap()
}

#[test]
fn an_adr_superseded_by_an_issue_is_refused_and_writes_nothing() {
    assert_refused("adr-issue", "adr/0003", "issues/0001", ["ADR", "Issue"]);
}

#[test]
fn an_issue_superseded_by_an_adr_is_refused_and_writes_nothing() {
    assert_refused("issue-adr", "issues/0001", "adr/0003", ["Issue", "ADR"]);
}

#[test]
fn research_superseded_by_an_adr_is_refused_and_writes_nothing() {
    assert_refused(
        "research-adr",
        "research/0001",
        "adr/0001",
        ["Research", "ADR"],
    );
}

#[test]
fn two_adrs_still_link_with_callout_links_and_index() {
    let docs = bundle("two-adrs");
    let output = run_supersede(&docs, "adr/0001", "adr/0002");
    assert!(output.status.success(), "{output:?}");
    let old = adr_text(&docs, "0001");
    let new = adr_text(&docs, "0002");
    assert!(old.contains("status: Superseded"));
    assert!(old.contains("superseded_by: 0002"));
    assert!(old.contains("SUPERSEDED — do not act"));
    assert!(new.contains("supersedes: 0001"));
    let index = fs::read_to_string(docs.join("adr/index.md")).unwrap();
    assert!(index.contains("0001"), "{index}");
}

#[test]
fn a_record_superseding_itself_keeps_the_existing_message() {
    let docs = bundle("self");
    let output = run_supersede(&docs, "adr/0001", "adr/0001");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(!output.status.success());
    assert!(
        stderr.contains("a record cannot supersede itself"),
        "{stderr}"
    );
}

#[test]
fn the_directory_decides_the_type_of_a_record_whose_frontmatter_disagrees() {
    let docs = bundle("dir-decides");
    let name = fs::read_dir(docs.join("adr"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .find(|name| name.starts_with("0003"))
        .unwrap();
    let path = docs.join("adr").join(name);
    let edited = fs::read_to_string(&path)
        .unwrap()
        .replacen("type: ADR", "type: Issue", 1);
    fs::write(&path, edited).unwrap();
    let output = run_supersede(&docs, "adr/0003", "adr/0002");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        !stderr.contains("one type"),
        "refused as cross-type: {stderr}"
    );
    assert!(output.status.success(), "{stderr}");
}
