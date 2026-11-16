//! The distributed pre-commit hook in changed-only mode passes every staged
//! path to the binary as one argument, whatever characters the path holds.
//! Drives the real `skills/living-docs/hooks/pre-commit` script through `git commit`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod common;
use common::write;

type Test = Result<(), String>;

const BROKEN: &str = "---\ntype: ADR\ntitle: Broken\ndescription: A record with debt.\nowner: a@b.c\nstatus: Accepted\n---\n\n# 0001. Broken\n\n{{UNFILLED: debt}}\n";
const VALID: &str = "---\ntype: ADR\ntitle: Valid\ndescription: A record with no debt.\nowner: a@b.c\nstatus: Accepted\n---\n\n# 0002. Valid\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n";

fn hook_source() -> Result<PathBuf, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("the cli crate has no parent directory")?;
    Ok(root.join("skills/living-docs/hooks/pre-commit"))
}

fn git(repo: &Path, args: &[&str]) -> Result<Output, String> {
    git_with(repo, &[], args)
}

fn git_with(repo: &Path, vars: &[(&str, &str)], args: &[&str]) -> Result<Output, String> {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_living-docs"));
    let bin_dir = binary.parent().ok_or("the test binary has no directory")?;
    Command::new("git")
        .current_dir(repo)
        .env_clear()
        .env("HOME", repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("PATH", format!("{}:/usr/bin:/bin", bin_dir.display()))
        .env("LIVING_DOCS_CHECK_CHANGED_ONLY", "1")
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .envs(vars.iter().copied())
        .args(args)
        .output()
        .map_err(|e| format!("failed to run git: {e}"))
}

fn repo_with_hook(label: &str) -> Result<common::Scratch, String> {
    let repo = common::temp_bundle("pre-commit-spaces", label);
    git(&repo, &["init", "-q"])?;
    let hook = repo.join(".git/hooks/pre-commit");
    fs::copy(hook_source()?, &hook).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    write(&repo.join("docs"), "index.md", "# Docs\n");
    Ok(repo)
}

/// Writes each record under `docs/`, then stages them with the generated indexes.
fn stage(repo: &Path, records: &[(&str, &str)]) -> Result<(), String> {
    let docs = repo.join("docs");
    for (rel, body) in records {
        write(&docs, rel, body);
    }
    common::index_bundle(&docs);
    git(repo, &["add", "-A"])?;
    Ok(())
}

fn commit(repo: &Path) -> Result<(bool, String), String> {
    commit_with(repo, &[])
}

fn commit_with(repo: &Path, vars: &[(&str, &str)]) -> Result<(bool, String), String> {
    let out = git_with(repo, vars, &["commit", "-m", "test: record"])?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Ok((out.status.success(), text))
}

fn assert_blocked(records: &[(&str, &str)], label: &str, named: &str) -> Test {
    let repo = repo_with_hook(label)?;
    stage(&repo, records)?;
    let (ok, text) = commit(&repo)?;
    if ok || !text.contains(named) {
        return Err(format!("expected a failure naming {named}: ok={ok} {text}"));
    }
    Ok(())
}

#[test]
fn a_staged_broken_record_with_a_space_in_its_name_fails_the_commit() -> Test {
    assert_blocked(&[("adr/in dex.md", BROKEN)], "name", "in dex.md")
}

#[test]
fn a_staged_broken_record_under_a_directory_with_a_space_fails_the_commit() -> Test {
    assert_blocked(
        &[("my adrs/0001-broken.md", BROKEN)],
        "dir",
        "0001-broken.md",
    )
}

#[test]
fn a_staged_valid_record_with_a_space_passes_the_commit() -> Test {
    let repo = repo_with_hook("valid")?;
    write(
        &repo.join("docs"),
        "index.md",
        "# Docs\n\n* [Valid](<valid record.md>)\n",
    );
    stage(&repo, &[("valid record.md", VALID)])?;
    let (ok, text) = commit(&repo)?;
    if !ok {
        return Err(format!("a valid spaced record must pass: {text}"));
    }
    Ok(())
}

#[test]
fn a_spaced_and_an_unspaced_staged_record_are_both_checked() -> Test {
    assert_blocked(
        &[("adr/0002-valid.md", VALID), ("adr/in dex.md", BROKEN)],
        "mix-spaced",
        "in dex.md",
    )?;
    assert_blocked(
        &[
            ("adr/0001-broken.md", BROKEN),
            ("adr/0002-valid record.md", VALID),
        ],
        "mix-plain",
        "0001-broken.md",
    )
}

#[test]
fn a_path_with_a_quote_or_a_leading_dash_is_one_argument() -> Test {
    assert_blocked(&[("adr/it's broken.md", BROKEN)], "quote", "broken.md")?;
    assert_blocked(&[("adr/-broken.md", BROKEN)], "dash", "-broken.md")
}

#[test]
fn every_broken_staged_record_is_named_in_one_commit() -> Test {
    let repo = repo_with_hook("both-named")?;
    stage(
        &repo,
        &[
            ("adr/0001-broken.md", BROKEN),
            ("adr/in dex.md", BROKEN),
            ("adr/0002-valid.md", VALID),
        ],
    )?;
    let (ok, text) = commit(&repo)?;
    if ok || !text.contains("0001-broken.md") || !text.contains("in dex.md") {
        return Err(format!("both broken records must be named: ok={ok} {text}"));
    }
    Ok(())
}

#[test]
fn a_dash_led_record_at_the_top_of_a_root_bundle_is_checked_not_a_usage_error() -> Test {
    let repo = repo_with_hook("root-dash")?;
    write(&repo, "-broken.md", BROKEN);
    write(&repo, "index.md", "# Docs\n");
    git(&repo, &["add", "-A"])?;
    let (ok, text) = commit_with(&repo, &[("LIVING_DOCS_BUNDLE", ".")])?;
    if ok || !text.contains("-broken.md") || text.contains("Usage") {
        return Err(format!(
            "expected a check failure naming -broken.md: ok={ok} {text}"
        ));
    }
    Ok(())
}
