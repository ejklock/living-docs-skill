mod common;

use common::{living_docs, run_new, temp_bundle};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

fn run(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap()])
        .args(args)
        .arg("--plain")
        .output()
        .unwrap()
}

fn two_adrs(label: &str) -> (PathBuf, PathBuf, PathBuf) {
    let docs = temp_bundle("set-read-back", label);
    assert!(run_new(&docs, "adr", "One").status.success());
    assert!(run_new(&docs, "adr", "Two").status.success());
    let one = docs.join("adr/0001-one.md");
    let two = docs.join("adr/0002-two.md");
    (docs, one, two)
}

/// Replaces the frontmatter line starting with `prefix` by a folded block
/// scalar spanning continuation lines, or adds one when the key is absent.
fn fold(path: &Path, key: &str) {
    let text = fs::read_to_string(path).unwrap();
    let prefix = format!("{key}:");
    let folded = format!("{key}: >\n  first line\n  second line");
    let edited = if text.lines().any(|line| line.starts_with(&prefix)) {
        text.lines()
            .map(|line| {
                if line.starts_with(&prefix) {
                    folded.clone()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    } else {
        text.replacen("\n---\n", &format!("\n{folded}\n---\n"), 1)
    };
    fs::write(path, edited).unwrap();
}

fn refused(docs: &Path, args: &[&str], touched: &[&Path]) -> String {
    let before: Vec<String> = touched
        .iter()
        .map(|p| fs::read_to_string(p).unwrap())
        .collect();
    let out = run(docs, args);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    for (path, original) in touched.iter().zip(&before) {
        assert_eq!(&fs::read_to_string(path).unwrap(), original);
    }
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn line(path: &Path, key: &str) -> String {
    let prefix = format!("{key}: ");
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .find_map(|l| l.strip_prefix(&prefix))
        .unwrap_or("")
        .to_string()
}

#[test]
fn refuse_description_set_over_a_folded_scalar() {
    let (docs, one, _) = two_adrs("fold-description");
    fold(&one, "description");
    let stderr = refused(&docs, &["set", "0001", "description", "x"], &[&one]);
    assert!(stderr.contains("description"), "got: {stderr}");
    assert!(stderr.contains("0001-one.md"), "got: {stderr}");
}

#[test]
fn refuse_owner_set_over_a_folded_scalar() {
    let (docs, one, _) = two_adrs("fold-owner");
    fold(&one, "owner");
    let stderr = refused(&docs, &["set", "0001", "owner", "x@y.z"], &[&one]);
    assert!(stderr.contains("owner"), "got: {stderr}");
}

#[test]
fn refuse_supersede_leaves_both_records_untouched_when_the_second_draft_fails() {
    let (docs, one, two) = two_adrs("fold-supersedes");
    fold(&two, "supersedes");
    let stderr = refused(&docs, &["supersede", "0001", "0002"], &[&one, &two]);
    assert!(stderr.contains("supersedes"), "got: {stderr}");
}

#[test]
fn refuse_leaves_ordinary_writes_accepted() {
    let (docs, one, two) = two_adrs("accepted");
    assert!(run(&docs, &["set", "0001", "status", "Accepted"])
        .status
        .success());
    assert_eq!(line(&one, "status"), "Accepted");
    assert!(run(&docs, &["set", "0001", "description", "A sentence."])
        .status
        .success());
    assert_eq!(line(&one, "description"), "A sentence.");
    assert!(run(&docs, &["set", "0001", "owner", "a@b.c"])
        .status
        .success());
    assert_eq!(line(&one, "owner"), "a@b.c");
    assert!(run(&docs, &["supersede", "0001", "0002"]).status.success());
    assert_eq!(line(&one, "status"), "Superseded");
    assert_eq!(line(&one, "superseded_by"), "0002");
    assert_eq!(line(&two, "supersedes"), "0001");
}

#[test]
fn refuse_leaves_titles_with_yaml_punctuation_written_as_before() {
    for (title, written, file) in [
        ("a: b", "\"a: b\"", "0001-a-b.md"),
        ("a #b", "\"a #b\"", "0001-a-b.md"),
        ("x{y", "x{y", "0001-x-y.md"),
        ("&x", "\"&x\"", "0001-x.md"),
    ] {
        let (docs, _, _) = two_adrs("titles");
        let out = run(&docs, &["set", "0001", "title", title]);
        assert!(out.status.success(), "{title}: {out:?}");
        assert_eq!(line(&docs.join("adr").join(file), "title"), written);
    }
}

fn fmt_check_ok(docs: &Path) -> bool {
    run(docs, &["fmt", "--check"]).status.success()
}

#[test]
fn comment_opener_values_are_quoted_and_stay_canonical() {
    let (docs, one, _) = two_adrs("comment-opener");
    for (key, value) in [("description", "x #y"), ("owner", "o #p")] {
        assert!(run(&docs, &["set", "0001", key, value]).status.success());
        assert_eq!(line(&one, key), format!("\"{value}\""));
        assert!(fmt_check_ok(&docs), "{key}: fmt --check failed");
    }
    assert!(run(&docs, &["set", "0001", "description", "a#b"])
        .status
        .success());
    assert_eq!(line(&one, "description"), "a#b");
}

#[test]
fn refuse_supersede_of_a_record_by_itself() {
    let (docs, one, _) = two_adrs("self-supersede");
    let index = docs.join("adr/index.md");
    let before = (
        fs::read_to_string(&one).unwrap(),
        fs::read_to_string(&index).unwrap(),
    );
    for (old, new) in [
        ("0001", "0001"),
        ("adr/0001", "adr/0001"),
        ("0001", "adr/0001"),
    ] {
        let out = run(&docs, &["supersede", old, new]);
        assert_eq!(out.status.code(), Some(2), "{old} {new}: {out:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("0001-one.md"), "got: {stderr}");
        assert_eq!(fs::read_to_string(&one).unwrap(), before.0);
        assert_eq!(fs::read_to_string(&index).unwrap(), before.1);
    }
}

#[path = "set_read_back/crlf.rs"]
mod crlf;
#[path = "set_read_back/mixed.rs"]
mod mixed;
