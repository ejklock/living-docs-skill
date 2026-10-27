//! `set <ref> number <NNNN|next>` renumbers one record with its heading, its
//! in-bundle links and its type index.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

mod common;
use common::{index_bundle, living_docs, run_check, run_new, temp_bundle, write};

type Tree = BTreeMap<PathBuf, Vec<u8>>;

const OLD_PATH: &str = "issues/0007-b.md";

fn issue(number: &str, title: &str, status: &str, extra: &str) -> String {
    format!(
        "---\ntype: Issue\ntitle: {title}\ndescription: d\nstatus: {status}\n{extra}timestamp: 2026-10-24T10:00:00Z\n---\n\n## {number}. {title}\n\nBody.\n\n### Scope\n\ns\n\n### Acceptance\n\n- a\n"
    )
}

const LINKER: &str = "---\ntype: Issue\ntitle: Linker\ndescription: d\nstatus: open\ntimestamp: 2026-10-24T10:00:00Z\n---\n\n## 0005. Linker\n\nSibling [b](0007-b.md), absolute [b](/issues/0007-b.md), reference [b][ref].\n\nMentions 0007-a.md and 0007 in prose.\n\n### Scope\n\ns\n\n### Acceptance\n\n- a\n\n[ref]: 0007-b.md\n";

const LINKER_MOVED: &str = "---\ntype: Issue\ntitle: Linker\ndescription: d\nstatus: open\ntimestamp: 2026-10-24T10:00:00Z\n---\n\n## 0005. Linker\n\nSibling [b](0010-b.md), absolute [b](/issues/0010-b.md), reference [b][ref].\n\nMentions 0007-a.md and 0007 in prose.\n\n### Scope\n\ns\n\n### Acceptance\n\n- a\n\n[ref]: 0010-b.md\n";

const ADR_MOVED: &str = "---\ntype: ADR\ntitle: Cache\ndescription: d\nowner: o\nstatus: Accepted\ntimestamp: 2026-10-24T10:00:00Z\n---\n\n# 0001. Cache\n\nSee [b](../issues/0010-b.md).\n\n## Context\n\nc\n\n## Decision\n\nd\n\n## Consequences\n\nq\n";

/// A record that links to itself and to its neighbours in every link form.
fn self_linking(number: &str, title: &str, own_name: &str) -> String {
    format!(
        "---\ntype: Issue\ntitle: {title}\ndescription: d\nstatus: open\ntimestamp: 2026-10-24T10:00:00Z\n---\n\n## {number}. {title}\n\nSelf [me]({own_name}#scope), [dot](./{own_name}), [abs](/issues/{own_name}), [def][d].\n\nSibling [a](0007-a.md) and [adr](../adr/0001-cache.md).\n\n### Scope\n\ns\n\n### Acceptance\n\n- a\n\n[d]: {own_name}\n"
    )
}

const ADR_LINKER: &str = "---\ntype: ADR\ntitle: Cache\ndescription: d\nowner: o\nstatus: Accepted\ntimestamp: 2026-10-24T10:00:00Z\n---\n\n# 0001. Cache\n\nSee [b](../issues/0007-b.md).\n\n## Context\n\nc\n\n## Decision\n\nd\n\n## Consequences\n\nq\n";

fn bundle(label: &str) -> common::Scratch {
    let docs = temp_bundle("set-number", label);
    write(&docs, "issues/0005-linker.md", LINKER);
    write(&docs, "issues/0007-a.md", &issue("0007", "a", "open", ""));
    write(&docs, OLD_PATH, &issue("0007", "b", "open", ""));
    write(&docs, "issues/0009-x.md", &issue("0009", "x", "open", ""));
    write(&docs, "adr/0001-cache.md", ADR_LINKER);
    index_bundle(&docs);
    docs
}

fn snapshot(root: &Path) -> Tree {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.insert(path.clone(), fs::read(&path).unwrap());
            }
        }
    }
    files
}

fn number(docs: &Path, reference: &str, value: &str) -> Output {
    living_docs()
        .arg("--docs-dir")
        .arg(docs)
        .args(["set", reference, "number", value, "--plain"])
        .output()
        .unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn read(docs: &Path, rel: &str) -> String {
    fs::read_to_string(docs.join(rel)).unwrap()
}

fn assert_refused_unchanged(docs: &Path, reference: &str, value: &str) {
    let before = snapshot(docs);
    let output = number(docs, reference, value);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{value}: {}",
        stderr(&output)
    );
    assert_eq!(snapshot(docs), before, "{value}: the tree must not change");
}

#[test]
fn next_renames_the_record_and_rewrites_its_heading_and_every_link() {
    let docs = bundle("next");
    let before = snapshot(&docs);
    let output = number(&docs, OLD_PATH, "next");
    assert!(output.status.success(), "{}", stderr(&output));

    assert!(!docs.join(OLD_PATH).exists());
    let moved = read(&docs, "issues/0010-b.md");
    assert!(moved.contains("## 0010. b"), "{moved}");
    assert!(moved.contains("title: b"), "{moved}");

    assert_eq!(read(&docs, "issues/0005-linker.md"), LINKER_MOVED);
    assert_eq!(read(&docs, "adr/0001-cache.md"), ADR_MOVED);
    let touched = [
        OLD_PATH,
        "issues/0010-b.md",
        "issues/0005-linker.md",
        "adr/0001-cache.md",
        "issues/index.md",
    ];
    for (path, bytes) in &before {
        let rel = path.strip_prefix(&docs).unwrap().to_str().unwrap();
        if !touched.contains(&rel) {
            assert_eq!(&fs::read(path).unwrap(), bytes, "{rel} must not change");
        }
    }

    let check = run_check(&docs);
    assert!(check.status.success(), "{:?}", check);
}

#[test]
fn index_after_a_renumbering_writes_nothing() {
    let docs = bundle("index-stable");
    assert!(number(&docs, OLD_PATH, "next").status.success());
    let before = snapshot(&docs);
    index_bundle(&docs);
    assert_eq!(snapshot(&docs), before);
}

#[test]
fn an_explicit_free_number_is_used_as_given() {
    let docs = bundle("explicit");
    let output = number(&docs, OLD_PATH, "0012");
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(read(&docs, "issues/0012-b.md").contains("## 0012. b"));
}

#[test]
fn a_taken_number_or_a_malformed_one_is_refused() {
    for value in ["0009", "12", "00012", "abcd", "0000"] {
        let docs = bundle(&format!("refused-{value}"));
        assert_refused_unchanged(&docs, OLD_PATH, value);
    }
}

#[test]
fn a_record_with_no_duplicate_cannot_take_its_own_number() {
    let docs = bundle("own-number");
    assert_refused_unchanged(&docs, "issues/0009-x.md", "0009");
}

#[test]
fn a_closed_record_renumbers() {
    let docs = bundle("closed");
    write(&docs, OLD_PATH, &issue("0007", "b", "closed", ""));
    index_bundle(&docs);
    let output = number(&docs, OLD_PATH, "next");
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(read(&docs, "issues/0010-b.md").contains("status: closed"));
}

fn assert_link_refused(label: &str, own: &str, other: &str, field: &str) {
    let docs = bundle("link");
    write(&docs, OLD_PATH, &issue("0007", "b", "open", own));
    write(
        &docs,
        "issues/0003-c.md",
        &issue("0003", "c", "open", other),
    );
    index_bundle(&docs);
    let before = snapshot(&docs);
    let output = number(&docs, OLD_PATH, "next");
    assert_eq!(output.status.code(), Some(2), "{label}");
    assert!(
        stderr(&output).contains(field),
        "{label}: {}",
        stderr(&output)
    );
    assert_eq!(snapshot(&docs), before, "{label}");
}

#[test]
fn a_record_in_a_supersede_link_is_refused_naming_the_link() {
    let own = [
        "supersedes: 0003\n",
        "supersedes: [0003, 0005]\n",
        "superseded_by: 0003\n",
    ];
    for line in own {
        let field = line.split(':').next().unwrap();
        assert_link_refused(&format!("own {line}"), line, "", field);
    }
    let named = [
        "supersedes: 0007\n",
        "supersedes: [0003, 0007]\n",
        "superseded_by: 0007\n",
    ];
    for line in named {
        let field = line.split(':').next().unwrap();
        assert_link_refused(&format!("other {line}"), "", line, field);
    }
}

#[test]
fn an_empty_or_unrelated_supersede_list_does_not_block_a_renumbering() {
    for (own, other) in [("supersedes: []\n", ""), ("", "supersedes: [0003, 0005]\n")] {
        let docs = bundle(&format!("free {own}{other}"));
        write(&docs, OLD_PATH, &issue("0007", "b", "open", own));
        write(
            &docs,
            "issues/0003-c.md",
            &issue("0003", "c", "open", other),
        );
        index_bundle(&docs);
        let output = number(&docs, OLD_PATH, "next");
        assert!(output.status.success(), "{own}{other}: {}", stderr(&output));
    }
}

#[test]
fn an_architecture_view_and_the_constitution_are_refused_unchanged() {
    let docs = temp_bundle("set-number", "named");
    let view = living_docs()
        .arg("--docs-dir")
        .arg(&docs)
        .args(["new", "view", "Rings", "--kind", "component"])
        .output()
        .unwrap();
    assert!(view.status.success());
    assert!(run_new(&docs, "constitution", "Product").status.success());
    assert_refused_unchanged(&docs, "view/rings", "0002");
    assert_refused_unchanged(&docs, "constitution", "0002");
}

#[test]
fn a_crlf_record_keeps_crlf_after_renumbering() {
    let docs = bundle("crlf");
    let crlf = issue("0007", "b", "open", "").replace('\n', "\r\n");
    write(&docs, OLD_PATH, &crlf);
    index_bundle(&docs);
    let output = number(&docs, OLD_PATH, "next");
    assert!(output.status.success(), "{}", stderr(&output));
    let moved = read(&docs, "issues/0010-b.md");
    assert!(moved.contains("## 0010. b\r\n"), "{moved:?}");
    assert!(!moved.replace("\r\n", "").contains('\n'), "{moved:?}");
}

#[test]
fn a_record_keeps_working_links_to_itself_after_a_renumbering() {
    let docs = bundle("self-links-number");
    write(&docs, OLD_PATH, &self_linking("0007", "b", "0007-b.md"));
    index_bundle(&docs);
    let output = number(&docs, OLD_PATH, "next");
    assert!(output.status.success(), "{}", stderr(&output));
    let expected = self_linking("0010", "b", "0010-b.md");
    assert_eq!(read(&docs, "issues/0010-b.md"), expected);
    let check = run_check(&docs);
    assert!(check.status.success(), "{check:?}");
}

#[test]
fn a_record_keeps_working_links_to_itself_after_a_retitle() {
    let docs = temp_bundle("set-number", "self-links-title");
    let own = "issues/0009-x.md";
    write(&docs, "issues/0007-a.md", &issue("0007", "a", "open", ""));
    write(&docs, own, &self_linking("0009", "x", "0009-x.md"));
    write(
        &docs,
        "adr/0001-cache.md",
        &ADR_LINKER.replace("0007-b", "0007-a"),
    );
    index_bundle(&docs);
    let output = living_docs()
        .arg("--docs-dir")
        .arg(&docs)
        .args(["set", own, "title", "Ex", "--plain"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let expected = self_linking("0009", "Ex", "0009-ex.md");
    assert_eq!(read(&docs, "issues/0009-ex.md"), expected);
    let check = run_check(&docs);
    assert!(check.status.success(), "{check:?}");
}
