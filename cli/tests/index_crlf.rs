mod common;

use common::{living_docs, run_check, temp_bundle, write};
use std::fs;
use std::path::Path;
use std::process::Output;

const ALPHA: &str =
    "---\ntype: ADR\ntitle: Alpha\nstatus: accepted\n---\n\n# 0001 — Alpha\n\nBody.\n";
const BETA: &str = "---\ntype: ADR\ntitle: Beta\nstatus: proposed\n---\n\n# 0002 — Beta\n\nBody.\n";
const GAMMA: &str =
    "---\ntype: Issue\ntitle: Gamma\nstatus: open\n---\n\n## 0001. Gamma\n\nBody.\n";

/// What the binary wrote for the all-LF bundle below before CRLF support.
const ADR_INDEX: &str = "# ADRs\n\n## Active\n\n* [0001 — Alpha](0001-alpha.md) - accepted\n* [0002 — Beta](0002-beta.md) - proposed\n";
const ISSUES_INDEX: &str = "# Issues\n\n## Open\n\n* [0001 — Gamma](0001-gamma.md) - open\n";
const ROOT_INDEX: &str = "# Docs\n* [ADRs](adr/)\n* [Issues](issues/)\n";

fn bundle(label: &str, crlf: bool) -> common::Scratch {
    let docs = temp_bundle("index-crlf", label);
    let eol = |text: &str| {
        if crlf {
            text.replace('\n', "\r\n")
        } else {
            text.to_string()
        }
    };
    write(&docs, "adr/0001-alpha.md", &eol(ALPHA));
    write(&docs, "adr/0002-beta.md", &eol(BETA));
    write(&docs, "issues/0001-gamma.md", &eol(GAMMA));
    write(&docs, "index.md", "# Docs\n");
    docs
}

fn index(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "index", "--plain"])
        .args(args)
        .output()
        .unwrap()
}

fn lf_text(docs: &Path, rel: &str) -> String {
    fs::read_to_string(docs.join(rel))
        .unwrap()
        .replace("\r\n", "\n")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn drift_in(docs: &Path) -> bool {
    String::from_utf8_lossy(&run_check(docs).stdout).contains("index differs")
}

fn tree(docs: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    for dir in ["", "adr", "issues"] {
        for entry in fs::read_dir(docs.join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                files.push((path.display().to_string(), fs::read(&path).unwrap()));
            }
        }
    }
    files.sort();
    files
}

#[test]
fn an_lf_bundle_is_indexed_exactly_as_before() {
    let docs = bundle("lf-oracle", false);
    assert!(index(&docs, &[]).status.success());
    assert_eq!(
        fs::read_to_string(docs.join("adr/index.md")).unwrap(),
        ADR_INDEX
    );
    assert_eq!(
        fs::read_to_string(docs.join("issues/index.md")).unwrap(),
        ISSUES_INDEX
    );
    assert_eq!(
        fs::read_to_string(docs.join("index.md")).unwrap(),
        ROOT_INDEX
    );
}

#[test]
fn a_crlf_bundle_gets_the_rows_of_its_lf_form_and_no_drift() {
    let docs = bundle("crlf-rows", true);
    let out = index(&docs, &[]);
    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(
        !stderr_of(&out).contains("no parseable 'title' frontmatter"),
        "{}",
        stderr_of(&out)
    );
    assert_eq!(lf_text(&docs, "adr/index.md"), ADR_INDEX);
    assert_eq!(lf_text(&docs, "issues/index.md"), ISSUES_INDEX);
    assert!(!drift_in(&docs), "check reports drift after index");
}

#[test]
fn set_on_a_crlf_record_leaves_an_index_that_index_and_check_accept() {
    let docs = bundle("crlf-set", true);
    assert!(index(&docs, &[]).status.success());
    let set = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "set", "adr/0002"])
        .args(["status", "Accepted", "--plain"])
        .output()
        .unwrap();
    assert!(set.status.success(), "{}", stderr_of(&set));
    assert_eq!(
        lf_text(&docs, "adr/index.md"),
        ADR_INDEX.replace("proposed", "Accepted")
    );
    assert!(!drift_in(&docs), "check reports drift after set");
    let before = tree(&docs);
    assert!(index(&docs, &[]).status.success());
    assert_eq!(tree(&docs), before);
}

#[test]
fn a_crlf_type_index_keeps_its_crlf_line_endings() {
    let docs = bundle("crlf-index-file", true);
    write(&docs, "adr/index.md", "# ADRs\r\n\r\n## Active\r\n\r\n");
    let out = index(&docs, &["adr"]);
    assert!(out.status.success(), "{}", stderr_of(&out));
    let raw = fs::read_to_string(docs.join("adr/index.md")).unwrap();
    assert_eq!(raw.replace("\r\n", "\n"), ADR_INDEX);
    assert_eq!(raw.matches('\n').count(), raw.matches("\r\n").count());
}

#[test]
fn a_record_with_mixed_endings_keeps_its_row_and_its_bytes() {
    let docs = bundle("mixed", false);
    let mixed = BETA.replacen('\n', "\r\n", 1);
    write(&docs, "adr/0002-beta.md", &mixed);
    let out = index(&docs, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert!(!stderr_of(&out).contains("no parseable 'title' frontmatter"));
    assert_eq!(lf_text(&docs, "adr/index.md"), ADR_INDEX);
    assert_eq!(lf_text(&docs, "issues/index.md"), ISSUES_INDEX);
    assert_eq!(
        fs::read_to_string(docs.join("adr/0002-beta.md")).unwrap(),
        mixed
    );
    let warning = format!(
        "living-docs index: {}: the record has mixed line endings (CRLF and bare LF); make them all LF or all CRLF first",
        docs.join("adr/0002-beta.md").display()
    );
    assert_eq!(stderr_of(&out).trim_end(), warning);
}

#[test]
fn the_mixed_endings_warning_follows_quiet_like_the_title_warning() {
    let docs = bundle("mixed-quiet", false);
    write(&docs, "adr/0002-beta.md", &BETA.replacen('\n', "\r\n", 1));
    write(&docs, "adr/0003-bare.md", "# 0003 — Bare\n");
    let loud = stderr_of(&index(&docs, &["adr"]));
    let quiet = stderr_of(&index(&docs, &["adr", "--quiet"]));
    for fragment in ["mixed line endings", "no parseable 'title' frontmatter"] {
        assert!(loud.contains(fragment), "{loud}");
        assert_eq!(quiet.contains(fragment), loud.contains(fragment), "{quiet}");
    }
}

const MIXED_TEXT: &str = "mixed line endings";

fn mixed_bundle(label: &str) -> common::Scratch {
    let docs = bundle(label, false);
    write(&docs, "adr/0002-beta.md", &BETA.replacen('\n', "\r\n", 1));
    docs
}

fn verb(docs: &Path, args: &[&str]) -> Output {
    let docs_dir = docs.to_str().unwrap();
    living_docs()
        .args(["--docs-dir", docs_dir])
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn only_the_index_verb_warns_about_a_mixed_record() {
    let docs = mixed_bundle("mixed-only-index");
    let root = docs.to_str().unwrap();
    let one = docs.join("adr/0001-alpha.md");
    let narrowed = [
        "check",
        "--plain",
        "--changed-files",
        one.to_str().unwrap(),
        "--",
        root,
    ];
    let runs = [
        verb(&docs, &["check", "--plain"]),
        verb(&docs, &narrowed),
        verb(&docs, &["new", "adr", "Third", "--plain"]),
        verb(&docs, &["set", "adr/0001", "status", "Proposed", "--plain"]),
        verb(&docs, &["supersede", "0001", "0003", "--plain"]),
    ];
    for run in &runs {
        assert!(!stderr_of(run).contains(MIXED_TEXT), "{}", stderr_of(run));
    }
    assert!(String::from_utf8_lossy(&runs[0].stdout).contains(MIXED_TEXT));
    assert!(stderr_of(&index(&docs, &[])).contains(MIXED_TEXT));
}

#[test]
fn a_mixed_index_or_non_record_file_draws_no_warning() {
    let docs = bundle("mixed-non-rows", false);
    let view = "---\ntype: Architecture View\ntitle: Overview\nkind: context\n---\n\n# Overview\n";
    write(&docs, "architecture/overview.md", view);
    for rel in ["adr/index.md", "adr/README.md", "architecture/index.md"] {
        write(&docs, rel, "# Heading\r\n\nText.\n");
    }
    let out = index(&docs, &[]);
    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(!stderr_of(&out).contains(MIXED_TEXT), "{}", stderr_of(&out));
}

#[test]
fn an_lf_bundle_prints_nothing_on_stderr() {
    let docs = bundle("lf-quiet-stderr", false);
    let out = index(&docs, &[]);
    assert_eq!(stderr_of(&out), "");
}

#[test]
fn a_crlf_architecture_view_is_indexed_like_its_lf_form() {
    let view = "---\ntype: Architecture View\ntitle: Overview\ndescription: The system.\nkind: context\n---\n\n# Overview\n\nBody.\n";
    let lf = bundle("view-lf", false);
    write(&lf, "architecture/overview.md", view);
    assert!(index(&lf, &["architecture"]).status.success());
    let expected = lf_text(&lf, "architecture/index.md");
    assert!(expected.contains("- context"), "{expected}");

    let crlf = bundle("view-crlf", true);
    write(
        &crlf,
        "architecture/overview.md",
        &view.replace('\n', "\r\n"),
    );
    write(
        &crlf,
        "architecture/index.md",
        &expected.replace('\n', "\r\n"),
    );
    let out = index(&crlf, &["architecture"]);
    assert!(out.status.success(), "{}", stderr_of(&out));
    assert_eq!(lf_text(&crlf, "architecture/index.md"), expected);
    assert!(!drift_in(&crlf), "check reports drift on the view index");
}
