//! The `state-machine` advisory: a state view whose diagram has an unreachable
//! state, a dead end or several initial transitions. It never fails the gate.

use serde_json::Value;
use std::path::Path;

mod common;
use common::{living_docs, write};

const VIEW: &str = "architecture/lifecycle.md";

fn view(kind: &str, fence: &str) -> String {
    format!(
        "---\ntype: Architecture View\ntitle: Lifecycle\ndescription: The lifecycle.\nkind: {kind}\ntimestamp: 2026-11-20T10:00:00Z\n---\n\n# Lifecycle\n\n```mermaid\n{fence}```\n\n## Drift instrument\n\nDrift caught by: the check.\n"
    )
}

fn bundle(label: &str, contents: &str) -> common::Scratch {
    let bundle = common::temp_bundle("state-machine", label);
    write(
        &bundle,
        "index.md",
        "# Index\n\n- [architecture](architecture/index.md)\n",
    );
    write(&bundle, VIEW, contents);
    common::index_bundle(&bundle);
    bundle
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
        .filter(|a| a["kind"] == "state-machine")
        .map(|a| a["message"].as_str().unwrap().to_owned())
        .collect();
    (output.status.code(), found)
}

fn judged(label: &str, kind: &str, fence: &str) -> (Option<i32>, Vec<String>) {
    check_json(&bundle(label, &view(kind, fence)))
}

#[test]
fn a_complete_lifecycle_raises_nothing() {
    let got = judged(
        "ok",
        "state",
        "stateDiagram-v2\n[*] --> A\nA --> B\nB --> [*]\n",
    );
    assert_eq!(got, (Some(0), vec![]));
}

#[test]
fn an_isolated_pair_is_unreachable_and_its_sink_a_dead_end() {
    let (code, found) = judged(
        "isolated",
        "state",
        "stateDiagram-v2\n[*] --> A\nA --> [*]\nC --> D\n",
    );
    assert_eq!(code, Some(0));
    let has = |needle: &str| found.iter().any(|m| m.contains(needle));
    assert!(has("state 'C' is unreachable"), "{found:?}");
    assert!(has("state 'D' is unreachable"), "{found:?}");
    assert!(has("state 'D' has no outgoing transition"), "{found:?}");
    assert_eq!(found.len(), 3, "{found:?}");
}

#[test]
fn a_state_without_an_exit_is_a_dead_end_naming_it() {
    let (code, found) = judged("dead", "state", "stateDiagram-v2\n[*] --> A\nA --> B\n");
    assert_eq!(code, Some(0));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("state 'B' has no outgoing transition"),
        "{found:?}"
    );
    assert!(found[0].contains("add 'B --> [*]'"), "{found:?}");
}

#[test]
fn a_composite_whose_inner_end_leads_out_raises_nothing() {
    let fence = "stateDiagram-v2\n[*] --> Run\nstate Run {\n[*] --> I\nI --> [*]\n}\nRun --> Done\nDone --> [*]\n";
    assert_eq!(judged("composite", "state", fence), (Some(0), vec![]));
}

#[test]
fn two_root_initial_transitions_are_reported() {
    let fence = "stateDiagram-v2\n[*] --> X\n[*] --> Y\nX --> [*]\nY --> [*]\n";
    let (code, found) = judged("two-initial", "state", fence);
    assert_eq!(code, Some(0));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("scope 'root' has 2 initial transitions"),
        "{found:?}"
    );
}

#[test]
fn concurrent_regions_raise_one_not_proved_advisory() {
    let fence = "stateDiagram-v2\n[*] --> Run\nstate Run {\n[*] --> A\nA --> [*]\n--\n[*] --> B\nB --> [*]\n}\nRun --> [*]\n";
    let (code, found) = judged("concurrent", "state", fence);
    assert_eq!(code, Some(0));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("not proved"), "{found:?}");
}

#[test]
fn a_note_on_a_state_with_no_exit_still_yields_its_dead_end() {
    let fence = "stateDiagram-v2\n[*] --> A\nA --> B\nnote right of B : terminal?\n";
    let (code, found) = judged("note-dead-end", "state", fence);
    assert_eq!(code, Some(0));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("state 'B' has no outgoing transition"),
        "{found:?}"
    );
}

#[test]
fn a_composite_entered_through_an_inner_state_raises_nothing() {
    let fence = "stateDiagram-v2\n[*] --> Pending\nPending --> Picking\nstate Processing {\n[*] --> Picking\nPicking --> Packing\nPacking --> [*]\n}\nProcessing --> [*]\n";
    assert_eq!(judged("inner-entry", "state", fence), (Some(0), vec![]));
}

#[test]
fn a_note_is_ignored() {
    let fence = "stateDiagram-v2\n[*] --> A\nA --> [*]\nnote right of A : remember\n";
    assert_eq!(judged("note", "state", fence), (Some(0), vec![]));
}

#[test]
fn the_same_diagram_in_a_view_of_another_kind_raises_nothing() {
    let fence = "stateDiagram-v2\n[*] --> A\nA --> B\n";
    assert_eq!(judged("component", "component", fence), (Some(0), vec![]));
}
