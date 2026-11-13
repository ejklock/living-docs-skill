//! The distributed pre-commit hook's opt-ins: `LIVING_DOCS_CHECK_CHANGED_ONLY`
//! lets a bundle carrying legacy debt still gate the records a commit touches,
//! and `LIVING_DOCS_REQUIRE_OWNER` requires an owner on them.
//! Drives the real `skills/living-docs/hooks/pre-commit` script through `git commit`.

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
        .join("skills/living-docs/hooks/pre-commit")
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

fn adr_path(name: &str) -> String {
    format!("docs/{}/{name}", "adr")
}

fn git(repo: &Path, args: &[&str]) -> Output {
    git_with(repo, &[], args)
}

/// A git repo with the repo's real pre-commit hook installed.
fn repo_with_hook(label: &str) -> common::Scratch {
    let repo = common::temp_bundle("pre-commit", label);
    git(&repo, &["init", "-q"]);
    let hook = repo.join(".git/hooks/pre-commit");
    fs::copy(hook_source(), &hook).expect("the repo ships a pre-commit hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).expect("hook is executable");
    }
    repo
}

/// Writes the bundle with a clean record, plus the legacy-debt record when
/// `debt` is set, and stages only the clean record.
fn stage_bundle(repo: &Path, debt: bool) {
    let docs = repo.join("docs");
    let debt_entry = "* [0001 — Legacy debt](0001-legacy-debt.md) - Accepted\n";
    let index = format!(
        "# ADRs\n\n{}* [0002 — Clean](0002-clean.md) - Accepted\n",
        if debt { debt_entry } else { "" }
    );
    write(&docs, "index.md", "# Docs\n\n* [ADRs](adr/)\n");
    write(&docs, "adr/index.md", &index);
    if debt {
        write(&docs, "adr/0001-legacy-debt.md", LEGACY_DEBT);
    }
    write(&docs, "adr/0002-clean.md", CLEAN);
    common::index_bundle(&docs);
    let clean = adr_path("0002-clean.md");
    git(
        repo,
        &["add", "docs/index.md", &adr_path("index.md"), &clean],
    );
}

/// A repo whose bundle carries one record with legacy debt and one clean
/// record, with only the clean record staged.
fn repo_with_debt(label: &str) -> common::Scratch {
    let repo = repo_with_hook(label);
    stage_bundle(&repo, true);
    repo
}

/// A repo whose bundle has no defect at all, with the clean record staged.
fn repo_without_debt(label: &str) -> common::Scratch {
    let repo = repo_with_hook(label);
    stage_bundle(&repo, false);
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

const OWNERLESS: &str = "---\ntype: ADR\ntitle: Ownerless\ndescription: A record with no owner.\nstatus: Accepted\n---\n\n# 0003. Ownerless\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n";
const BOTH: [(&str, &str); 2] = [
    ("LIVING_DOCS_CHECK_CHANGED_ONLY", "1"),
    ("LIVING_DOCS_REQUIRE_OWNER", "1"),
];

/// Stages an ADR that has no owner on top of the repo's staged clean record.
fn stage_ownerless(repo: &Path) {
    let docs = repo.join("docs");
    write(&docs, "adr/0003-ownerless.md", OWNERLESS);
    common::index_bundle(&docs);
    git(
        repo,
        &["add", &adr_path("0003-ownerless.md"), &adr_path("index.md")],
    );
}

fn commit_with(repo: &Path, vars: &[(&str, &str)]) -> Output {
    git_with(repo, vars, &["commit", "-m", "test: record"])
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn a_staged_record_without_an_owner_fails_when_owners_are_required() {
    let repo = repo_with_debt("owner-required");
    stage_ownerless(&repo);

    let output = commit_with(&repo, &BOTH);

    let text = combined(&output);
    assert!(!output.status.success(), "ownerless record passed: {text}");
    assert!(
        text.contains("0003-ownerless"),
        "wrong record named: {text}"
    );
    assert!(
        !text.contains("0001-legacy-debt"),
        "unstaged debt must stay ignored: {text}"
    );
}

#[test]
fn a_staged_record_without_an_owner_passes_unless_the_variable_is_exactly_one() {
    for value in [None, Some("0"), Some("yes")] {
        let repo = repo_with_debt("owner-optional");
        stage_ownerless(&repo);
        let mut vars = vec![("LIVING_DOCS_CHECK_CHANGED_ONLY", "1")];
        if let Some(v) = value {
            vars.push(("LIVING_DOCS_REQUIRE_OWNER", v));
        }

        let output = commit_with(&repo, &vars);

        assert!(
            output.status.success(),
            "owner variable {value:?} must not require owners: {}",
            combined(&output)
        );
    }
}

#[test]
fn owners_are_required_over_the_whole_bundle_without_the_changed_only_opt_in() {
    let repo = repo_without_debt("owner-full");
    stage_ownerless(&repo);

    let output = commit_with(&repo, &[("LIVING_DOCS_REQUIRE_OWNER", "1")]);

    let text = combined(&output);
    assert!(!output.status.success(), "ownerless record passed: {text}");
    assert!(
        text.contains("0003-ownerless"),
        "wrong record named: {text}"
    );
}

#[test]
fn the_whole_bundle_passes_an_ownerless_record_unless_the_variable_is_exactly_one() {
    for value in [None, Some("0"), Some("yes")] {
        let repo = repo_without_debt("owner-full-optional");
        stage_ownerless(&repo);
        let vars: Vec<(&str, &str)> = value
            .map(|v| ("LIVING_DOCS_REQUIRE_OWNER", v))
            .into_iter()
            .collect();

        let output = commit_with(&repo, &vars);

        assert!(
            output.status.success(),
            "owner variable {value:?} must not require owners: {}",
            combined(&output)
        );
    }
}

#[test]
fn both_opt_ins_pass_a_commit_that_stages_nothing_in_the_bundle() {
    let repo = repo_with_debt("nothing-staged");
    git(&repo, &["reset", "-q"]);
    write(&repo, "NOTES.md", "notes\n");
    git(&repo, &["add", "NOTES.md"]);

    let output = commit_with(&repo, &BOTH);

    assert!(output.status.success(), "{}", combined(&output));
}
