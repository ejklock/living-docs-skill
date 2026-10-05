//! The pre-commit hook's `LIVING_DOCS_CHECK_CHANGED_ONLY` opt-in (ADR 0062):
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

fn git(repo: &Path, args: &[&str]) -> Output {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_living-docs"));
    let bin_dir = binary.parent().expect("the test binary has a directory");
    let path = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    Command::new("git")
        .current_dir(repo)
        .env("PATH", path)
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .args(args)
        .output()
        .expect("failed to run git")
}

/// A git repo whose bundle carries one record with legacy debt and one clean
/// record, with the repo's real pre-commit hook installed and only the clean
/// record staged.
fn repo_with_debt(label: &str) -> PathBuf {
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

    let output = Command::new("git")
        .current_dir(&repo)
        .env("LIVING_DOCS_CHECK_CHANGED_ONLY", "1")
        .env(
            "PATH",
            format!(
                "{}:{}",
                PathBuf::from(env!("CARGO_BIN_EXE_living-docs"))
                    .parent()
                    .expect("the test binary has a directory")
                    .display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .args(["commit", "-m", "test: clean record"])
        .output()
        .expect("failed to run git commit");

    assert!(
        output.status.success(),
        "the scoped gate must pass: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
