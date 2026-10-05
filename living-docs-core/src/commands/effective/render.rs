//! Rendering for the effective view (ADR 0050, simplified by ADR 0057): a
//! one-line index entry per record by default, or the body under `--full`
//! and `--contract`; a contract body arrives already filtered to its tier.
//! No token budget — the view is the active set, read whole.

use super::View;

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
        "_Withheld {withheld} retired record(s) (superseded or deprecated): history only, never act on them._\n\n"
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
    line
}

fn full_block(view: &View) -> String {
    let mut header = format!("## [{}] {}", label(view), view.title);
    if let Some(lineage) = &view.lineage {
        header.push_str(&format!("\n_{lineage}_"));
    }
    format!("{}\n\n{}", header, view.body.trim_end())
}

fn label(view: &View) -> String {
    match view.number {
        Some(number) => format!("{} {number:04}", view.doc_type),
        None => view.doc_type.clone(),
    }
}
