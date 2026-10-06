use super::*;
use crate::test_support::MapStore;
use std::collections::BTreeMap;

fn store_of(files: &[(&str, &str)]) -> MapStore {
    let mut map = BTreeMap::new();
    for (path, contents) in files {
        map.insert(PathBuf::from(path), (*contents).to_string());
    }
    MapStore { files: map }
}

fn options(topic: Option<&str>, full: bool) -> Options {
    Options {
        topic: topic.map(str::to_string),
        full,
        contract: false,
    }
}

fn adr(number: &str, status: &str, supersedes: Option<&str>, body: &str) -> (String, String) {
    let mut fm = format!("---\ntype: ADR\ntitle: ADR {number}\ndescription: d\nstatus: {status}\n");
    if let Some(s) = supersedes {
        fm.push_str(&format!("supersedes: \"{s}\"\n"));
    }
    fm.push_str("---\n\n");
    (format!("docs/adr/{number}-x.md"), format!("{fm}{body}"))
}

#[test]
fn a_supersede_chain_collapses_to_the_head_with_a_lineage_line() {
    let a = adr("0131", "Superseded", None, "old");
    let b = adr("0133", "Superseded", Some("0131"), "middle");
    let c = adr("0143", "Accepted", Some("0133"), "the rule in force");
    let store = store_of(&[(&a.0, &a.1), (&b.0, &b.1), (&c.0, &c.1)]);
    let out = compile(&store, Path::new("docs"), &options(None, false));

    assert!(out.contains("ADR 0143"), "head must appear:\n{out}");
    assert!(
        out.contains("supersedes 0131 via 0133"),
        "lineage line:\n{out}"
    );
    assert_eq!(
        out.matches("- [ADR").count(),
        1,
        "only the head is a row:\n{out}"
    );
}

#[test]
fn superseded_and_deprecated_records_are_excluded() {
    let a = adr("0001", "Superseded", None, "gone");
    let b = adr("0002", "Deprecated", None, "retired");
    let c = adr("0003", "Accepted", None, "live");
    let store = store_of(&[(&a.0, &a.1), (&b.0, &b.1), (&c.0, &c.1)]);
    let out = compile(&store, Path::new("docs"), &options(None, false));
    assert!(out.contains("ADR 0003"));
    assert!(!out.contains("ADR 0001"));
    assert!(!out.contains("ADR 0002"));
}

#[test]
fn a_proposed_record_stays_in_force() {
    let proposed = adr("0001", "Proposed", None, "a live proposal");
    let store = store_of(&[(&proposed.0, &proposed.1)]);
    let out = compile(&store, Path::new("docs"), &options(None, false));
    assert!(
        out.contains("ADR 0001"),
        "a proposal is still active:\n{out}"
    );
}

#[test]
fn topic_filters_by_case_insensitive_term_across_title_and_body() {
    let a = adr("0001", "Accepted", None, "concerns the STORAGE backend");
    let b = adr("0002", "Accepted", None, "about mermaid diagrams");
    let store = store_of(&[(&a.0, &a.1), (&b.0, &b.1)]);
    let out = compile(&store, Path::new("docs"), &options(Some("storage"), false));
    assert!(out.contains("ADR 0001"));
    assert!(!out.contains("ADR 0002"));
}

#[test]
fn constitution_and_prd_group_above_adrs() {
    let store = store_of(&[
        (
            "docs/adr/0001-x.md",
            "---\ntype: ADR\ntitle: A\ndescription: d\nstatus: Accepted\n---\n\nb",
        ),
        (
            "docs/prd/0001-x.md",
            "---\ntype: PRD\ntitle: P\ndescription: d\nstatus: Accepted\n---\n\nb",
        ),
        (
            "docs/constitution.md",
            "---\ntype: Constitution\ntitle: C\ndescription: d\n---\n\nb",
        ),
    ]);
    let out = compile(&store, Path::new("docs"), &options(None, false));
    let c = out.find("[Constitution]").unwrap();
    let p = out.find("[PRD 0001]").unwrap();
    let a = out.find("[ADR 0001]").unwrap();
    assert!(
        c < p && p < a,
        "order should be constitution, PRD, ADR:\n{out}"
    );
}

#[test]
fn withheld_retired_records_open_the_view_with_their_count() {
    let a = adr("0001", "Superseded", None, "gone");
    let b = adr("0002", "Deprecated", None, "retired");
    let c = adr("0003", "Accepted", None, "live");
    let store = store_of(&[(&a.0, &a.1), (&b.0, &b.1), (&c.0, &c.1)]);
    let out = compile(&store, Path::new("docs"), &options(None, false));
    assert!(
        out.starts_with(
            "_Withheld 2 retired record(s) (superseded or deprecated): history only, never act on them._\n\n"
        ),
        "must open with the withheld count and a blank line:\n{out}"
    );
}

#[test]
fn no_withheld_records_leaves_the_view_unchanged() {
    let a = adr("0001", "Accepted", None, "live");
    let store = store_of(&[(&a.0, &a.1)]);
    let out = compile(&store, Path::new("docs"), &options(None, false));
    assert!(
        !out.contains("Withheld"),
        "no retired records means no withheld line:\n{out}"
    );
}

#[test]
fn full_prints_bodies_while_the_default_prints_one_line_entries() {
    let a = adr("0001", "Accepted", None, "## Context\n\nthe whole story");
    let store = store_of(&[(&a.0, &a.1)]);

    let index = compile(&store, Path::new("docs"), &options(None, false));
    assert!(
        !index.contains("the whole story"),
        "index omits bodies:\n{index}"
    );

    let full = compile(&store, Path::new("docs"), &options(None, true));
    assert!(
        full.contains("the whole story"),
        "full includes bodies:\n{full}"
    );
}
