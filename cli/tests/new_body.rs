mod common;

use common::{living_docs, run_check, stdout_of, temp_bundle};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Output, Stdio};

const BODY: &str = "### Scope\n\nOne thing.\n\n### Acceptance\n\n- It works.\n";
const BODY_TAIL: &str = "## 0001. T\n\n### Scope\n\nOne thing.\n\n### Acceptance\n\n- It works.\n";

fn new_issue(docs: &Path, extra: &[&str], stdin: &str) -> Output {
    let mut child = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "new", "issue", "T"])
        .args(extra)
        .arg("--plain")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut pipe = child.stdin.take().unwrap();
    pipe.write_all(stdin.as_bytes()).unwrap();
    drop(pipe);
    child.wait_with_output().unwrap()
}

fn new_with_body_file(docs: &Path, content: &str, extra: &[&str]) -> Output {
    let file = docs.parent().unwrap().join("body.md");
    fs::write(&file, content).unwrap();
    let mut args = extra.to_vec();
    args.extend(["--body", file.to_str().unwrap()]);
    new_issue(docs, &args, "")
}

fn index_bundle(docs: &Path) {
    let args = ["--docs-dir", docs.to_str().unwrap(), "index"];
    assert!(living_docs().args(args).output().unwrap().status.success());
    let root = "# Docs\n\n- [Issues](/issues/index.md)\n";
    fs::write(docs.join("index.md"), root).unwrap();
}

fn record(docs: &Path) -> String {
    fs::read_to_string(docs.join("issues/0001-t.md")).unwrap()
}

fn frontmatter_without_timestamp(text: &str) -> String {
    text.split("\n---\n")
        .next()
        .unwrap()
        .lines()
        .filter(|line| !line.starts_with("timestamp:"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_refused_without_record(docs: &Path, output: &Output) {
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(!docs.join("issues/0001-t.md").exists());
}

#[test]
fn body_file_replaces_everything_below_the_heading() {
    let docs = temp_bundle("new-body", "file");
    let out = new_with_body_file(&docs, BODY, &[]);
    assert!(out.status.success(), "{out:?}");
    let text = record(&docs);
    assert!(text.ends_with(BODY_TAIL), "{text}");
    assert!(!text.contains("{{"), "{text}");
}

#[test]
fn body_leaves_the_frontmatter_as_without_it() {
    let docs = temp_bundle("new-body", "frontmatter");
    let plain = temp_bundle("new-body", "frontmatter-plain");
    assert!(new_issue(&plain, &[], "").status.success());
    new_with_body_file(&docs, BODY, &[]);
    assert_eq!(
        frontmatter_without_timestamp(&record(&docs)),
        frontmatter_without_timestamp(&record(&plain))
    );
}

#[test]
fn body_with_required_sections_passes_check() {
    let docs = temp_bundle("new-body", "check-ok");
    new_with_body_file(&docs, BODY, &["--description", "d"]);
    index_bundle(&docs);
    let checked = run_check(&docs);
    assert!(checked.status.success(), "{}", stdout_of(&checked));
}

#[test]
fn body_missing_a_required_section_fails_check_naming_it() {
    let docs = temp_bundle("new-body", "check-bad");
    new_with_body_file(&docs, "### Scope\n\nOnly scope.\n", &["--description", "d"]);
    index_bundle(&docs);
    let checked = run_check(&docs);
    assert!(!checked.status.success());
    assert!(
        stdout_of(&checked).contains("Acceptance"),
        "{}",
        stdout_of(&checked)
    );
}

#[test]
fn body_from_stdin_matches_the_file_form() {
    let docs = temp_bundle("new-body", "stdin");
    let out = new_issue(&docs, &["--body", "-"], BODY);
    assert!(out.status.success(), "{out:?}");
    assert!(record(&docs).ends_with(BODY_TAIL));
}

#[test]
fn body_normalizes_line_endings_and_ends_with_one_newline() {
    let docs = temp_bundle("new-body", "crlf");
    new_with_body_file(&docs, "### Scope\r\n\r\nx\r\n\r\n\r\n", &[]);
    assert!(record(&docs).ends_with("### Scope\n\nx\n"));
}

#[test]
fn body_opening_with_a_frontmatter_fence_is_refused() {
    let docs = temp_bundle("new-body", "fence");
    let out = new_with_body_file(&docs, "---\ntype: Issue\n---\nx\n", &[]);
    assert_refused_without_record(&docs, &out);
}

#[test]
fn empty_body_is_refused() {
    let docs = temp_bundle("new-body", "empty");
    let out = new_with_body_file(&docs, " \n\n", &[]);
    assert_refused_without_record(&docs, &out);
}

#[test]
fn missing_body_file_is_refused() {
    let docs = temp_bundle("new-body", "missing");
    let missing = docs.join("nope.md");
    let out = new_issue(&docs, &["--body", missing.to_str().unwrap()], "");
    assert_refused_without_record(&docs, &out);
}
