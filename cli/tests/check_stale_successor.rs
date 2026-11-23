//! `check` fails a successor link on a record that is not superseded, and the
//! message says whether the record still reads as in force.

use serde_json::Value;

mod common;
use common::{living_docs, write};

const STAMP: &str = "2026-11-20T10:00:00Z";

fn record(kind: &str, status: &str, body: &str) -> String {
    format!(
        "---\ntype: {kind}\ntitle: X\ndescription: A record.\nowner: a@b.c\nstatus: {status}\nsuperseded_by: 0002\ntimestamp: {STAMP}\n---\n\n# 0001. X\n\n{body}"
    )
}

fn adr(status: &str) -> String {
    record(
        "ADR",
        status,
        "## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n",
    )
}

fn issue(status: &str) -> String {
    record("Issue", status, "## Scope\n\ns.\n\n## Acceptance\n\na.\n")
}

fn successor_messages(dir: &str, contents: String) -> (Option<i32>, Vec<String>) {
    let bundle = common::temp_bundle("stale-successor", dir);
    write(
        &bundle,
        "index.md",
        &format!("# Index\n\n- [{dir}]({dir}/index.md)\n"),
    );
    let successor = adr("Accepted")
        .replace("superseded_by: 0002", "supersedes: 0001")
        .replace("# 0001.", "# 0002.");
    write(&bundle, &format!("{dir}/0001-x.md"), &contents);
    write(&bundle, "adr/0002-y.md", &successor);
    common::index_bundle(&bundle);
    let output = living_docs()
        .args(["check", bundle.to_str().unwrap(), "--json"])
        .output()
        .expect("failed to run living-docs check");
    let json: Value = serde_json::from_slice(&output.stdout).expect("check --json prints JSON");
    let found = json["violations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v["message"].as_str())
        .filter(|m| m.contains("not Superseded"))
        .map(str::to_owned)
        .collect();
    (output.status.code(), found)
}

#[test]
fn a_terminal_record_with_a_successor_fails_without_claiming_to_be_in_force() {
    for (dir, contents, status) in [
        ("adr", adr("Deprecated"), "Deprecated"),
        ("issues", issue("closed"), "closed"),
    ] {
        let (code, found) = successor_messages(dir, contents);
        assert_eq!(code, Some(1), "{status}: {found:?}");
        assert_eq!(found.len(), 1, "{status}: {found:?}");
        assert!(
            found[0].contains("0002") && found[0].contains(status),
            "{found:?}"
        );
        assert!(found[0].ends_with("(invariant 4)"), "{found:?}");
        assert!(!found[0].contains("in force"), "{found:?}");
    }
}

#[test]
fn a_live_record_with_a_successor_fails_and_reads_as_in_force() {
    let (code, found) = successor_messages("adr", adr("Accepted"));
    assert_eq!(code, Some(1), "{found:?}");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("0002") && found[0].contains("still reads as in force"));
    assert!(found[0].ends_with("(invariant 4)"), "{found:?}");
}

#[test]
fn a_status_terminal_only_for_another_type_still_reads_as_in_force() {
    let (code, found) = successor_messages("adr", adr("closed"));
    assert_eq!(code, Some(1), "{found:?}");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("still reads as in force"), "{found:?}");
}

#[test]
fn a_terminal_status_is_recognised_in_any_ascii_case() {
    let (code, found) = successor_messages("adr", adr("deprecated"));
    assert_eq!(code, Some(1), "{found:?}");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("deprecated status"), "{found:?}");
    assert!(!found[0].contains("in force"), "{found:?}");
}
