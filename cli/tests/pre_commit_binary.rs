//! Which `living-docs` the pre-commit doc-gate runs: the repository build
//! when it is executable, the one on `PATH` otherwise, and a fail-open skip
//! when neither exists. Drives both hook sources through `git commit` with
//! stand-in executables that record which one ran.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod common;

const HOOKS: [&str; 2] = [
    ".githooks/pre-commit",
    "skills/living-docs/hooks/pre-commit",
];

fn hook_source(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the cli crate sits under the workspace root")
        .join(relative)
}

/// Writes an executable that appends `name` to the file `$STAND_IN_MARKER`
/// names, so the marker path never passes through shell quoting.
fn stand_in(path: &Path, name: &str, exit: i32, mode: u32) {
    fs::create_dir_all(path.parent().expect("a stand-in has a directory")).unwrap();
    let body = format!("#!/bin/sh\necho {name} >> \"$STAND_IN_MARKER\"\nexit {exit}\n");
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

struct Setup {
    on_path: Option<i32>,
    in_repo: Option<(i32, u32)>,
}

struct Run {
    output: Output,
    ran: String,
}

/// A `git` command that sees only the scratch directory: no inherited
/// environment, no system or user configuration, and the stand-in `PATH`.
fn isolated_git(repo: &Path, root: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .current_dir(repo)
        .env_clear()
        .env("HOME", root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("STAND_IN_MARKER", root.join("ran"))
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", root.join("bin").display()),
        )
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .args(args)
        .output()
        .expect("failed to run git")
}

fn commit_with(hook: &str, setup: &Setup) -> Run {
    let scratch = common::Scratch::with_subdir("pre-commit-binary", "repo");
    let repo: &Path = &scratch;
    let root = scratch.root();
    fs::create_dir_all(root.join("bin")).unwrap();
    isolated_git(repo, root, &["init", "-q"]);
    let installed = repo.join(".git/hooks/pre-commit");
    fs::copy(hook_source(hook), &installed).unwrap();
    fs::set_permissions(&installed, fs::Permissions::from_mode(0o755)).unwrap();
    if let Some(exit) = setup.on_path {
        stand_in(&root.join("bin/living-docs"), "path", exit, 0o755);
    }
    if let Some((exit, mode)) = setup.in_repo {
        stand_in(&repo.join("target/release/living-docs"), "repo", exit, mode);
    }
    let output = isolated_git(repo, root, &["commit", "-q", "--allow-empty", "-m", "x"]);
    let ran = fs::read_to_string(root.join("ran")).unwrap_or_default();
    Run {
        output,
        ran: ran.trim().to_string(),
    }
}

fn each_hook(setup: &Setup, check: impl Fn(&str, Run)) {
    for hook in HOOKS {
        check(hook, commit_with(hook, setup));
    }
}

#[test]
fn the_repository_build_runs_when_a_path_binary_is_also_present() {
    let setup = Setup {
        on_path: Some(0),
        in_repo: Some((0, 0o755)),
    };
    each_hook(&setup, |hook, run| {
        assert_eq!(run.ran, "repo", "{hook} ran the wrong binary");
        assert!(run.output.status.success(), "{hook} blocked the commit");
    });
}

#[test]
fn the_path_binary_runs_when_there_is_no_repository_build() {
    let setup = Setup {
        on_path: Some(0),
        in_repo: None,
    };
    each_hook(&setup, |hook, run| assert_eq!(run.ran, "path", "{hook}"));
}

#[test]
fn the_repository_build_runs_when_there_is_no_path_binary() {
    let setup = Setup {
        on_path: None,
        in_repo: Some((0, 0o755)),
    };
    each_hook(&setup, |hook, run| assert_eq!(run.ran, "repo", "{hook}"));
}

#[test]
fn a_non_executable_repository_build_is_passed_over_for_the_path_binary() {
    let setup = Setup {
        on_path: Some(0),
        in_repo: Some((0, 0o644)),
    };
    each_hook(&setup, |hook, run| assert_eq!(run.ran, "path", "{hook}"));
}

#[test]
fn the_hook_skips_with_a_message_when_no_binary_is_found() {
    let setup = Setup {
        on_path: None,
        in_repo: None,
    };
    each_hook(&setup, |hook, run| {
        let stderr = String::from_utf8_lossy(&run.output.stderr);
        assert!(run.output.status.success(), "{hook} blocked the commit");
        assert!(
            stderr.contains("(target/release or PATH)"),
            "{hook}: {stderr}"
        );
        assert_eq!(run.ran, "", "{hook}");
    });
}

#[test]
fn the_chosen_binary_exit_status_is_the_hook_verdict() {
    let setup = Setup {
        on_path: Some(0),
        in_repo: Some((1, 0o755)),
    };
    each_hook(&setup, |hook, run| {
        assert_eq!(run.ran, "repo", "{hook}");
        assert!(!run.output.status.success(), "{hook} let a failure through");
    });
}
