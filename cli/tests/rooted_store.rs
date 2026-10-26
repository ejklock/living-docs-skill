//! The filesystem store refuses any read, write or rename that resolves
//! outside the bundle a verb works on, and keeps every in-bundle link working.

#![cfg(unix)]

mod common;

use common::{index_bundle, living_docs, run_check, run_new, stdout_of, temp_bundle, write};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Output;

const RULE: &str = "a record must lie inside the bundle";

fn issue(number: &str, title: &str) -> String {
    format!(
        "---\ntype: Issue\ntitle: {title}\ndescription: A record\nowner: Someone\nstatus: Open\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n# {number}. {title}\n\n### Scope\n\nOne thing.\n\n### Acceptance\n\n- It works.\n"
    )
}

/// The same record with its frontmatter keys out of canonical order, so
/// `fmt` has something to rewrite.
fn untidy_issue(number: &str, title: &str) -> String {
    format!(
        "---\ntitle: {title}\ntype: Issue\ndescription: A record\nowner: Someone\nstatus: Open\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n# {number}. {title}\n\n### Scope\n\nOne thing.\n\n### Acceptance\n\n- It works.\n"
    )
}

struct Fixture {
    docs: PathBuf,
    outside: PathBuf,
    link: PathBuf,
}

/// A bundle holding the real record 0001, the real untidy record 0003 and
/// the record 0002, which is a link to an untidy file outside the bundle.
fn linked_bundle(label: &str) -> Fixture {
    let docs = temp_bundle("rooted-store", label);
    write(&docs, "issues/0001-real.md", &issue("0001", "real"));
    write(&docs, "issues/0002-link.md", &issue("0002", "link"));
    write(
        &docs,
        "issues/0003-untidy.md",
        &untidy_issue("0003", "untidy"),
    );
    index_bundle(&docs);
    let outside = docs.parent().unwrap().join("outside.md");
    fs::write(&outside, untidy_issue("0002", "link")).unwrap();
    let link = docs.join("issues/0002-link.md");
    fs::remove_file(&link).unwrap();
    symlink(&outside, &link).unwrap();
    Fixture {
        docs,
        outside,
        link,
    }
}

fn docs_cmd(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .arg("--docs-dir")
        .arg(docs)
        .args(args)
        .output()
        .unwrap()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn assert_refused_naming(output: &Output, link: &Path) {
    let text = stderr_of(output);
    assert_eq!(output.status.code(), Some(2), "{text}");
    assert!(text.contains(&link.display().to_string()), "{text}");
}

fn assert_outside_kept(fixture: &Fixture) {
    assert_eq!(
        fs::read_to_string(&fixture.outside).unwrap(),
        untidy_issue("0002", "link")
    );
}

#[test]
fn set_on_a_record_linked_outside_is_refused_and_the_target_kept() {
    let fixture = linked_bundle("set");

    let output = docs_cmd(
        &fixture.docs,
        &["set", "issues/0002", "description", "changed"],
    );

    assert_refused_naming(&output, &fixture.link);
    assert_outside_kept(&fixture);
}

#[test]
fn supersede_with_a_record_linked_outside_on_either_side_is_refused() {
    let fixture = linked_bundle("supersede");

    let as_old = docs_cmd(&fixture.docs, &["supersede", "issues/0002", "issues/0001"]);
    assert_refused_naming(&as_old, &fixture.link);
    assert_outside_kept(&fixture);

    let as_new = docs_cmd(&fixture.docs, &["supersede", "issues/0001", "issues/0002"]);
    assert_refused_naming(&as_new, &fixture.link);
    assert_outside_kept(&fixture);
}

#[test]
fn fmt_on_a_record_linked_outside_is_refused_and_the_target_kept() {
    let fixture = linked_bundle("fmt-record");

    let output = living_docs()
        .args(["fmt", "--plain"])
        .arg(&fixture.link)
        .output()
        .unwrap();

    assert_refused_naming(&output, &fixture.link);
    assert_outside_kept(&fixture);
}

#[test]
fn fmt_over_a_bundle_holding_a_link_outside_refuses_the_run_and_writes_nothing() {
    let fixture = linked_bundle("fmt-bundle");
    let untidy = fixture.docs.join("issues/0003-untidy.md");
    let before = fs::read(&untidy).unwrap();

    let output = living_docs()
        .args(["fmt", "--plain"])
        .arg(&fixture.docs)
        .output()
        .unwrap();

    assert_refused_naming(&output, &fixture.link);
    assert_outside_kept(&fixture);
    assert_eq!(fs::read(&untidy).unwrap(), before);
}

#[test]
fn new_through_a_type_directory_linked_outside_is_refused_and_creates_nothing() {
    let docs = temp_bundle("rooted-store", "new-linked-dir");
    let outside = docs.parent().unwrap().join("elsewhere");
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, docs.join("issues")).unwrap();

    let output = run_new(&docs, "issue", "Through the link");

    assert_eq!(output.status.code(), Some(2), "{}", stderr_of(&output));
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}

#[test]
fn set_on_a_record_linked_to_another_file_inside_the_bundle_succeeds() {
    let docs = temp_bundle("rooted-store", "link-inside");
    write(&docs, "issues/0001-real.md", &issue("0001", "real"));
    symlink(
        docs.join("issues/0001-real.md"),
        docs.join("issues/0002-alias.md"),
    )
    .unwrap();

    let output = docs_cmd(&docs, &["set", "issues/0002", "description", "changed"]);

    assert!(output.status.success(), "{}", stderr_of(&output));
    let real = fs::read_to_string(docs.join("issues/0001-real.md")).unwrap();
    assert!(real.contains("description: changed"), "{real}");
}

#[test]
fn every_verb_works_through_a_symlinked_docs_dir() {
    let docs = temp_bundle("rooted-store", "alias-root");
    let alias = docs.parent().unwrap().join("alias");
    symlink(&docs, &alias).unwrap();

    assert!(run_new(&alias, "issue", "First").status.success());
    let set = docs_cmd(&alias, &["set", "issues/0001", "description", "changed"]);
    assert!(set.status.success(), "{}", stderr_of(&set));
    let fmt = living_docs()
        .args(["fmt", "--plain"])
        .arg(&alias)
        .output()
        .unwrap();
    assert!(fmt.status.success(), "{}", stderr_of(&fmt));
    fs::remove_file(docs.join("issues/0001-first.md")).unwrap();
    write(&docs, "issues/0001-clean.md", &issue("0001", "clean"));
    index_bundle(&alias);
    let check = run_check(&alias);
    assert!(check.status.success(), "{}", stdout_of(&check));
    assert!(stdout_of(&check).contains("no invariant violations"));
}

#[test]
fn new_in_a_project_with_no_docs_directory_creates_the_bundle() {
    let project = temp_bundle("rooted-store", "fresh")
        .parent()
        .unwrap()
        .to_path_buf();
    let docs = project.join("fresh-docs");

    let output = run_new(&docs, "issue", "First");

    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(docs.join("issues").is_dir());
}

/// Asserts `check` holds exactly one violation for the link, the refusal,
/// in plain and JSON output, and returns the JSON violations.
fn assert_one_refusal(fixture: &Fixture) -> Vec<serde_json::Value> {
    let link = fixture.link.display().to_string();
    let plain = run_check(&fixture.docs);
    let text = stdout_of(&plain);
    assert_eq!(plain.status.code(), Some(1), "{text}");
    let mentions: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("0002-link.md"))
        .collect();
    assert_eq!(mentions.len(), 1, "{text}");
    assert!(mentions[0].contains(RULE), "{text}");
    assert!(!mentions[0].contains("frontmatter"), "{text}");
    let json = docs_cmd(&fixture.docs, &["check", "--json"]);
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    let all = report["violations"].as_array().unwrap().clone();
    let own: Vec<&serde_json::Value> = all.iter().filter(|v| v["file"] == link.as_str()).collect();
    assert_eq!(own.len(), 1, "{report}");
    assert!(own[0]["message"].as_str().unwrap().contains(RULE));
    all
}

#[test]
fn check_reports_one_violation_for_a_record_linked_outside_the_bundle() {
    assert_one_refusal(&linked_bundle("check"));
}

#[test]
fn check_after_index_skipped_the_linked_record_reports_only_the_refusal() {
    let fixture = linked_bundle("check-reindexed");
    fs::remove_file(fixture.docs.join("issues/0003-untidy.md")).unwrap();
    assert!(docs_cmd(&fixture.docs, &["index"]).status.success());

    let violations = assert_one_refusal(&fixture);

    assert_eq!(violations.len(), 1, "{violations:?}");
}

/// A bundle with the linked record 0002 and a record 0003 made unreadable by
/// the operating system; `None` when the process reads it anyway (root).
fn locked_bundle(label: &str) -> Option<Fixture> {
    let fixture = linked_bundle(label);
    set_mode(&fixture, 0o000);
    let readable = fs::read(fixture.docs.join("issues/0003-untidy.md")).is_ok();
    (!readable).then_some(fixture)
}

fn set_mode(fixture: &Fixture, mode: u32) {
    let locked = fixture.docs.join("issues/0003-untidy.md");
    fs::set_permissions(locked, fs::Permissions::from_mode(mode)).unwrap();
}

fn fmt_check(docs: &Path) -> Output {
    let args = ["fmt", "--check", "--plain"];
    living_docs().args(args).arg(docs).output().unwrap()
}

#[test]
fn an_unreadable_record_keeps_the_handling_it_had_while_a_linked_one_is_refused() {
    let Some(fixture) = locked_bundle("locked") else {
        return;
    };
    let fmt = fmt_check(&fixture.docs);
    let text = stdout_of(&run_check(&fixture.docs));
    set_mode(&fixture, 0o644);

    assert_refused_naming(&fmt, &fixture.link);
    assert!(!stderr_of(&fmt).contains("0003-untidy"));
    let locked: Vec<&str> = text.lines().filter(|l| l.contains("0003-untidy")).collect();
    assert!(
        !locked.iter().any(|l| l.contains("Permission denied")),
        "{text}"
    );
    assert!(!locked.iter().any(|l| l.contains(RULE)), "{text}");
    assert!(
        locked.iter().any(|l| l.contains("missing OKF frontmatter")),
        "{text}"
    );
}

#[test]
fn fmt_check_over_a_bundle_with_only_an_unreadable_record_succeeds_as_before() {
    let Some(fixture) = locked_bundle("locked-only") else {
        return;
    };
    fs::remove_file(&fixture.link).unwrap();
    let fmt = fmt_check(&fixture.docs);
    set_mode(&fixture, 0o644);
    assert_eq!(fmt.status.code(), Some(0), "{}", stderr_of(&fmt));
}
