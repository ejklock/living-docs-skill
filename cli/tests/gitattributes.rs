mod common;

use common::{living_docs, stdout_of, Scratch};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const UNION_LINE: &str = "docs/**/index.md merge=union";
const BODY: &str = "### Scope\n\nOne thing.\n\n### Acceptance\n\n- It works.\n";

fn project(label: &str) -> Scratch {
    let project = Scratch::new(&format!("living-docs-gitattributes-test-{label}"));
    fs::create_dir_all(project.join("docs")).unwrap();
    project
}

fn install(project: &Path, extra: &[&str]) -> Output {
    living_docs()
        .args(["install", "hooks", "--dir"])
        .arg(project)
        .args(["--docs-dir", "docs", "--quiet"])
        .args(extra)
        .output()
        .unwrap()
}

fn uninstall(project: &Path) -> Output {
    living_docs()
        .args(["uninstall", "hooks", "--dir"])
        .arg(project)
        .output()
        .unwrap()
}

fn attributes(project: &Path) -> String {
    fs::read_to_string(project.join(".gitattributes")).unwrap()
}

fn union_lines(text: &str) -> usize {
    text.lines().filter(|line| *line == UNION_LINE).count()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[path = "gitattributes/scope.rs"]
mod scope;

#[test]
fn install_hooks_creates_the_attributes_file_with_one_union_line_and_stays_idempotent() {
    let project = project("create");
    assert!(install(&project, &[]).status.success());
    assert!(install(&project, &[]).status.success());
    assert_eq!(attributes(&project), format!("{UNION_LINE}\n"));
}

#[test]
fn install_hooks_keeps_a_foreign_attribute_line_byte_identical() {
    let project = project("foreign");
    let foreign = "*.png binary\n*.lock  -diff  \n";
    fs::write(project.join(".gitattributes"), foreign).unwrap();
    assert!(install(&project, &[]).status.success());
    assert!(install(&project, &[]).status.success());
    let text = attributes(&project);
    assert!(text.starts_with(foreign), "foreign lines moved: {text:?}");
    assert_eq!(union_lines(&text), 1);
    assert_eq!(text.len(), foreign.len() + UNION_LINE.len() + 1);
}

#[test]
fn install_hooks_dry_run_writes_no_attributes_file() {
    let project = project("dry");
    let output = install(&project, &["--dry-run"]);
    assert!(output.status.success());
    assert!(!project.join(".gitattributes").exists());
    assert!(stdout_of(&output).contains(".gitattributes"));
}

#[test]
fn uninstall_hooks_removes_only_the_union_line() {
    let project = project("remove");
    fs::write(project.join(".gitattributes"), "*.png binary\n").unwrap();
    assert!(install(&project, &[]).status.success());
    assert!(uninstall(&project).status.success());
    assert_eq!(attributes(&project), "*.png binary\n");
}

#[test]
fn uninstall_hooks_removes_an_attributes_file_that_held_only_the_union_line() {
    let project = project("only");
    assert!(install(&project, &[]).status.success());
    assert!(uninstall(&project).status.success());
    assert!(!project.join(".gitattributes").exists());
}

#[test]
fn uninstall_hooks_without_an_attributes_file_succeeds_and_creates_none() {
    let project = project("absent");
    assert!(uninstall(&project).status.success());
    assert!(!project.join(".gitattributes").exists());
}

#[test]
fn a_symlinked_attributes_file_fails_install_and_uninstall_writing_nothing() {
    let project = project("symlink");
    let target = project.join("elsewhere");
    fs::write(&target, "*.png binary\n").unwrap();
    symlink(&target, project.join(".gitattributes")).unwrap();
    for output in [install(&project, &[]), uninstall(&project)] {
        assert!(!output.status.success());
        assert!(stderr_of(&output).contains(".gitattributes"));
    }
    assert_eq!(fs::read_to_string(&target).unwrap(), "*.png binary\n");
    assert!(!project.join(".living-docs").exists());
}

#[test]
fn a_docs_dir_outside_the_project_skips_the_union_line_and_says_so() {
    let root = project("outside");
    let outside_project = project("outside-docs");
    let outside = outside_project.join("docs");
    let output = living_docs()
        .args(["install", "hooks", "--dir"])
        .arg(&root)
        .arg("--docs-dir")
        .arg(&outside)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(!root.join(".gitattributes").exists());
    let stderr = stderr_of(&output);
    assert_eq!(stderr.matches("merge=union").count(), 1, "{stderr}");
    assert!(stderr.contains("outside"), "{stderr}");
}

#[test]
fn a_directory_named_gitattributes_fails_install_naming_it() {
    let project = project("directory");
    fs::create_dir(project.join(".gitattributes")).unwrap();
    let output = install(&project, &[]);
    assert!(!output.status.success());
    assert!(stderr_of(&output).contains(".gitattributes"));
}

fn git(repo: &Path, args: &[&str]) -> Output {
    let config = repo.join(".git-test-config");
    let output = Command::new("git")
        .current_dir(repo)
        .env("GIT_CONFIG_GLOBAL", &config)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        stderr_of(&output)
    );
    output
}

fn new_issue(repo: &Path, title: &str) {
    let body = repo.join("body.txt");
    fs::write(&body, BODY).unwrap();
    let output = living_docs()
        .current_dir(repo)
        .args(["new", "issue", title, "--plain", "--body"])
        .arg(&body)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr_of(&output));
}

fn commit_all(repo: &Path, message: &str) {
    git(repo, &["add", "docs", ".gitattributes"]);
    git(repo, &["commit", "-q", "--no-verify", "-m", message]);
}

fn check(repo: &Path) -> Output {
    living_docs()
        .current_dir(repo)
        .args(["check", "docs", "--plain"])
        .output()
        .unwrap()
}

fn merged_repo(label: &str) -> Scratch {
    let repo = project(label);
    fs::write(
        repo.join("docs/index.md"),
        "# Docs\n\n- [Issues](issues/)\n",
    )
    .unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    assert!(install(&repo, &[]).status.success());
    new_issue(&repo, "Base");
    commit_all(&repo, "base");
    for branch in ["alpha", "beta"] {
        git(&repo, &["checkout", "-q", "-b", branch, "main"]);
        new_issue(&repo, branch);
        commit_all(&repo, branch);
    }
    git(&repo, &["merge", "-q", "--no-edit", "alpha"]);
    repo
}

#[test]
fn parallel_branches_adding_issues_merge_without_conflict_and_check_names_the_repair() {
    let repo = merged_repo("merge-check");
    let index = fs::read_to_string(repo.join("docs").join("issues").join("index.md")).unwrap();
    assert!(!index.contains("<<<<<<<"), "conflict markers: {index}");
    assert!(index.contains("0002-alpha.md") && index.contains("0002-beta.md"));
    let drifted = check(&repo);
    assert_eq!(drifted.status.code(), Some(1), "{}", stdout_of(&drifted));
    let report = stdout_of(&drifted);
    assert!(report.contains("issues/index.md"), "{report}");
    assert!(report.contains("living-docs index issue"), "{report}");
}

#[test]
fn indexing_after_a_union_merge_leaves_only_the_colliding_number() {
    let repo = merged_repo("merge-index");
    let indexed = living_docs()
        .current_dir(&repo)
        .args(["index", "--plain"])
        .output()
        .unwrap();
    assert!(indexed.status.success());
    let report = check(&repo);
    let text = stdout_of(&report);
    assert_eq!(report.status.code(), Some(1), "{text}");
    assert!(text.contains("1 violation(s)"), "{text}");
    assert!(text.contains("records share the number 0002"), "{text}");
    assert!(
        text.contains("0002-alpha.md") && text.contains("0002-beta.md"),
        "{text}"
    );
    assert!(!text.contains("living-docs index issue"), "{text}");
}

fn issue_file(repo: &Path, name: &str) -> PathBuf {
    repo.join("docs").join("issues").join(name)
}

fn set_beta_number_next(repo: &Path) -> Output {
    let stale = Path::new("docs").join("issues").join("0002-beta.md");
    living_docs()
        .current_dir(repo)
        .arg("set")
        .arg(stale)
        .args(["number", "next", "--plain"])
        .output()
        .unwrap()
}

#[test]
fn set_number_next_repairs_a_union_merge_collision_until_check_passes() {
    let repo = merged_repo("merge-repair");
    let indexed = living_docs()
        .current_dir(&repo)
        .args(["index", "--plain"])
        .output()
        .unwrap();
    assert!(indexed.status.success());
    let alpha = fs::read(issue_file(&repo, "0002-alpha.md")).unwrap();
    let moved = set_beta_number_next(&repo);
    assert_eq!(moved.status.code(), Some(0), "{}", stderr_of(&moved));
    assert!(issue_file(&repo, "0003-beta.md").exists());
    assert!(!issue_file(&repo, "0002-beta.md").exists());
    let report = check(&repo);
    assert_eq!(report.status.code(), Some(0), "{}", stdout_of(&report));
    let index = fs::read_to_string(issue_file(&repo, "index.md")).unwrap();
    assert!(index.contains("0002-alpha.md") && index.contains("0003-beta.md"));
    assert_eq!(fs::read(issue_file(&repo, "0002-alpha.md")).unwrap(), alpha);
    let stale = set_beta_number_next(&repo);
    assert_eq!(stale.status.code(), Some(2), "{}", stderr_of(&stale));
}
