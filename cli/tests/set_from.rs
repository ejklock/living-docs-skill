mod common;

use common::{living_docs, run_new, temp_bundle, write};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

fn set(docs: &Path, args: &[&str], stdin: &str) -> Output {
    let mut child = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "set"])
        .args(args)
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

fn issue_bundle(label: &str) -> (common::Scratch, PathBuf) {
    let docs = temp_bundle("set-from", label);
    assert!(run_new(&docs, "issue", "T").status.success());
    let path = docs.join("issues/0001-t.md");
    (docs, path)
}

fn field(path: &Path, key: &str) -> String {
    let text = fs::read_to_string(path).unwrap();
    let prefix = format!("{key}: ");
    text.lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap_or("")
        .to_string()
}

fn value_file(docs: &Path, content: &str) -> String {
    write(docs, "v.txt", content);
    docs.join("v.txt").to_str().unwrap().to_string()
}

fn set_from_file(docs: &Path, args: &[&str], content: &str) -> Output {
    let file = value_file(docs, content);
    let mut full = args.to_vec();
    full.extend(["--from", file.as_str()]);
    set(docs, &full, "")
}

#[test]
fn from_value_description_matches_the_argv_form() {
    let (docs, path) = issue_bundle("from-desc");
    assert!(
        set_from_file(&docs, &["0001", "description"], "A fine sentence.\n")
            .status
            .success()
    );
    let via_file = field(&path, "description");
    assert!(set(&docs, &["0001", "description", "A fine sentence."], "")
        .status
        .success());
    assert_eq!(field(&path, "description"), via_file);
    assert_eq!(via_file, "A fine sentence.");
}

#[test]
fn from_value_status_closes_the_record() {
    let (docs, path) = issue_bundle("from-status");
    assert!(set_from_file(&docs, &["0001", "status"], "closed\n")
        .status
        .success());
    assert_eq!(field(&path, "status"), "closed");
}

#[test]
fn from_value_invalid_status_fails_like_the_argv_form() {
    let (docs, _) = issue_bundle("from-bogus");
    let via_file = set_from_file(&docs, &["0001", "status"], "bogus\n");
    let via_argv = set(&docs, &["0001", "status", "bogus"], "");
    assert_eq!(via_file.status.code(), Some(2));
    assert_eq!(via_file.stderr, via_argv.stderr);
}

#[test]
fn from_value_stdin_closes_the_record() {
    let (docs, path) = issue_bundle("from-stdin");
    let out = set(&docs, &["0001", "status", "--from", "-"], "closed\n");
    assert!(out.status.success(), "{out:?}");
    assert_eq!(field(&path, "status"), "closed");
}

#[test]
fn from_value_trims_and_normalizes_line_endings() {
    let (docs, path) = issue_bundle("from-crlf");
    assert!(
        set_from_file(&docs, &["0001", "description"], "  \r\n one line \r\n\r\n")
            .status
            .success()
    );
    assert_eq!(field(&path, "description"), "one line");
}

const WITH_OUTCOME: &str = "### Outcome\n\nold\n\n#### sub\n\nold sub\n\n### Plan\n\nkeep\n";

fn issue_with(label: &str, tail: &str) -> (common::Scratch, PathBuf) {
    let (docs, path) = issue_bundle(label);
    let text = fs::read_to_string(&path).unwrap();
    let head = text.split("### Scope").next().unwrap();
    fs::write(&path, format!("{head}### Scope\n\ns\n\n{tail}")).unwrap();
    (docs, path)
}

fn set_section(docs: &Path, heading: &str, content: &str) -> Output {
    set_from_file(docs, &["0001", "section", heading], content)
}

#[test]
fn section_replaces_existing_text_up_to_the_next_sibling() {
    let (docs, path) = issue_with("sec-replace", WITH_OUTCOME);
    let out = set_section(&docs, "Outcome", "new text\n");
    assert!(out.status.success(), "{out:?}");
    let text = fs::read_to_string(&path).unwrap();
    assert!(
        text.ends_with("### Outcome\n\nnew text\n\n### Plan\n\nkeep\n"),
        "{text}"
    );
    assert!(!text.contains("old sub"));
}

#[test]
fn section_matches_the_heading_case_insensitively() {
    let (docs, path) = issue_with("sec-case", WITH_OUTCOME);
    assert!(set_section(&docs, "outcome", "new text\n").status.success());
    let text = fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("### Outcome\n\nnew text\n\n### Plan"),
        "{text}"
    );
}

#[test]
fn section_appends_at_the_end_when_absent() {
    let (docs, path) = issue_with("sec-append", "### Plan\n\nkeep\n");
    assert!(set_section(&docs, "Outcome", "done\n").status.success());
    let text = fs::read_to_string(&path).unwrap();
    assert!(
        text.ends_with("### Plan\n\nkeep\n\n### Outcome\n\ndone\n"),
        "{text}"
    );
}

#[test]
fn section_appends_with_the_level_of_the_records_sections() {
    let docs = temp_bundle("set-from", "sec-adr");
    assert!(run_new(&docs, "adr", "A").status.success());
    assert!(set_section(&docs, "Notes", "extra\n").status.success());
    let text = fs::read_to_string(docs.join("adr/0001-a.md")).unwrap();
    assert!(text.ends_with("\n\n## Notes\n\nextra\n"), "{text}");
}

#[test]
fn section_leaves_index_and_frontmatter_untouched() {
    let (docs, path) = issue_with("sec-index", WITH_OUTCOME);
    let index = docs.join("issues/index.md");
    let before_index = fs::read_to_string(&index).unwrap();
    let before = fs::read_to_string(&path).unwrap();
    assert!(set_section(&docs, "Outcome", "x\n").status.success());
    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(fs::read_to_string(&index).unwrap(), before_index);
    assert_eq!(
        after.split("### Scope").next(),
        before.split("### Scope").next()
    );
}

#[test]
fn section_ignores_a_heading_inside_a_fenced_block() {
    let tail = "```\n### Outcome\n```\n\n### Plan\n\nkeep\n";
    let (docs, path) = issue_with("sec-fence", tail);
    assert!(set_section(&docs, "Outcome", "real\n").status.success());
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("```\n### Outcome\n```"), "{text}");
    assert!(
        text.ends_with("### Plan\n\nkeep\n\n### Outcome\n\nreal\n"),
        "{text}"
    );
}

fn assert_usage_error_writing_nothing(label: &str, args: &[&str], needle: &str) {
    let (docs, path) = issue_bundle(label);
    let before = fs::read_to_string(&path).unwrap();
    let file = value_file(&docs, "x\n");
    let args: Vec<&str> = args
        .iter()
        .map(|arg| if *arg == "FILE" { file.as_str() } else { arg })
        .collect();
    let out = set(&docs, &args, "");
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(needle), "{stderr}");
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
}

#[test]
fn usage_value_and_from_together_are_refused() {
    let args = ["0001", "description", "x", "--from", "FILE"];
    assert_usage_error_writing_nothing("u-both", &args, "--from");
}

#[test]
fn usage_neither_value_nor_from_is_refused() {
    assert_usage_error_writing_nothing("u-none", &["0001", "description"], "--from");
}

#[test]
fn usage_section_without_from_is_refused() {
    let args = ["0001", "section", "Outcome"];
    assert_usage_error_writing_nothing("u-nofrom", &args, "--from");
}

#[test]
fn usage_section_without_heading_is_refused() {
    let args = ["0001", "section", "--from", "FILE"];
    assert_usage_error_writing_nothing("u-nohead", &args, "heading");
}
