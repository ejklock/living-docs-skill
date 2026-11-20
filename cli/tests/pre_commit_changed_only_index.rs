//! The distributed pre-commit hook in changed-only mode fails a commit whose
//! staged record leaves its type's index stale, and only that index.
//! Drives the real `skills/living-docs/hooks/pre-commit` script through `git commit`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod common;
use common::write;

const ADR: &str = "---\ntype: ADR\ntitle: TITLE\ndescription: A record.\nowner: a@b.c\nstatus: Accepted\n---\n\n# NUM. TITLE\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n";
const ISSUE: &str = "---\ntype: Issue\ntitle: X\ndescription: A filled description.\nstatus: open\ntimestamp: 2026-11-20T16:31:10Z\n---\n\n## 0001. X\n\n### Scope\n\nOne thing.\n\n### Acceptance\n\n- It works.\n";
const CHANGED_ONLY: [(&str, &str); 1] = [("LIVING_DOCS_CHECK_CHANGED_ONLY", "1")];

fn adr(number: &str, title: &str) -> String {
    ADR.replace("NUM", number).replace("TITLE", title)
}

fn git_with(repo: &Path, vars: &[(&str, &str)], args: &[&str]) -> Output {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_living-docs"));
    let bin_dir = binary.parent().expect("the test binary has a directory");
    Command::new("git")
        .current_dir(repo)
        .env_clear()
        .env("HOME", repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("PATH", format!("{}:/usr/bin:/bin", bin_dir.display()))
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .envs(vars.iter().copied())
        .args(args)
        .output()
        .expect("failed to run git")
}

fn git(repo: &Path, args: &[&str]) -> Output {
    git_with(repo, &[], args)
}

fn commit(repo: &Path, vars: &[(&str, &str)]) -> Output {
    git_with(repo, vars, &["commit", "-q", "-m", "test: change"])
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A repo with a committed, fully indexed bundle (two ADRs, one issue), the
/// hook installed afterwards so the baseline commit runs without it.
fn committed_repo(label: &str) -> common::Scratch {
    let repo = common::temp_bundle("pre-commit-index", label);
    git(&repo, &["init", "-q"]);
    let docs = repo.join("docs");
    write(
        &docs,
        "index.md",
        "# Docs\n\n* [ADRs](adr/)\n* [Issues](issues/)\n",
    );
    write(&docs, "adr/0001-a.md", &adr("0001", "A"));
    write(&docs, "adr/0002-b.md", &adr("0002", "B"));
    write(&docs, "issues/0001-x.md", ISSUE);
    common::index_bundle(&docs);
    git(&repo, &["add", "."]);
    assert!(commit(&repo, &[]).status.success(), "baseline commit");
    let hook = repo.join(".git/hooks/pre-commit");
    fs::copy(hook_source(), &hook).expect("the repo ships a pre-commit hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    }
    repo
}

fn hook_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the cli crate sits under the workspace root")
        .join("skills/living-docs/hooks/pre-commit")
}

fn stale_adr_index(repo: &Path) {
    let index = repo.join("docs/adr/index.md");
    let current = fs::read_to_string(&index).unwrap();
    fs::write(&index, current.replace("Accepted", "Proposed")).unwrap();
}

#[test]
fn a_staged_adr_added_without_its_index_row_fails() {
    let repo = committed_repo("added");
    write(&repo.join("docs"), "adr/0003-c.md", &adr("0003", "C"));
    git(&repo, &["add", "docs/adr/0003-c.md"]);

    let output = commit(&repo, &CHANGED_ONLY);

    assert!(
        !output.status.success(),
        "stale index passed: {}",
        text(&output)
    );
}

#[test]
fn a_staged_body_edit_of_an_existing_adr_fails_while_its_index_is_stale() {
    let repo = committed_repo("body-edit");
    stale_adr_index(&repo);
    let edited = adr("0001", "A").replace("c.\n", "c, reworded.\n");
    write(&repo.join("docs"), "adr/0001-a.md", &edited);
    git(&repo, &["add", "docs/adr/0001-a.md"]);

    let output = commit(&repo, &CHANGED_ONLY);

    assert!(
        !output.status.success(),
        "stale index passed: {}",
        text(&output)
    );
    assert!(text(&output).contains("adr/index.md"), "{}", text(&output));
}

#[test]
fn a_staged_deletion_of_a_dash_named_root_file_prints_no_dirname_error() {
    let repo = common::Scratch::new("living-docs-pre-commit-index-dash");
    git(&repo, &["init", "-q"]);
    write(&repo, "index.md", "# Docs\n");
    write(&repo, "-x.md", "stray\n");
    git(&repo, &["add", "."]);
    assert!(commit(&repo, &[]).status.success(), "baseline commit");
    let hook = repo.join(".git/hooks/pre-commit");
    fs::copy(hook_source(), &hook).expect("the repo ships a pre-commit hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    }
    git(&repo, &["rm", "-q", "--", "-x.md"]);
    let vars = [
        ("LIVING_DOCS_CHECK_CHANGED_ONLY", "1"),
        ("LIVING_DOCS_BUNDLE", "."),
    ];

    let output = commit(&repo, &vars);

    assert!(!text(&output).contains("dirname"), "{}", text(&output));
}

#[test]
fn a_staged_adr_deletion_with_its_row_left_in_the_index_fails() {
    let repo = committed_repo("deleted");
    git(&repo, &["rm", "-q", "docs/adr/0002-b.md"]);

    let output = commit(&repo, &CHANGED_ONLY);

    assert!(
        !output.status.success(),
        "stale index passed: {}",
        text(&output)
    );
}

#[test]
fn a_staged_file_outside_the_bundle_passes() {
    let repo = committed_repo("outside");
    stale_adr_index(&repo);
    write(&repo, "README.md", "hello\n");
    git(&repo, &["add", "README.md"]);

    let output = commit(&repo, &CHANGED_ONLY);

    assert!(output.status.success(), "{}", text(&output));
}

#[test]
fn a_staged_issue_passes_while_only_the_adr_index_is_stale() {
    let repo = committed_repo("issue");
    stale_adr_index(&repo);
    let issue = ISSUE.replace("One thing.", "Another thing.");
    write(&repo.join("docs"), "issues/0001-x.md", &issue);
    git(&repo, &["add", "docs/issues/0001-x.md"]);

    let output = commit(&repo, &CHANGED_ONLY);

    assert!(output.status.success(), "{}", text(&output));
}

#[test]
fn a_hand_broken_unstaged_record_passes() {
    let repo = committed_repo("unstaged");
    write(&repo.join("docs"), "adr/0001-a.md", "{{UNFILLED: debt}}\n");
    let issue = ISSUE.replace("One thing.", "Another thing.");
    write(&repo.join("docs"), "issues/0001-x.md", &issue);
    git(&repo, &["add", "docs/issues/0001-x.md"]);

    let output = commit(&repo, &CHANGED_ONLY);

    assert!(output.status.success(), "{}", text(&output));
}

#[test]
fn without_the_variable_the_whole_bundle_is_checked() {
    let repo = committed_repo("whole");
    stale_adr_index(&repo);
    let issue = ISSUE.replace("One thing.", "Another thing.");
    write(&repo.join("docs"), "issues/0001-x.md", &issue);
    git(&repo, &["add", "docs/issues/0001-x.md"]);

    let output = commit(&repo, &[]);

    assert!(!output.status.success(), "{}", text(&output));
}
