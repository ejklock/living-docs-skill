//! The canonical writer gives every list and scalar a form that reads back as
//! the same value: a supersedes list in flow form, a scalar quoted exactly
//! when its plain form would read as something else, and a description line
//! that looks like a key never taking the key's place.

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
    let bundle = temp_bundle("supersede-canonical", label);
    for (rel, title) in [("0001-a", "A"), ("0002-b", "B"), ("0003-c", "C")] {
        write(&bundle, &format!("adr/{rel}.md"), &adr(title, ""));
    }
    bundle
}

fn read(bundle: &Path, rel: &str) -> String {
    fs::read_to_string(bundle.join(rel)).unwrap()
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

fn checked(bundle: &Path) -> Output {
    assert!(run(bundle, &["index"]).status.success());
    run_check(bundle)
}

fn snapshot(bundle: &Path, files: &[&str]) -> Vec<String> {
    files.iter().map(|f| read(bundle, f)).collect()
}

const REL: &str = "adr/0004-d.md";

/// Formats the bundle twice, requiring the second pass to change nothing and
/// check to find the record canonical; returns the file after the first pass.
fn formatted_twice(bundle: &Path, what: &str) -> String {
    assert!(fmt(bundle).status.success(), "{what}");
    let once = read(bundle, REL);
    assert!(fmt(bundle).status.success(), "{what}");
    assert_eq!(read(bundle, REL), once, "second fmt {what}");
    let verdict = stdout_of(&checked(bundle));
    assert!(
        !verdict.contains("non-canonical"),
        "check {what}: {verdict}"
    );
    once
}

/// Each `supersedes` spelling and the canonical flow list it reaches.
const CANONICAL: [(&str, &str); 17] = [
    ("[]", "[]"),
    ("[ 0001 , 0002 ]", "[0001, 0002]"),
    ("[0001, 0001]", "[0001, 0001]"),
    ("\n- 0001\n- 0002", "[0001, 0002]"),
    ("\n- 0001\n# note\n- 0002", "[0001, 0002]"),
    ("\n- 0001\n# c\n- \"\"", "[0001, \"\"]"),
    ("[0001, \"x]\", ~]", "[0001, \"x]\", ~]"),
    ("[0001, \"a, b\", ~]", "[0001, \"a, b\", ~]"),
    ("[0001, \"~\", ~]", "[0001, \"~\", ~]"),
    ("[0001, '']", "[0001, \"\"]"),
    ("[ 0001 ,\"\" ]", "[0001, \"\"]"),
    ("[0001, null]", "[0001, ~]"),
    ("[0001, \"\"] # note", "[0001, \"\"]"),
    ("\n- 0001\n- \"\"", "[0001, \"\"]"),
    ("\n  - 0001\n  - ~", "[0001, ~]"),
    ("[0001, abc]", "[0001, abc]"),
    ("\n- 0001\n\n- \"\"", "[0001, \"\"]"),
];

fn source_for(value: &str) -> String {
    let gap = if value.starts_with('\n') { "" } else { " " };
    format!("supersedes:{gap}{value}")
}

/// A list of record numbers is typed, so supersede accepts it.
fn is_record_number_list(expected: &str) -> bool {
    expected == "[]" || (expected.starts_with("[0001") && !expected.contains(['"', '~', 'a']))
}

fn assert_supersede_refused(bundle: &Path, source: &str) {
    let before = snapshot(bundle, &["adr/0002-b.md", REL]);
    let refused = run_supersede(bundle, "0002", "0004");
    assert_eq!(refused.status.code(), Some(2), "supersede {source}");
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("0004-d.md") && stderr.contains("supersedes"),
        "{stderr}"
    );
    assert_eq!(
        before,
        snapshot(bundle, &["adr/0002-b.md", REL]),
        "{source}"
    );
}

fn assert_canonical_row(index: usize, value: &str, list: &str) {
    let (source, expected) = (source_for(value), format!("supersedes: {list}"));
    let bundle = three_adrs(&format!("canonical-{index}"));
    write(&bundle, REL, &adr("D", &format!("{source}\n")));

    let once = formatted_twice(&bundle, &source);
    assert_eq!(
        frontmatter_line(&once, "supersedes").as_deref(),
        Some(expected.as_str()),
        "{source}"
    );

    write(&bundle, REL, &adr("D", &format!("{source}\n")));
    let set = run(&bundle, &["set", "0004", "description", "x"]);
    assert!(set.status.success(), "{source}");
    assert_eq!(
        frontmatter_line(&read(&bundle, REL), "supersedes").as_deref(),
        Some(expected.as_str()),
        "set {source}"
    );
    if !is_record_number_list(list) {
        assert_supersede_refused(&bundle, &source);
    }
}

#[test]
fn an_untyped_supersedes_value_is_written_in_canonical_flow_form_by_fmt_and_set() {
    for (index, (value, list)) in CANONICAL.iter().enumerate() {
        assert_canonical_row(index, value, list);
    }
}

fn lookalike(label: &str, description: &str) -> String {
    let bundle = three_adrs(label);
    let record = adr("D", "supersedes: [0001, \"x]\"]\n").replace("description: d.", description);
    write(&bundle, REL, &record);
    formatted_twice(&bundle, description)
}

#[test]
fn a_quoted_description_continued_at_column_zero_never_replaces_the_real_supersedes() {
    let after = lookalike("quoted", "description: \"first\nsupersedes: [9]\"");

    assert_eq!(
        frontmatter_line(&after, "description").as_deref(),
        Some("description: \"first supersedes: [9]\""),
        "{after}"
    );
    assert_eq!(
        frontmatter_line(&after, "supersedes").as_deref(),
        Some("supersedes: [0001, \"x]\"]"),
        "{after}"
    );
}

#[test]
fn a_block_description_holding_a_supersedes_line_never_replaces_the_real_supersedes() {
    let after = lookalike("block", "description: |-\n  first\n  supersedes: [9]");

    assert_eq!(
        frontmatter_line(&after, "description").as_deref(),
        Some("description: \"first\\nsupersedes: [9]\""),
        "{after}"
    );
    assert_eq!(
        frontmatter_line(&after, "supersedes").as_deref(),
        Some("supersedes: [0001, \"x]\"]"),
        "{after}"
    );
}

#[test]
fn a_block_description_with_a_line_break_is_written_quoted_with_its_break_escaped() {
    let after = lookalike("newline", "description: |\n  first\n  second");

    assert_eq!(
        frontmatter_line(&after, "description").as_deref(),
        Some("description: \"first\\nsecond\\n\""),
        "{after}"
    );
}

#[test]
fn a_stripped_block_description_with_an_inner_line_break_is_written_quoted() {
    let after = lookalike("stripped", "description: |-\n  first\n  second");

    assert_eq!(
        frontmatter_line(&after, "description").as_deref(),
        Some("description: \"first\\nsecond\""),
        "{after}"
    );
}

/// A tail value as written in the source and the line the writer emits.
const SCALARS: [(&str, &str); 9] = [
    ("alias: \"1.50\"", "alias: \"1.50\""),
    ("alias: \"~\"", "alias: \"~\""),
    ("alias: \"null\"", "alias: \"null\""),
    ("alias: \"0x10\"", "alias: \"0x10\""),
    ("alias: 1.5", "alias: 1.5"),
    ("priority: 3", "priority: 3"),
    ("flag: true", "flag: true"),
    ("alias: a, b", "alias: a, b"),
    ("alias: \"a: b\"", "alias: \"a: b\""),
];

#[test]
fn a_scalar_is_quoted_exactly_when_its_plain_form_would_read_as_another_string() {
    for (index, (source, expected)) in SCALARS.iter().enumerate() {
        let bundle = temp_bundle("supersede-canonical", &format!("scalar-{index}"));
        write(&bundle, REL, &adr("D", &format!("{source}\n")));

        let once = formatted_twice(&bundle, source);

        let key = source.split(':').next().unwrap();
        assert_eq!(
            frontmatter_line(&once, key).as_deref(),
            Some(*expected),
            "{once}"
        );
    }
}

#[test]
fn a_description_that_reads_as_a_number_when_plain_stays_quoted() {
    let bundle = temp_bundle("supersede-canonical", "description-number");
    write(
        &bundle,
        REL,
        &adr("D", "").replace("description: d.", "description: \"1.50\""),
    );

    let once = formatted_twice(&bundle, "description 1.50");

    assert_eq!(
        frontmatter_line(&once, "description").as_deref(),
        Some("description: \"1.50\"")
    );
}
