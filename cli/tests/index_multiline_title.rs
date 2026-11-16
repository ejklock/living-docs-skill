mod common;

use common::{living_docs, run_check, stdout_of, temp_bundle};
use std::fs;
use std::path::{Path, PathBuf};

fn run_index(docs: &Path) {
    let output = living_docs()
        .args([
            "--docs-dir",
            docs.to_str().unwrap(),
            "manage",
            "index",
            "--plain",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn multiline_titled_issue(docs: &Path) -> PathBuf {
    let body = docs.parent().unwrap().join("body.txt");
    fs::write(
        &body,
        "### Scope\n\nOne thing.\n\n### Acceptance\n\n- It works.\n",
    )
    .unwrap();
    let created = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap()])
        .args(["write", "new", "issue", "one", "--plain", "--body"])
        .arg(&body)
        .output()
        .unwrap();
    assert!(created.status.success(), "{}", stdout_of(&created));
    let issues = docs.join("issues");
    let record = fs::read_dir(&issues)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("0001"))
        })
        .unwrap();
    let text = fs::read_to_string(&record).unwrap();
    let edited = text.replacen("title: one", "title: \"one\\ntwo\"", 1);
    assert_ne!(edited, text);
    fs::write(&record, edited).unwrap();
    record
}

#[test]
fn a_record_title_holding_a_line_break_gets_one_single_line_bullet() {
    let docs = temp_bundle("index-multiline-title", "bullet");
    fs::write(docs.join("index.md"), "# Docs\n\n* [Issues](issues/)\n").unwrap();
    let record = multiline_titled_issue(&docs);
    let issues = docs.join("issues");
    let filename = record.file_name().unwrap().to_string_lossy().into_owned();

    run_index(&docs);
    let first = fs::read_to_string(issues.join("index.md")).unwrap();
    let rows: Vec<&str> = first.lines().filter(|l| l.starts_with("* [")).collect();
    assert_eq!(rows.len(), 1, "{first}");
    assert!(rows[0].starts_with("* [0001 — one two]("), "{first}");
    assert!(rows[0].contains(&format!("]({filename})")), "{first}");

    run_index(&docs);
    assert_eq!(fs::read_to_string(issues.join("index.md")).unwrap(), first);
    let check = run_check(&docs);
    assert_eq!(check.status.code(), Some(0), "{}", stdout_of(&check));
}
