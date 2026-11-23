//! `manage view <view> --export tla`: a TLA+ module for a flat state view, as
//! a pure string. The module is named after the view slug (hyphens become
//! underscores) and declares `VARIABLE state`, `States`, `Terminal`, `Init`,
//! one action per transition in source order, `Next`, `TypeOK` and `Spec`.
//! Every state is a string constant, so any Mermaid id is a valid value. The
//! initial `[*]` gives `Init`; a transition into `[*]` has no action and makes
//! its source a terminal state, which `Next` lets stutter. A diagram with a
//! composite state, concurrent regions, or not exactly one initial transition
//! is refused. The output has no timestamp or version, so it is byte-stable.

use crate::check::state_machine::{
    is_state, is_state_view, mermaid_fences, parse_state_model, ROOT_START, STATE_KIND,
};
use crate::commands::reference::find_settable_record;
use crate::record::strip_frontmatter;
use crate::store::DocStore;
use merman_core::diagrams::state::StateDiagramRenderModel;
use merman_core::Engine;
use std::path::Path;

/// One transition between two states, with its label (empty when none).
struct Move<'a> {
    from: &'a str,
    to: &'a str,
    label: &'a str,
}

/// The module for the architecture view `reference` names: its one state
/// diagram, exported under the view's slug. A view that is not `kind: state`,
/// holds no state diagram or several, or whose diagram is refused, is an error.
pub fn export_view(
    store: &dyn DocStore,
    docs_dir: &Path,
    reference: &str,
) -> Result<String, String> {
    let (path, _) = find_settable_record(store, docs_dir, reference)?;
    let contents = store.read(&path).map_err(|e| e.to_string())?;
    if !is_state_view(&path, &contents) {
        return Err(format!(
            "{reference} is not a state view (an architecture view with `kind: {STATE_KIND}`)"
        ));
    }
    let slug = path
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    if !slug.chars().any(|c| c.is_ascii_alphabetic()) {
        return Err(format!(
            "{reference}: the slug '{slug}' has no letter, so it cannot name a TLA+ module; rename the view"
        ));
    }
    let model = single_state_model(strip_frontmatter(&contents), reference)?;
    export_model(&slug, &model)
}

/// The one state diagram a view's fences hold; with none, the parse failures
/// of the fences that did not parse are named.
fn single_state_model(body: &str, reference: &str) -> Result<StateDiagramRenderModel, String> {
    let engine = Engine::new();
    let (mut models, mut failures) = (Vec::new(), Vec::new());
    for fence in mermaid_fences(body) {
        match parse_state_model(&engine, &fence) {
            Ok(Some(model)) => models.push(model),
            Ok(None) => {}
            Err(failure) => failures.push(failure),
        }
    }
    match (models.pop(), models.is_empty()) {
        (Some(model), true) => Ok(model),
        (None, _) if failures.is_empty() => Err(format!("{reference} holds no state diagram")),
        (None, _) => Err(format!(
            "{reference} holds no state diagram that parses; a diagram does not parse: {}",
            failures.join("; ")
        )),
        _ => Err(format!(
            "{reference} holds several state diagrams; export needs one"
        )),
    }
}

/// The module for one Mermaid state-diagram body, named after `slug`.
pub fn export_diagram(slug: &str, body: &str) -> Result<String, String> {
    let model = parse_state_model(&Engine::new(), body)
        .map_err(|failure| format!("the diagram does not parse: {failure}"))?
        .ok_or("the text is not a state diagram")?;
    export_model(slug, &model)
}

fn export_model(slug: &str, model: &StateDiagramRenderModel) -> Result<String, String> {
    let refused = refusals(model);
    if !refused.is_empty() {
        return Err(format!("cannot export: {}", refused.join("; ")));
    }
    Ok(render(slug, model))
}

/// What keeps a diagram from being exported, each named.
fn refusals(model: &StateDiagramRenderModel) -> Vec<String> {
    let mut found: Vec<String> = model
        .nodes
        .iter()
        .filter(|n| n.is_group)
        .map(|n| format!("composite state '{}'", n.id))
        .collect();
    if model.nodes.iter().any(|n| n.shape == "divider") {
        found.push("concurrent regions (`--`)".to_owned());
    }
    let initial = model.edges.iter().filter(|e| e.start == ROOT_START).count();
    match initial {
        0 => found.push("no initial transition from '[*]'".to_owned()),
        1 => {}
        n => found.push(format!("{n} initial transitions from '[*]'")),
    }
    found
}

/// The transitions between states, without notes and without `[*]` ends.
fn moves(model: &StateDiagramRenderModel) -> Vec<Move<'_>> {
    let is_state_id = |id: &str| model.nodes.iter().any(|n| n.id == id && is_state(n));
    model
        .edges
        .iter()
        .filter(|e| !e.classes.contains("note-edge") && is_state_id(&e.start))
        .filter(|e| is_state_id(&e.end))
        .map(|e| Move {
            from: &e.start,
            to: &e.end,
            label: e.label.trim(),
        })
        .collect()
}

/// States that have a transition into `[*]`, in source order.
fn terminals(model: &StateDiagramRenderModel) -> Vec<&str> {
    let ends = |id: &str| {
        model
            .nodes
            .iter()
            .any(|n| n.id == id && n.shape == "stateEnd")
    };
    let mut out: Vec<&str> = Vec::new();
    for edge in model.edges.iter().filter(|e| ends(&e.end)) {
        if !out.contains(&edge.start.as_str()) {
            out.push(&edge.start);
        }
    }
    out
}

/// Every state once, in order of first appearance: the initial target, the
/// transitions' endpoints, then the states no transition touches.
fn states<'a>(
    model: &'a StateDiagramRenderModel,
    moves: &[Move<'a>],
    init: &'a str,
) -> Vec<&'a str> {
    let mut out = vec![init];
    let listed = moves.iter().flat_map(|m| [m.from, m.to]);
    let declared = model
        .nodes
        .iter()
        .filter(|n| is_state(n))
        .map(|n| n.id.as_str());
    for id in listed.chain(declared) {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// A TLA+ string constant for a state id.
fn constant(id: &str) -> String {
    format!("\"{}\"", id.replace('\\', "\\\\").replace('"', "\\\""))
}

fn set_of(ids: &[&str]) -> String {
    let items: Vec<String> = ids.iter().map(|id| constant(id)).collect();
    format!("{{{}}}", items.join(", "))
}

/// The one-line comment naming a transition and its label.
fn comment(m: &Move<'_>) -> String {
    let words = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let label = words(m.label);
    let tail = if label.is_empty() {
        String::new()
    } else {
        format!(": {label}")
    };
    format!("\\* {} --> {}{tail}", words(m.from), words(m.to))
}

/// The text of a module: its header, the sets, `Init`, the actions, `Next`,
/// `TypeOK` and `Spec`.
fn render(slug: &str, model: &StateDiagramRenderModel) -> String {
    let moves = moves(model);
    let init = model
        .edges
        .iter()
        .find(|e| e.start == ROOT_START)
        .map_or("", |e| e.end.as_str());
    let all = states(model, &moves, init);
    let mut out = format!(
        "---- MODULE {} ----\nVARIABLE state\n\n",
        slug.replace('-', "_")
    );
    out.push_str(&format!("States == {}\n\n", set_of(&all)));
    out.push_str(&format!("Terminal == {}\n\n", set_of(&terminals(model))));
    out.push_str(&format!("Init == state = {}\n\n", constant(init)));
    out.push_str(&actions(&moves));
    out.push_str(&next(moves.len()));
    out.push_str("TypeOK == state \\in States\n\n");
    out.push_str("Spec == Init /\\ [][Next]_state /\\ WF_state(Next)\n====\n");
    out
}

/// One action per transition, each preceded by its comment.
fn actions(moves: &[Move<'_>]) -> String {
    let mut out = String::new();
    for (index, m) in moves.iter().enumerate() {
        out.push_str(&format!(
            "{}\nT{} == state = {} /\\ state' = {}\n\n",
            comment(m),
            index + 1,
            constant(m.from),
            constant(m.to)
        ));
    }
    out
}

/// `Next`: the actions' disjunction and a stuttering step on each terminal state.
fn next(count: usize) -> String {
    let mut parts: Vec<String> = (1..=count).map(|i| format!("T{i}")).collect();
    parts.push("(state \\in Terminal /\\ state' = state)".to_owned());
    format!("Next == {}\n\n", parts.join(" \\/ "))
}

#[cfg(test)]
mod tests;
