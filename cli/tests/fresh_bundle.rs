mod common;

use common::{living_docs, run_check, run_check_advisories, run_new, stdout_of, temp_bundle};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Output;

/// A fresh docs directory inside a new git repository, as a real project has.
fn git_bundle(label: &str) -> common::Scratch {
    let docs = temp_bundle("fresh-bundle", label);
    let init = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(docs.parent().unwrap())
        .status()
        .expect("failed to run git init");
    assert!(init.success());
    docs
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn run_index(docs: &Path) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "index"])
        .output()
        .expect("failed to run living-docs index")
}

fn root_of(docs: &Path) -> String {
    fs::read_to_string(docs.join("index.md")).unwrap()
}

fn assert_new_ok(docs: &Path, doc_type: &str, title: &str) {
    let output = run_new(docs, doc_type, title);
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));
}

fn invariant_three_lines(docs: &Path) -> Vec<String> {
    stdout_of(&run_check(docs))
        .lines()
        .filter(|line| line.contains("invariant 3"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn root_index_is_written_when_missing_and_reaches_the_type_index() {
    let docs = git_bundle("root-written");
    assert_new_ok(&docs, "adr", "A");

    let root = root_of(&docs);
    assert!(root.contains("](adr/)"), "got:\n{root}");
    assert!(root.contains("okf_version"), "got:\n{root}");
    assert_eq!(invariant_three_lines(&docs), Vec::<String>::new());
}

#[test]
fn root_index_gains_one_appended_link_for_a_new_type_and_is_then_left_alone() {
    let docs = git_bundle("root-appended");
    assert_new_ok(&docs, "adr", "A");
    let before = root_of(&docs);

    assert_new_ok(&docs, "issue", "B");
    let after = root_of(&docs);
    assert!(after.starts_with(&before), "got:\n{after}");
    let appended = &after[before.len()..];
    assert_eq!(appended.lines().count(), 1, "got:\n{appended}");
    assert!(appended.contains("](issues/)"), "got:\n{appended}");
    assert_eq!(invariant_three_lines(&docs), Vec::<String>::new());

    assert_new_ok(&docs, "adr", "C");
    assert_eq!(root_of(&docs), after);
    assert!(run_index(&docs).status.success());
    assert_eq!(root_of(&docs), after);
}

#[test]
fn root_index_that_already_reaches_every_type_index_is_byte_identical() {
    let docs = git_bundle("root-framed");
    let framed = "---\nokf_version: \"0.1\"\n---\n\n# Mine\n\n* [Decisions](adr/index.md)\n* [Work](issues/)\n";
    common::write(&docs, "index.md", framed);

    assert_new_ok(&docs, "adr", "A");
    assert_new_ok(&docs, "issue", "B");
    assert!(run_index(&docs).status.success());
    assert_eq!(root_of(&docs), framed);
}

#[test]
fn root_index_is_written_in_the_nested_docs_dir() {
    let bundle = git_bundle("root-nested");
    let docs = bundle.join("deep/docs");
    fs::create_dir_all(&docs).unwrap();
    assert_new_ok(&docs, "adr", "A");

    assert!(docs.join("index.md").is_file());
    assert!(!docs.parent().unwrap().join("index.md").exists());
}

#[test]
fn index_alone_writes_a_missing_root_and_is_idempotent() {
    let docs = git_bundle("root-by-index");
    assert_new_ok(&docs, "adr", "A");
    fs::remove_file(docs.join("index.md")).unwrap();

    assert!(run_index(&docs).status.success());
    let first = root_of(&docs);
    assert!(first.contains("](adr/)"), "got:\n{first}");
    assert!(run_index(&docs).status.success());
    assert_eq!(root_of(&docs), first);
}

#[test]
fn hostile_root_that_is_a_directory_fails_new_naming_it_and_deletes_nothing() {
    let docs = git_bundle("hostile-dir");
    fs::create_dir_all(docs.join("index.md")).unwrap();

    let output = run_new(&docs, "adr", "A");
    assert!(!output.status.success());
    assert!(
        stderr_of(&output).contains("index.md"),
        "{}",
        stderr_of(&output)
    );
    assert!(
        stdout_of(&output).contains("0001-a.md"),
        "{}",
        stdout_of(&output)
    );
    assert!(docs.join("adr/0001-a.md").is_file());
    assert!(docs.join("adr/index.md").is_file());
    assert!(docs.join("index.md").is_dir());
}

#[test]
fn hostile_unreadable_root_fails_new_naming_it_and_deletes_nothing() {
    let docs = git_bundle("hostile-unreadable");
    common::write(&docs, "index.md", "keep\n");
    let path = docs.join("index.md");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();

    let output = run_new(&docs, "adr", "A");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(!output.status.success());
    assert!(
        stderr_of(&output).contains("index.md"),
        "{}",
        stderr_of(&output)
    );
    assert!(docs.join("adr/0001-a.md").is_file());
    assert_eq!(fs::read_to_string(&path).unwrap(), "keep\n");
}

#[test]
fn hostile_empty_root_is_present_and_only_gains_the_missing_link() {
    let docs = git_bundle("hostile-empty");
    common::write(&docs, "index.md", "");

    assert_new_ok(&docs, "adr", "A");
    let root = root_of(&docs);
    assert_eq!(root.lines().count(), 1, "got:\n{root}");
    assert!(root.contains("](adr/)"), "got:\n{root}");
}

#[test]
fn hostile_dot_slash_link_to_the_type_index_counts_as_reaching_it() {
    let docs = git_bundle("hostile-dotslash");
    let framed = "---\nokf_version: \"0.1\"\n---\n\n* [ADRs](./adr/index.md)\n";
    common::write(&docs, "index.md", framed);

    assert_new_ok(&docs, "adr", "A");
    assert_eq!(root_of(&docs), framed);
}

#[test]
fn constitution_seeds_its_own_status_and_the_given_title_as_heading() {
    let docs = git_bundle("constitution");
    assert_new_ok(&docs, "constitution", "Our rules");

    let text = fs::read_to_string(docs.join("constitution.md")).unwrap();
    assert!(text.contains("\nstatus: Draft"), "got:\n{text}");
    assert!(text.contains("\ntitle: Our rules\n"), "got:\n{text}");
    assert!(text.contains("\n# Our rules\n"), "got:\n{text}");
    let advisories = stdout_of(&run_check_advisories(&docs));
    assert!(!advisories.contains("HEADING"), "got:\n{advisories}");
}

#[test]
fn constitution_first_writes_a_link_less_root_that_a_later_type_extends() {
    let docs = git_bundle("constitution-first");
    assert_new_ok(&docs, "constitution", "Our rules");

    let before = root_of(&docs);
    assert!(before.contains("okf_version"), "got:\n{before}");
    assert!(!before.contains("]("), "got:\n{before}");
    let checked = stdout_of(&run_check(&docs));
    assert!(!checked.contains("invariant 3"), "got:\n{checked}");
    assert!(!checked.contains("okf_version"), "got:\n{checked}");

    assert_new_ok(&docs, "adr", "A");
    let after = root_of(&docs);
    assert_eq!(after, format!("{before}* [ADRs](adr/)\n"));

    let again = run_new(&docs, "constitution", "Other");
    assert!(!again.status.success());
    assert_eq!(root_of(&docs), after);
}

#[test]
fn index_alone_writes_no_root_for_a_constitution_only_bundle() {
    let docs = git_bundle("constitution-index-only");
    assert_new_ok(&docs, "constitution", "Our rules");
    fs::remove_file(docs.join("index.md")).unwrap();

    assert!(run_index(&docs).status.success());
    assert!(!docs.join("index.md").exists());
}

#[test]
fn hostile_root_failure_gets_its_own_repair_line_when_the_root_is_a_directory() {
    let docs = git_bundle("hostile-own-line");
    fs::create_dir_all(docs.join("index.md")).unwrap();

    let output = run_new(&docs, "adr", "A");
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(docs.join("adr/0001-a.md").is_file());
    assert!(docs.join("adr/index.md").is_file());
    assert!(stderr.contains("index.md"), "{stderr}");
    assert!(
        stderr.contains("fix that path or its directory"),
        "{stderr}"
    );
    let stdout = stdout_of(&output);
    let type_index = docs.join("adr").join("index.md");
    let root_index = docs.join("index.md");
    assert!(stdout.contains(type_index.to_str().unwrap()), "{stdout}");
    assert!(!stdout.contains(root_index.to_str().unwrap()), "{stdout}");
    assert!(stderr.contains("living-docs index"), "{stderr}");
    assert!(!stderr.contains("index not updated"), "{stderr}");
    assert!(!stderr.contains("index adr"), "{stderr}");
}

#[test]
fn hostile_root_failure_gets_its_own_repair_line_when_the_docs_dir_is_read_only() {
    let docs = git_bundle("hostile-readonly");
    fs::create_dir_all(docs.join("adr")).unwrap();
    fs::set_permissions(&docs, fs::Permissions::from_mode(0o555)).unwrap();

    let output = run_new(&docs, "adr", "A");
    fs::set_permissions(&docs, fs::Permissions::from_mode(0o755)).unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(docs.join("adr/0001-a.md").is_file());
    assert!(docs.join("adr/index.md").is_file());
    assert!(stderr.contains("index.md"), "{stderr}");
    assert!(
        stderr.contains("fix that path or its directory"),
        "{stderr}"
    );
    let stdout = stdout_of(&output);
    let type_index = docs.join("adr").join("index.md");
    let root_index = docs.join("index.md");
    assert!(stdout.contains(type_index.to_str().unwrap()), "{stdout}");
    assert!(!stdout.contains(root_index.to_str().unwrap()), "{stdout}");
    assert!(!stderr.contains("index not updated"), "{stderr}");
    assert!(!docs.join("index.md").exists());
}

#[test]
fn hostile_type_index_failure_keeps_its_own_repair_line() {
    let docs = git_bundle("hostile-type-index");
    fs::create_dir_all(docs.join("adr/index.md")).unwrap();

    let output = run_new(&docs, "adr", "A");
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("index not updated"), "{stderr}");
    assert!(stderr.contains("living-docs index adr"), "{stderr}");
}

#[test]
fn constitution_change_leaves_the_other_seeded_statuses_alone() {
    let docs = git_bundle("constitution-others");
    assert_new_ok(&docs, "adr", "X");
    assert_new_ok(&docs, "issue", "Y");

    let adr = fs::read_to_string(docs.join("adr/0001-x.md")).unwrap();
    let issue = fs::read_to_string(docs.join("issues/0001-y.md")).unwrap();
    assert!(adr.contains("\nstatus: Proposed"), "got:\n{adr}");
    assert!(issue.contains("\nstatus: open"), "got:\n{issue}");
}
