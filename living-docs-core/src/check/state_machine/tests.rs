use super::{check_state_machine, findings_in, Reporter};
use crate::test_support::MapStore;
use merman_core::{Engine, ParseOptions, RenderSemanticModel};

fn findings(body: &str) -> Vec<String> {
    findings_in(&Engine::new(), body)
}

fn unreachable(state: &str) -> String {
    format!("state '{state}' is unreachable from the initial state")
}

fn dead_end(state: &str) -> String {
    format!(
        "state '{state}' has no outgoing transition and is not final; add '{state} --> [*]' if it is terminal"
    )
}

fn none() -> Vec<String> {
    Vec::new()
}

#[test]
fn a_well_formed_lifecycle_raises_nothing() {
    let body = "stateDiagram-v2\n[*] --> A\nA --> B\nB --> [*]\n";
    assert_eq!(findings(body), none());
}

#[test]
fn an_isolated_pair_is_unreachable_and_its_sink_is_a_dead_end() {
    let got = findings("stateDiagram-v2\n[*] --> A\nA --> [*]\nC --> D\n");
    assert_eq!(got, vec![unreachable("C"), unreachable("D"), dead_end("D")]);
}

#[test]
fn a_state_without_an_exit_is_a_dead_end() {
    let got = findings("stateDiagram-v2\n[*] --> A\nA --> B\n");
    assert_eq!(got, vec![dead_end("B")]);
}

#[test]
fn a_composite_with_an_inner_end_that_leads_out_raises_nothing() {
    let body = "stateDiagram-v2\n[*] --> Run\nstate Run {\n[*] --> I\nI --> J\nJ --> [*]\n}\nRun --> Done\nDone --> [*]\n";
    assert_eq!(findings(body), none());
}

#[test]
fn a_state_inside_a_composite_with_an_outgoing_edge_is_not_a_dead_end() {
    let body = "stateDiagram-v2\n[*] --> Run\nstate Run {\n[*] --> I\nI --> J\n}\nRun --> [*]\n";
    assert_eq!(findings(body), none());
}

#[test]
fn a_composite_without_an_initial_transition_leaves_its_states_unreachable() {
    let body = "stateDiagram-v2\n[*] --> Run\nstate Run {\nI --> J\nJ --> [*]\n}\nRun --> [*]\n";
    assert_eq!(findings(body), vec![unreachable("I"), unreachable("J")]);
}

#[test]
fn two_initial_transitions_at_the_root_name_the_scope() {
    let got = findings("stateDiagram-v2\n[*] --> X\n[*] --> Y\nX --> [*]\nY --> [*]\n");
    assert_eq!(got, vec!["scope 'root' has 2 initial transitions"]);
}

#[test]
fn two_initial_transitions_in_a_composite_name_that_composite() {
    let body = "stateDiagram-v2\n[*] --> Run\nstate Run {\n[*] --> I\n[*] --> J\nI --> [*]\nJ --> [*]\n}\nRun --> [*]\n";
    assert_eq!(
        findings(body),
        vec!["scope 'Run' has 2 initial transitions"]
    );
}

#[test]
fn concurrent_regions_get_one_not_proved_advisory() {
    let body = "stateDiagram-v2\n[*] --> Run\nstate Run {\n[*] --> A\nA --> [*]\n--\n[*] --> B\nB --> [*]\n}\nRun --> [*]\n";
    let got = findings(body);
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(got[0].starts_with("not proved: "), "{got:?}");
    assert!(got[0].contains("concurrent"), "{got:?}");
}

#[test]
fn a_diagram_without_a_root_initial_state_is_not_proved_but_dead_ends_are_found() {
    let got = findings("stateDiagram-v2\nA --> B\nB --> C\nC --> [*]\nD --> E\n");
    assert_eq!(got.len(), 2, "{got:?}");
    assert!(got[0].starts_with("not proved: "), "{got:?}");
    assert_eq!(got[1], dead_end("E"));
}

#[test]
fn a_note_on_a_state_with_no_exit_does_not_hide_its_dead_end() {
    let body = "stateDiagram-v2\n[*] --> A\nA --> B\nnote right of B : terminal?\n";
    assert_eq!(findings(body), vec![dead_end("B")]);
}

#[test]
fn a_composite_entered_through_an_inner_state_is_reached() {
    let body = "stateDiagram-v2\n[*] --> Pending\nPending --> Picking\nstate Processing {\n[*] --> Picking\nPicking --> Packing\nPacking --> [*]\n}\nProcessing --> [*]\n";
    assert_eq!(findings(body), none());
}

#[test]
fn entering_a_composite_through_an_inner_state_does_not_take_its_own_initial_state() {
    let body = "stateDiagram-v2\n[*] --> A\nA --> Y\nstate C {\n[*] --> X\nX --> [*]\nY --> [*]\n}\nC --> [*]\n";
    assert_eq!(findings(body), vec![unreachable("X")]);
}

#[test]
fn a_composite_nobody_enters_is_unreachable() {
    let body = "stateDiagram-v2\n[*] --> A\nA --> [*]\nstate Orphan {\n[*] --> I\nI --> [*]\n}\nOrphan --> [*]\n";
    assert_eq!(
        findings(body),
        vec![unreachable("Orphan"), unreachable("I")]
    );
}

#[test]
fn a_note_and_its_edge_are_ignored() {
    let body = "stateDiagram-v2\n[*] --> A\nA --> [*]\nnote right of A : remember this\n";
    assert_eq!(findings(body), none());
}

#[test]
fn choice_is_a_plain_state() {
    let body =
        "stateDiagram-v2\n[*] --> C\nstate C <<choice>>\nC --> A\nC --> B\nA --> [*]\nB --> [*]\n";
    assert_eq!(findings(body), none());
}

#[test]
fn a_flowchart_fence_is_not_judged() {
    assert_eq!(findings("flowchart LR\nA --> B\n"), none());
}

fn view(kind: &str, fence: &str) -> String {
    format!(
        "---\ntype: Architecture View\ntitle: Lifecycle\nkind: {kind}\n---\n\n# Lifecycle\n\n```mermaid\n{fence}```\n"
    )
}

fn run(path: &str, contents: &str) -> Vec<(String, String)> {
    let (store, all_md) = MapStore::seeded(&[(path, contents)]);
    let mut reporter = Reporter::new();
    check_state_machine(&store, &all_md, &mut reporter);
    reporter
        .advisories
        .into_iter()
        .map(|a| (a.kind, a.message))
        .collect()
}

const OPEN_ENDED: &str = "stateDiagram-v2\n[*] --> A\nA --> B\n";

#[test]
fn a_state_view_reports_each_finding_under_the_kind_naming_the_view() {
    let got = run("architecture/life.md", &view("state", OPEN_ENDED));
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].0, "state-machine");
    assert!(got[0].1.contains("life"), "{got:?}");
    assert!(got[0].1.contains(&dead_end("B")), "{got:?}");
}

#[test]
fn a_view_of_another_kind_is_not_judged() {
    assert_eq!(
        run("architecture/life.md", &view("component", OPEN_ENDED)),
        vec![]
    );
}

#[test]
fn a_record_outside_the_architecture_directory_is_not_judged() {
    assert_eq!(run("notes/life.md", &view("state", OPEN_ENDED)), vec![]);
}

fn model_of(text: &str) -> merman_core::diagrams::state::StateDiagramRenderModel {
    let parsed = Engine::new()
        .parse_diagram_for_render_model_sync(text, ParseOptions::strict())
        .unwrap()
        .unwrap();
    let RenderSemanticModel::State(model) = parsed.model else {
        panic!("a stateDiagram parses to the state model");
    };
    model
}

/// The check reads these shapes and identifiers from merman's model; a change
/// of either in an upgrade must fail here rather than silence the rules.
#[test]
fn the_model_keeps_the_shapes_and_ids_the_rules_read() {
    let model = model_of(
        "stateDiagram-v2\n[*] --> Run\nstate Run {\n[*] --> I\nI --> [*]\n}\nnote right of Run : n\n",
    );
    let shape = |id: &str| {
        model
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.shape.as_str())
    };
    assert_eq!(shape("root_start"), Some("stateStart"));
    assert_eq!(shape("Run_start"), Some("stateStart"));
    assert_eq!(shape("Run_end"), Some("stateEnd"));
    assert!(model.nodes.iter().any(|n| n.shape == "note"));
    assert!(model.nodes.iter().any(|n| n.id == "Run" && n.is_group));
    assert!(model
        .nodes
        .iter()
        .any(|n| n.id == "I" && n.parent_id.as_deref() == Some("Run")));
    let has_edge = |a: &str, b: &str| model.edges.iter().any(|e| e.start == a && e.end == b);
    assert!(has_edge("root_start", "Run"));
    assert!(has_edge("Run_start", "I"));
    assert!(has_edge("I", "Run_end"));
    assert!(model.edges.iter().any(|e| e.classes.contains("note-edge")));
}

#[test]
fn the_model_marks_concurrent_regions_with_a_divider_shape() {
    let model = model_of("stateDiagram-v2\n[*] --> Run\nstate Run {\nA --> B\n--\nC --> D\n}\n");
    assert!(model.nodes.iter().any(|n| n.shape == "divider"));
}
