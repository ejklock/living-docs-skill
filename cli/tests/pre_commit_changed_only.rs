//! The pre-commit hook's `LIVING_DOCS_CHECK_CHANGED_ONLY` opt-in:
//! a bundle carrying legacy debt can still gate the records a commit touches.
//! Drives the real `.githooks/pre-commit` script through `git commit`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod common;
use common::write;

const LEGACY_DEBT: &str = "---\ntype: ADR\ntitle: Legacy debt\ndescription: A record adopted with debt.\nowner: a@b.c\nstatus: Accepted\n---\n\n# 0001. Legacy debt\n\n{{UNFILLED: inherited debt}}\n";
const CLEAN: &str = "---\ntype: ADR\ntitle: Clean\ndescription: A record with no debt.\nowner: a@b.c\nstatus: Accepted\n---\n\n# 0002. Clean\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n";

fn hook_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the cli crate sits under the workspace root")
        .join(".githooks/pre-commit")
}

/// Runs `git` seeing only the scratch repo: no inherited environment and no
/// system or user configuration. `PATH` holds the test binary's directory and
/// a fixed system tail, and `vars` are the only extra variables the hook sees.
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

/// A git repo whose bundle carries one record with legacy debt and one clean
/// record, with the repo's real pre-commit hook installed and only the clean
/// record staged.
fn repo_with_debt(label: &str) -> common::Scratch {
    let repo = common::temp_bundle("pre-commit", label);
    git(&repo, &["init", "-q"]);
    let hook = repo.join(".git/hooks/pre-commit");
    fs::copy(hook_source(), &hook).expect("the repo ships a pre-commit hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).expect("hook is executable");
    }

    let docs = repo.join("docs");
    write(&docs, "index.md", "# Docs\n\n* [ADRs](adr/)\n");
    write(
        &docs,
        "adr/index.md",
        "# ADRs\n\n* [0001 — Legacy debt](0001-legacy-debt.md) - Accepted\n* [0002 — Clean](0002-clean.md) - Accepted\n",
    );
    write(&docs, "adr/0001-legacy-debt.md", LEGACY_DEBT);
    write(&docs, "adr/0002-clean.md", CLEAN);
    common::index_bundle(&docs);
    git(
        &repo,
        &[
            "add",
            "docs/index.md",
            "docs/adr/index.md",
            "docs/adr/0002-clean.md",
        ],
    );
    repo
}

#[test]
fn the_hook_blocks_a_commit_on_legacy_debt_by_default() {
    let repo = repo_with_debt("default");

    let output = git(&repo, &["commit", "-m", "test: clean record"]);

    assert!(
        !output.status.success(),
        "the full-bundle gate must still fail: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn the_hook_passes_the_same_commit_under_the_changed_only_opt_in() {
    let repo = repo_with_debt("changed-only");

    let output = git_with(
        &repo,
        &[("LIVING_DOCS_CHECK_CHANGED_ONLY", "1")],
        &["commit", "-m", "test: clean record"],
    );

    assert!(
        output.status.success(),
        "the scoped gate must pass: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
