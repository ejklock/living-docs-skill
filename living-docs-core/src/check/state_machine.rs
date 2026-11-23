//! `state-machine` advisory: in every `stateDiagram` fence of an architecture
//! view whose `kind` is `state`, a state not reachable from the initial state,
//! a dead end, or a scope with more than one initial transition. The graph is
//! the one merman-core parses; notes are ignored, choice, fork and join are
//! plain states, and a diagram with concurrent regions or no root initial state
//! is advised as not proved. It never fails the gate.

use super::records::{for_each_readable_record, frontmatter_scalar};
use super::stale_view::is_view;
use super::{kinds, Reporter};
use crate::record::strip_frontmatter;
use crate::store::DocStore;
use merman_core::diagrams::state::{StateDiagramRenderModel, StateDiagramRenderNode};
use merman_core::{Engine, ParseOptions, RenderSemanticModel};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// The `kind` of an architecture view whose diagrams are judged.
const STATE_KIND: &str = "state";
/// The id merman gives the initial pseudo-state of the root scope.
const ROOT_START: &str = "root_start";

type Transitions<'a> = HashMap<&'a str, Vec<&'a str>>;

pub(crate) fn check_state_machine(
    store: &dyn DocStore,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    let engine = Engine::new();
    for_each_readable_record(store, all_md, |file, contents| {
        if !is_state_view(file, &contents) {
            return;
        }
        let view = file
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        for fence in mermaid_fences(strip_frontmatter(&contents)) {
            for finding in findings_in(&engine, &fence) {
                reporter.advise(file, kinds::STATE_MACHINE, format!("{view}: {finding}"));
            }
        }
    });
}

/// Whether `path` is an architecture view declaring `kind: state`.
fn is_state_view(path: &Path, contents: &str) -> bool {
    is_view(path, contents) && frontmatter_scalar(contents, "kind").as_deref() == Some(STATE_KIND)
}

/// The body of every closed ```mermaid``` fence in `text`.
fn mermaid_fences(text: &str) -> Vec<String> {
    let mut fences = Vec::new();
    let mut open: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        match &mut open {
            None if trimmed == "```mermaid" => open = Some(String::new()),
            Some(body) if trimmed == "```" => {
                fences.push(std::mem::take(body));
                open = None;
            }
            Some(body) => {
                body.push_str(line);
                body.push('\n');
            }
            None => {}
        }
    }
    fences
}

/// Findings for one fence body; a body that is not a state diagram, or does
/// not parse (the fence check reports that), has none.
fn findings_in(engine: &Engine, body: &str) -> Vec<String> {
    let parsed = engine.parse_diagram_for_render_model_sync(body, ParseOptions::strict());
    match parsed {
        Ok(Some(parsed)) => match parsed.model {
            RenderSemanticModel::State(model) => findings(&model),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Whether a shape is a note's, which never takes part in the graph.
fn is_note(shape: &str) -> bool {
    matches!(shape, "note" | "noteGroup")
}

/// A node is a state unless it is a pseudo-state, a divider or a note.
fn is_state(node: &StateDiagramRenderNode) -> bool {
    !matches!(node.shape.as_str(), "stateStart" | "stateEnd" | "divider") && !is_note(&node.shape)
}

/// The edges between states, without a note's edge.
fn transitions(model: &StateDiagramRenderModel) -> Transitions<'_> {
    let notes: HashSet<&str> = model
        .nodes
        .iter()
        .filter(|n| is_note(&n.shape))
        .map(|n| n.id.as_str())
        .collect();
    let mut out = Transitions::new();
    for edge in &model.edges {
        let to_note = notes.contains(edge.start.as_str()) || notes.contains(edge.end.as_str());
        if !to_note && !edge.classes.contains("note-edge") {
            out.entry(&edge.start).or_default().push(&edge.end);
        }
    }
    out
}

fn findings(model: &StateDiagramRenderModel) -> Vec<String> {
    let out = transitions(model);
    let has_root = model.nodes.iter().any(|n| n.id == ROOT_START);
    let concurrent = model.nodes.iter().any(|n| n.shape == "divider");
    let mut found = Vec::new();
    if concurrent {
        found.push(
            "not proved: concurrent regions (`--`) are not analysed for reachability or initial transitions"
                .to_owned(),
        );
    } else if !has_root {
        found.push(
            "not proved: no initial transition from '[*]' at the root, so reachability was not checked"
                .to_owned(),
        );
    } else {
        found.extend(unreachable_states(model, &out));
    }
    found.extend(dead_ends(model, &out));
    if !concurrent {
        found.extend(initial_counts(model, &out));
    }
    found
}

/// States a walk from the root initial state never reaches; a composite entered
/// directly takes its own initial pseudo-state, while entering an inner state
/// also reaches its enclosing composites but not their initial pseudo-states.
fn unreachable_states(model: &StateDiagramRenderModel, out: &Transitions<'_>) -> Vec<String> {
    let reached = reached_ids(model, out);
    model
        .nodes
        .iter()
        .filter(|n| is_state(n) && !reached.contains(n.id.as_str()))
        .map(|n| format!("state '{}' is unreachable from the initial state", n.id))
        .collect()
}

/// Each node's enclosing composite state, for the nodes that have one.
fn enclosing(model: &StateDiagramRenderModel) -> HashMap<&str, &str> {
    model
        .nodes
        .iter()
        .filter_map(|n| Some((n.id.as_str(), n.parent_id.as_deref()?)))
        .collect()
}

/// Every id a walk from the root initial state reaches. Entering a composite
/// directly also enters its own initial pseudo-state; entering a state inside a
/// composite enters each enclosing composite, without taking its initial one.
fn reached_ids<'a>(model: &'a StateDiagramRenderModel, out: &Transitions<'a>) -> HashSet<&'a str> {
    let ids: HashSet<&str> = model.nodes.iter().map(|n| n.id.as_str()).collect();
    let parents = enclosing(model);
    let mut walked: HashSet<(&str, bool)> = HashSet::new();
    let mut queue = VecDeque::from([(ROOT_START, true)]);
    while let Some((id, direct)) = queue.pop_front() {
        if !walked.insert((id, direct)) {
            continue;
        }
        queue.extend(out.get(id).into_iter().flatten().map(|next| (*next, true)));
        if direct {
            let inner_start = ids.iter().find(|i| i.strip_suffix("_start") == Some(id));
            queue.extend(inner_start.map(|start| (*start, true)));
        }
        queue.extend(parents.get(id).map(|parent| (*parent, false)));
    }
    walked.into_iter().map(|(id, _)| id).collect()
}

/// States with no way out: not final, no outgoing edge, and no enclosing
/// composite state with one.
fn dead_ends(model: &StateDiagramRenderModel, out: &Transitions<'_>) -> Vec<String> {
    let parents = enclosing(model);
    let stuck = |id: &str| {
        let mut cursor = Some(id);
        while let Some(current) = cursor {
            if out.contains_key(current) {
                return false;
            }
            cursor = parents.get(current).copied();
        }
        true
    };
    model
        .nodes
        .iter()
        .filter(|n| is_state(n) && stuck(&n.id))
        .map(|n| {
            format!(
                "state '{0}' has no outgoing transition and is not final; add '{0} --> [*]' if it is terminal",
                n.id
            )
        })
        .collect()
}

/// Scopes whose initial pseudo-state has more than one transition out.
fn initial_counts(model: &StateDiagramRenderModel, out: &Transitions<'_>) -> Vec<String> {
    model
        .nodes
        .iter()
        .filter(|n| n.shape == "stateStart")
        .filter_map(|n| {
            let count = out.get(n.id.as_str()).map_or(0, Vec::len);
            let scope = n.id.strip_suffix("_start")?;
            (count > 1).then(|| format!("scope '{scope}' has {count} initial transitions"))
        })
        .collect()
}

#[cfg(test)]
mod tests;
