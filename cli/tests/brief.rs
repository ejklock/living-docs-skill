use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("living-docs-brief-test-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_brief(docs: &Path, doc_type: &str, title: &str) -> Output {
    living_docs()
        .args([
            "--docs-dir",
            docs.to_str().unwrap(),
            "brief",
            doc_type,
            title,
        ])
        .output()
        .expect("failed to run living-docs brief")
}

#[test]
fn brief_scaffolds_a_numbered_record_with_filled_title_and_byte_identical_judgment_markers() {
    let docs = temp_dir("markers");

    let output = run_brief(&docs, "adr", "Choose X Over Y");

    assert!(output.status.success());
    let contents = fs::read_to_string(docs.join("adr/0001-choose-x-over-y.md")).unwrap();
    assert!(contents.contains("title: \"Choose X Over Y\""));
    assert!(contents.contains("# 0001. Choose X Over Y"));
    assert!(contents.contains("## Context\n\n<!-- judgment: context -->\n"));
    assert!(contents.contains("## Decision\n\n<!-- judgment: decision -->\n"));
    assert!(contents.contains("## Consequences\n\n<!-- judgment: consequences -->\n"));
    assert!(contents.contains("<!-- trail: motivated-by /research/NNNN-<slug>.md"));

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn brief_output_passes_check_for_every_supported_doc_type() {
    let docs = temp_dir("passes-check");
    for (doc_type, title) in [
        ("adr", "A Decision"),
        ("bdr", "A Behavior"),
        ("prd", "A Feature"),
        ("issue", "A Slice"),
    ] {
        assert!(run_brief(&docs, doc_type, title).status.success());
    }
    fs::write(
        docs.join("index.md"),
        "# Docs\n\n- [ADRs](/adr/index.md)\n- [BDRs](/bdr/index.md)\n- [PRDs](/prd/index.md)\n- [Issues](/issues/index.md)\n",
    )
    .unwrap();
    let indexed = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "index"])
        .output()
        .expect("failed to run living-docs index");
    assert!(indexed.status.success());

    let check = living_docs()
        .args(["check", docs.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs check");

    let stdout = String::from_utf8_lossy(&check.stdout).to_string();
    assert_eq!(check.status.code(), Some(0), "check failed:\n{stdout}");
    assert!(stdout.contains("no invariant violations"), "got: {stdout}");

    let _ = fs::remove_dir_all(&docs);
}

fn git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .expect("failed to run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn scratch_repo_with_two_commits(label: &str) -> PathBuf {
    let repo = temp_dir(label);
    git(&repo, &["init", "-q"]);
    fs::write(repo.join("kept.rs"), "fn kept() {}\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "first"]);
    fs::write(repo.join("kept.rs"), "fn kept() { }\n").unwrap();
    fs::write(repo.join("added.md"), "content\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "second"]);
    repo
}

#[test]
fn from_diff_lists_exactly_what_git_diff_name_only_prints_for_the_range() {
    let repo = scratch_repo_with_two_commits("from-diff");
    let docs = repo.join("docs");

    let output = living_docs()
        .args([
            "--docs-dir",
            docs.to_str().unwrap(),
            "brief",
            "issue",
            "Follow Up",
            "--from-diff",
            "HEAD~1..HEAD",
        ])
        .current_dir(&repo)
        .output()
        .expect("failed to run living-docs brief");
    assert!(output.status.success());

    let expected = Command::new("git")
        .args(["diff", "--name-only", "HEAD~1..HEAD"])
        .current_dir(&repo)
        .output()
        .expect("failed to run git diff");
    let contents = fs::read_to_string(docs.join("issues/0001-follow-up.md")).unwrap();
    assert!(contents.contains("Touched files (`git diff --name-only HEAD~1..HEAD`):"));
    for file in String::from_utf8_lossy(&expected.stdout)
        .lines()
        .filter(|l| !l.is_empty())
    {
        assert!(contents.contains(&format!("- `{file}`")), "missing {file}");
    }

    let _ = fs::remove_dir_all(&repo);
}

#[test]
fn an_invalid_from_diff_range_fails_with_a_clear_error_and_writes_no_file() {
    let repo = scratch_repo_with_two_commits("bad-range");
    let docs = repo.join("docs");

    let output = living_docs()
        .args([
            "--docs-dir",
            docs.to_str().unwrap(),
            "brief",
            "issue",
            "Broken",
            "--from-diff",
            "not-a-real-ref..HEAD",
        ])
        .current_dir(&repo)
        .output()
        .expect("failed to run living-docs brief");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not-a-real-ref"), "got: {stderr}");
    assert!(!docs.join("issues").exists());

    let _ = fs::remove_dir_all(&repo);
}

#[test]
fn brief_rejects_an_unsupported_doc_type() {
    let docs = temp_dir("unsupported");

    let output = run_brief(&docs, "constitution", "Root Rules");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported doc type"));

    let _ = fs::remove_dir_all(&docs);
}
