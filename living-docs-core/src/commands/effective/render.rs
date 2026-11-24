//! Rendering for the effective view: a
//! one-line index entry per record by default, or the body under `--full`
//! and `--contract`; a contract body arrives already filtered to its tier.
//! No token budget — the view is the active set, read whole.

use super::View;
use std::path::Path;

/// What an empty topic answer tells the caller: how many records are in force,
/// that a word match is not a conflict check, and the commands that list them.
pub fn empty_topic_note(in_force: usize, topic: &str, bundle: &Path) -> String {
    let quote = |text: &str| format!("'{}'", text.replace('\'', r"'\''"));
    let (word, dir) = (quote(topic), quote(&bundle.display().to_string()));
    let s = if in_force == 1 { "" } else { "s" };
    format!(
        "{in_force} record{s} in force and a word match is not a conflict check, as a record using other words may still govern the topic: list them with `living-docs read records`, or `grep -ril -e {word} {dir}`"
    )
}

pub(super) fn render(views: &[View], full: bool, withheld: usize) -> String {
    let prefix = withheld_line(withheld);
    if views.is_empty() {
        return prefix;
    }
    let blocks: Vec<String> = views
        .iter()
        .map(|view| {
            if full {
                full_block(view)
            } else {
                index_line(view)
            }
        })
        .collect();
    let separator = if full { "\n\n" } else { "\n" };
    format!("{prefix}{}\n", blocks.join(separator))
}

/// The withheld-count callout that opens the view above the first block,
/// blank afterward. Empty when nothing was withheld.
fn withheld_line(withheld: usize) -> String {
    if withheld == 0 {
        return String::new();
    }
    format!(
        "_Withheld {withheld} retired record(s) (superseded, deprecated, closed or done): history only, never act on them._\n\n"
    )
}

fn index_line(view: &View) -> String {
    let mut line = format!("- [{}] {}", label(view), view.title);
    if !view.description.is_empty() {
        line.push_str(&format!(" — {}", view.description));
    }
    if let Some(lineage) = &view.lineage {
        line.push_str(&format!(" ({lineage})"));
    }
    if let Some(distance) = view.distance {
        line.push_str(&format!(" [distance {distance}]"));
    }
    line
}

fn full_block(view: &View) -> String {
    let mut header = format!("## [{}] {}", label(view), view.title);
    if let Some(lineage) = &view.lineage {
        header.push_str(&format!("\n_{lineage}_"));
    }
    if let Some(distance) = view.distance {
        header.push_str(&format!("\n_distance {distance}_"));
    }
    format!("{}\n\n{}", header, view.body.trim_end())
}

fn label(view: &View) -> String {
    match view.number {
        Some(number) => format!("{} {number:04}", view.doc_type),
        None => view.doc_type.clone(),
    }
}
