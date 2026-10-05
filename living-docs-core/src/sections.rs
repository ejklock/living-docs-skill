//! Heading lookup over a record's body, shared by every consumer that asks
//! which declared sections a record carries (`check`'s required-section pass,
//! `read`'s tiered disclosure).

use crate::doc_type::{SectionSpec, Tier};
use crate::frontmatter::frontmatter_block;

/// The heading names a record's body carries, in order, trimmed and in their
/// written case. The record's own title — the first heading — and any line
/// inside a fenced code block are not sections and never appear.
pub(crate) fn heading_names(contents: &str) -> Vec<String> {
    let mut names: Vec<String> = classified(body_of(contents))
        .into_iter()
        .filter_map(|(_, heading)| heading.map(|(_, text)| text))
        .collect();
    if !names.is_empty() {
        names.remove(0);
    }
    names
}

/// Whether `names` carries `wanted`, compared exactly but case-insensitively.
pub(crate) fn has_section(names: &[String], wanted: &str) -> bool {
    names.iter().any(|name| name.eq_ignore_ascii_case(wanted))
}

/// `body` with every section the schema marks `Detail` removed, content and
/// deeper subheadings included. The title, the preamble and any heading the
/// schema does not name stay. A section ends at the next heading of its own
/// level or higher; fenced lines are content, never boundaries.
pub(crate) fn contract_body(body: &str, schema: &[SectionSpec]) -> String {
    let mut kept = String::new();
    let mut dropping: Option<usize> = None;
    for (line, heading) in classified(body) {
        if let Some((level, text)) = heading {
            if dropping.is_some_and(|open| level <= open) {
                dropping = None;
            }
            if dropping.is_none() && is_detail(schema, &text) {
                dropping = Some(level);
            }
        }
        if dropping.is_none() {
            kept.push_str(line);
        }
    }
    kept
}

fn is_detail(schema: &[SectionSpec], name: &str) -> bool {
    schema
        .iter()
        .any(|spec| spec.tier == Tier::Detail && spec.name.eq_ignore_ascii_case(name))
}

/// Each line, terminator kept, paired with its heading level and text when it
/// is a heading outside any fenced code block.
fn classified(body: &str) -> Vec<(&str, Option<(usize, String)>)> {
    let mut fence: Option<char> = None;
    body.split_inclusive('\n')
        .map(|line| {
            if let Some(marker) = fence_marker(line) {
                fence = match fence {
                    None => Some(marker),
                    Some(open) if open == marker => None,
                    still_open => still_open,
                };
                return (line, None);
            }
            (line, fence.map_or_else(|| heading_of(line), |_| None))
        })
        .collect()
}

fn body_of(contents: &str) -> &str {
    match frontmatter_block(contents) {
        Some(block) => &contents[("---\n".len() + block.len() + "\n---".len())..],
        None => contents,
    }
}

fn fence_marker(line: &str) -> Option<char> {
    let trimmed = line.trim_start();
    ['`', '~']
        .into_iter()
        .find(|marker| trimmed.starts_with(&marker.to_string().repeat(3)))
}

fn heading_of(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim();
    let text = trimmed.trim_start_matches('#');
    let marks = trimmed.len() - text.len();
    ((1..=6).contains(&marks) && text.starts_with(' ')).then(|| (marks, text.trim().to_owned()))
}

#[cfg(test)]
mod tests;
