//! The `supersede-cycle` advisory: records whose `superseded_by` links loop
//! back on themselves. It never fails the gate and never judges a chain that
//! ends in a retired record or at a link that names no record.

use serde_json::Value;
use std::path::Path;

mod common;
use common::{living_docs, write};

/// An ADR whose frontmatter carries the given status and extra link lines.
fn adr(number: u32, status: &str, links: &str, eol: &str) -> String {
    let text = format!(
        "---\ntype: ADR\ntitle: Decision {number}\ndescription: Decision {number}.\nowner: Someone\nstatus: {status}\ntimestamp: 2026-11-20T10:00:00Z\n{links}---\n\n# {number:04}. Decision {number}\n\n## Context\n\nWhy.\n\n## Decision\n\nWhat.\n\n## Consequences\n\nSo.\n"
    );
    text.replace('\n', eol)
}

fn link(field: &str, number: u32) -> String {
    format!("{field}: {number:04}\n")
}

/// Writes ADR `number` with the given status and links.
fn put(bundle: &Path, number: u32, status: &str, links: &str, eol: &str) {
    let name = format!("adr/{number:04}-decision-{number}.md");
    write(bundle, &name, &adr(number, status, links, eol));
}

fn bundle(label: &str) -> common::Scratch {
    let bundle = common::temp_bundle("supersede-cycle", label);
    write(&bundle, "index.md", "# Index\n\n- [adr](adr/index.md)\n");
    bundle
}

/// Gives the hand-written records the callout and canonical frontmatter the
/// CLI writes, then indexes them, so only the links are left to judge.
fn settle(bundle: &Path) {
    let output = living_docs()
        .args(["manage", "fmt", "--plain", "--docs-dir"])
        .arg(bundle)
        .output()
        .expect("failed to run living-docs fmt");
    assert!(output.status.success(), "fmt failed");
    common::index_bundle(bundle);
}

fn check_json(bundle: &Path) -> (Option<i32>, Vec<String>) {
    let output = living_docs()
        .args(["check", "--json", "--"])
        .arg(bundle)
        .output()
        .expect("failed to run living-docs check");
    let json: Value = serde_json::from_slice(&output.stdout).expect("check --json prints JSON");
    let found = json["advisories"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["kind"] == "supersede-cycle")
        .map(|a| a["message"].as_str().unwrap().to_owned())
        .collect();
    (output.status.code(), found)
}

/// A ring of `size` superseded records, each naming the next.
fn ring(label: &str, size: u32, eol: &str) -> common::Scratch {
    let bundle = bundle(label);
    for number in 1..=size {
        let next = number % size + 1;
        let prior = (number + size - 2) % size + 1;
        let links = format!(
            "{}{}",
            link("supersedes", prior),
            link("superseded_by", next)
        );
        put(&bundle, number, "Superseded", &links, eol);
    }
    settle(&bundle);
    bundle
}

fn names_all(message: &str, numbers: &[u32]) -> bool {
    numbers
        .iter()
        .all(|n| message.contains(&format!("{n:04}-decision-{n}.md")))
}

#[test]
fn a_two_record_cycle_names_both_records_once() {
    let (code, found) = check_json(&ring("two", 2, "\n"));
    assert_eq!(code, Some(0));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(names_all(&found[0], &[1, 2]), "{found:?}");
}

#[test]
fn a_three_record_cycle_names_all_three_once() {
    let (code, found) = check_json(&ring("three", 3, "\n"));
    assert_eq!(code, Some(0));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(names_all(&found[0], &[1, 2, 3]), "{found:?}");
}

#[test]
fn a_cycle_in_crlf_records_is_read_like_one_in_lf_records() {
    let (code, found) = check_json(&ring("crlf", 2, "\r\n"));
    assert_eq!(code, Some(0));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(names_all(&found[0], &[1, 2]), "{found:?}");
}

#[test]
fn a_self_loop_and_a_tail_into_a_cycle_are_two_advisories_naming_only_cycle_members() {
    let bundle = bundle("tail");
    let both = |from: &str, to: u32| format!("{from}\nsuperseded_by: {to:04}\n");
    put(&bundle, 1, "Superseded", &both("supersedes: 0001", 1), "\n");
    put(&bundle, 5, "Superseded", &link("superseded_by", 2), "\n");
    put(&bundle, 2, "Superseded", &both("supersedes: 0005", 3), "\n");
    put(
        &bundle,
        3,
        "Superseded",
        &both("supersedes: [0002, 0004]", 4),
        "\n",
    );
    put(&bundle, 4, "Superseded", &both("supersedes: 0003", 3), "\n");
    settle(&bundle);
    let (code, found) = check_json(&bundle);
    assert_eq!(code, Some(0));
    assert_eq!(found.len(), 2, "{found:?}");
    let own = found
        .iter()
        .find(|m| names_all(m, &[1]))
        .expect("self-loop");
    assert!(!names_all(own, &[2]) && !names_all(own, &[3]), "{own}");
    let ring = found.iter().find(|m| names_all(m, &[3, 4])).expect("ring");
    assert!(!names_all(ring, &[1]), "{ring}");
    assert!(
        found
            .iter()
            .all(|m| !names_all(m, &[2]) && !names_all(m, &[5])),
        "{found:?}"
    );
}

#[test]
fn a_chain_ending_in_a_retired_record_raises_nothing() {
    let bundle = bundle("retired");
    put(&bundle, 1, "Superseded", &link("superseded_by", 2), "\n");
    put(&bundle, 2, "Deprecated", &link("supersedes", 1), "\n");
    settle(&bundle);
    assert_eq!(check_json(&bundle), (Some(0), vec![]));
}

#[test]
fn a_dangling_successor_raises_no_cycle_advisory() {
    let bundle = bundle("dangling");
    put(&bundle, 1, "Superseded", &link("superseded_by", 9), "\n");
    settle(&bundle);
    let (_, found) = check_json(&bundle);
    assert_eq!(found, Vec::<String>::new());
}
