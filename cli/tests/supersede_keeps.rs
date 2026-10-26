//! A record that replaces several records carries all of them in its
//! `supersedes` list: a later supersede adds to it, a repeat adds nothing,
//! and neither `fmt` nor `set` ever drops it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

mod common;
use common::{living_docs, run_check, run_supersede, stdout_of, temp_bundle, write};

/// A record already in canonical shape, so `fmt` has nothing to rewrite
/// except what the test is probing; `extra` lands after `status`.
fn adr(title: &str, extra: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: {title}\ndescription: d.\nowner: o\nstatus: Accepted\n{extra}timestamp: 2026-01-01T00:00:00Z\n---\n\n# {title}\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n"
    )
}

fn three_adrs(label: &str) -> PathBuf {
    let bundle = temp_bundle("supersede-keeps", label);
    write(&bundle, "adr/0001-a.md", &adr("A", ""));
    write(&bundle, "adr/0002-b.md", &adr("B", ""));
    write(&bundle, "adr/0003-c.md", &adr("C", ""));
    bundle
}

fn read(bundle: &Path, rel: &str) -> String {
    fs::read_to_string(bundle.join(rel)).unwrap()
}

fn supersede_ok(bundle: &Path, old: &str, new: &str) {
    let output = run_supersede(bundle, old, new);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run(bundle: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap()])
        .args(args)
        .output()
        .expect("failed to run living-docs")
}

fn fmt(bundle: &Path) -> Output {
    living_docs()
        .args(["fmt", bundle.to_str().unwrap(), "--plain"])
        .output()
        .expect("failed to run living-docs fmt")
}

fn frontmatter_line(contents: &str, key: &str) -> Option<String> {
    contents
        .lines()
        .find(|line| line.starts_with(&format!("{key}:")))
        .map(str::to_owned)
}

#[test]
fn a_second_supersede_into_the_same_record_adds_to_its_supersedes_list() {
    let bundle = three_adrs("second");
    supersede_ok(&bundle, "0001", "0003");
    assert_eq!(
        frontmatter_line(&read(&bundle, "adr/0003-c.md"), "supersedes").as_deref(),
        Some("supersedes: 0001")
    );
    supersede_ok(&bundle, "0002", "0003");

    let head = read(&bundle, "adr/0003-c.md");
    assert_eq!(
        frontmatter_line(&head, "supersedes").as_deref(),
        Some("supersedes: [0001, 0002]"),
        "got: {head}"
    );
    for rel in ["adr/0001-a.md", "adr/0002-b.md"] {
        let old = read(&bundle, rel);
        assert_eq!(
            frontmatter_line(&old, "superseded_by").as_deref(),
            Some("superseded_by: 0003"),
            "got: {old}"
        );
    }
    let verdict = checked(&bundle);
    assert!(verdict.status.success(), "check: {}", stdout_of(&verdict));
}

#[test]
fn repeating_a_supersede_adds_no_duplicate_and_changes_no_file() {
    let bundle = three_adrs("repeat");
    supersede_ok(&bundle, "0001", "0003");
    supersede_ok(&bundle, "0002", "0003");
    let files = ["adr/0001-a.md", "adr/0002-b.md", "adr/0003-c.md"];
    let before = snapshot(&bundle, &files);

    supersede_ok(&bundle, "0001", "0003");

    assert_eq!(before, snapshot(&bundle, &files));
}

#[test]
fn fmt_and_set_keep_a_supersedes_list_and_a_scalar_supersedes_byte_identical() {
    let bundle = three_adrs("keep");
    write(
        &bundle,
        "adr/0004-d.md",
        &adr("D", "supersedes: [0001, 0002]\n"),
    );
    write(&bundle, "adr/0005-e.md", &adr("E", "supersedes: 0003\n"));
    let files = ["adr/0004-d.md", "adr/0005-e.md"];
    let before = snapshot(&bundle, &files);

    assert!(fmt(&bundle).status.success());
    assert_eq!(before, snapshot(&bundle, &files));

    assert!(run(&bundle, &["set", "0004", "description", "changed."])
        .status
        .success());
    let list = before[0].replace("description: d.", "description: changed.");
    assert_eq!(read(&bundle, files[0]), list);
}

#[test]
fn effective_lineage_names_every_ancestor_of_a_record_that_supersedes_two() {
    let bundle = three_adrs("lineage");
    supersede_ok(&bundle, "0001", "0003");
    supersede_ok(&bundle, "0002", "0003");

    let stdout = stdout_of(&run(&bundle, &["read", "--plain"]));
    assert!(stdout.contains("supersedes 0001, 0002"), "got:\n{stdout}");
}

fn snapshot(bundle: &Path, files: &[&str]) -> Vec<String> {
    files.iter().map(|f| read(bundle, f)).collect()
}

fn checked(bundle: &Path) -> Output {
    assert!(run(bundle, &["index"]).status.success());
    run_check(bundle)
}

/// Records whose list value has no flow form, with the key that names it.
const UNWRITABLE: [(&str, &str); 3] = [
    ("supersedes: [0001, [0002]]", "supersedes"),
    ("supersedes: [0001, {a: 1}]", "supersedes"),
    ("tags: [a, [b]]", "tags"),
];

/// Writes a refused record and a pending one beside it; returns the bundle.
fn bundle_with_refused(index: usize, source: &str) -> PathBuf {
    let bundle = three_adrs(&format!("unwritable-{index}"));
    write(&bundle, REFUSED, &adr("D", &format!("{source}\n")));
    let pending = adr("E", "")
        .replace("owner: o\n", "")
        .replace("status: Accepted\n", "status: Accepted\nowner: o\n");
    write(&bundle, PENDING, &pending);
    bundle
}

const REFUSED: &str = "adr/0004-d.md";
const PENDING: &str = "adr/0005-e.md";

fn assert_fmt_refuses_and_writes_the_rest(bundle: &Path, key: &str, source: &str) {
    let (refused_before, pending_before) = (read(bundle, REFUSED), read(bundle, PENDING));
    let output = fmt(bundle);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "fmt {source}");
    assert!(
        stderr.contains("0004-d.md") && stderr.contains(key),
        "{stderr}"
    );
    assert_eq!(read(bundle, REFUSED), refused_before, "fmt {source}");
    assert_ne!(read(bundle, PENDING), pending_before, "fmt {source}");
}

fn assert_check_names_the_hand_edit(bundle: &Path, key: &str, source: &str) {
    let verdict = stdout_of(&checked(bundle));
    let lines: Vec<&str> = verdict
        .lines()
        .filter(|l| l.contains("0004-d.md"))
        .collect();
    assert_eq!(lines.len(), 1, "check {source}: {verdict}");
    assert!(
        lines[0].contains(key) && lines[0].contains("by hand"),
        "{verdict}"
    );
    assert!(!lines[0].contains("living-docs fmt"), "{verdict}");
}

#[test]
fn a_value_with_no_flow_form_is_refused_by_set_fmt_and_check_without_a_write() {
    for (index, (source, key)) in UNWRITABLE.iter().enumerate() {
        let bundle = bundle_with_refused(index, source);
        let before = read(&bundle, REFUSED);

        let set = run(&bundle, &["set", "0004", "description", "x"]);
        assert_eq!(set.status.code(), Some(2), "set {source}");
        assert_eq!(read(&bundle, REFUSED), before, "set {source}");

        assert_fmt_refuses_and_writes_the_rest(&bundle, key, source);
        assert_check_names_the_hand_edit(&bundle, key, source);
    }
}
