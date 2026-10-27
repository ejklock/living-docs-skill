use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

mod common;
use common::Scratch;

const HINT: &str = "living-docs install hooks";
const RUNS_CHECK: &str = "#!/bin/sh\nliving-docs check docs\n";

fn isolated<'a>(command: &'a mut Command, home: &Path) -> &'a mut Command {
    command
        .env("HOME", home)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
}

fn temp_dir(label: &str) -> Scratch {
    let dir = Scratch::new(&format!("living-docs-check-hint-{label}"));
    fs::create_dir_all(dir.join("docs")).unwrap();
    fs::write(dir.join("docs/index.md"), "# Documents\n").unwrap();
    dir
}

fn living_docs(cwd: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_living-docs"));
    isolated(&mut command, cwd)
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn repo(label: &str) -> Scratch {
    let dir = temp_dir(label);
    let mut command = Command::new("git");
    let status = isolated(&mut command, &dir)
        .current_dir(&dir)
        .args(["init", "-q"])
        .status()
        .unwrap();
    assert!(status.success());
    dir
}

fn write_pre_commit(repo: &Path, body: &str) {
    let path = repo.join(".git/hooks/pre-commit");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, body).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn hint_names_install_hooks_once_when_the_repository_has_no_pre_commit() {
    let dir = repo("none");
    let err = stderr(&living_docs(&dir, &["check", "docs"]));
    assert_eq!(err.matches(HINT).count(), 1, "stderr: {err}");
}

#[test]
fn hint_is_absent_after_install_hooks_arms_the_gate() {
    let dir = repo("installed");
    let install = living_docs(&dir, &["install", "hooks"]);
    assert!(install.status.success(), "{}", stderr(&install));
    let err = stderr(&living_docs(&dir, &["check", "docs"]));
    assert!(!err.contains(HINT), "stderr: {err}");
}

#[test]
fn hint_is_absent_when_the_default_hooks_directory_runs_check() {
    let dir = repo("default-hook");
    write_pre_commit(&dir, RUNS_CHECK);
    let err = stderr(&living_docs(&dir, &["check", "docs"]));
    assert!(!err.contains(HINT), "stderr: {err}");
}

#[test]
fn hint_appears_when_the_pre_commit_does_not_run_living_docs() {
    let dir = repo("foreign-hook");
    write_pre_commit(&dir, "#!/bin/sh\nexit 0\n");
    let err = stderr(&living_docs(&dir, &["check", "docs"]));
    assert!(err.contains(HINT), "stderr: {err}");
}

#[test]
fn hint_follows_the_repository_holding_the_bundle_not_the_working_directory() {
    let ungated = repo("cwd-ungated");
    let gated = repo("bundle-gated");
    write_pre_commit(&gated, RUNS_CHECK);
    let gated_docs = gated.join("docs");
    let err = stderr(&living_docs(
        &ungated,
        &["check", gated_docs.to_str().unwrap()],
    ));
    assert!(!err.contains(HINT), "stderr: {err}");

    let ungated_docs = ungated.join("docs");
    let err = stderr(&living_docs(
        &gated,
        &["check", ungated_docs.to_str().unwrap()],
    ));
    assert!(err.contains(HINT), "stderr: {err}");
}

#[test]
fn hint_is_absent_when_the_bundle_is_nested_below_the_repository_root() {
    let dir = repo("nested");
    write_pre_commit(&dir, RUNS_CHECK);
    let nested = dir.join("sub/docs");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("index.md"), "# Documents\n").unwrap();
    let err = stderr(&living_docs(&dir, &["check", "sub/docs"]));
    assert!(!err.contains(HINT), "stderr: {err}");
    let err = stderr(&living_docs(&dir.join("sub"), &["check", "docs"]));
    assert!(!err.contains(HINT), "stderr: {err}");
}

#[test]
fn hint_is_absent_when_a_relative_hooks_path_holds_the_gate() {
    let dir = repo("relative-hooks-path");
    fs::create_dir_all(dir.join(".githooks")).unwrap();
    fs::write(dir.join(".githooks/pre-commit"), RUNS_CHECK).unwrap();
    let mut git = Command::new("git");
    let status = isolated(&mut git, &dir)
        .current_dir(&dir)
        .args(["config", "core.hooksPath", ".githooks"])
        .status()
        .unwrap();
    assert!(status.success());
    let err = stderr(&living_docs(&dir.join("docs"), &["check", "."]));
    assert!(!err.contains(HINT), "stderr: {err}");
}

#[test]
fn hint_is_absent_outside_a_git_repository() {
    let dir = temp_dir("no-git");
    let err = stderr(&living_docs(&dir, &["check", "docs"]));
    assert!(!err.contains(HINT), "stderr: {err}");
    assert!(!err.to_lowercase().contains("fatal"), "stderr: {err}");
}

#[test]
fn contract_quiet_silences_the_hint() {
    let dir = repo("quiet");
    let err = stderr(&living_docs(&dir, &["--quiet", "check", "docs"]));
    assert!(!err.contains(HINT), "stderr: {err}");
}

#[test]
fn contract_passing_bundle_keeps_stdout_and_exit_zero_with_and_without_a_gate() {
    let dir = repo("json-pass");
    let bare = living_docs(&dir, &["check", "--json", "docs"]);
    assert!(stderr(&bare).contains(HINT));
    write_pre_commit(&dir, RUNS_CHECK);
    let armed = living_docs(&dir, &["check", "--json", "docs"]);
    assert!(!stderr(&armed).contains(HINT));
    assert_eq!(bare.stdout, armed.stdout);
    assert_eq!(bare.status.code(), Some(0));
    assert_eq!(armed.status.code(), Some(0));
}

#[test]
fn contract_failing_bundle_keeps_stdout_and_exit_one_with_and_without_a_gate() {
    let dir = repo("json-fail");
    fs::write(dir.join("docs/broken.md"), "no frontmatter here\n").unwrap();
    let bare = living_docs(&dir, &["check", "--json", "docs"]);
    write_pre_commit(&dir, RUNS_CHECK);
    let armed = living_docs(&dir, &["check", "--json", "docs"]);
    assert_eq!(bare.status.code(), Some(1));
    assert_eq!(armed.status.code(), Some(1));
    assert_eq!(bare.stdout, armed.stdout);
}
