//! Heading lookup over a record's body, shared by every consumer that asks
//! which declared sections a record carries (`check`'s required-section pass,
//! `read`'s tiered disclosure).

use crate::doc_type::{SectionSpec, Tier};
use crate::frontmatter::{frontmatter_block, read_scalar_strict};

/// The heading names a record's body carries, in order, trimmed and in their
/// written case. The record's own title heading — the first heading, when its
/// text equals the `title` frontmatter — and any line inside a fenced code
/// block are not sections and never appear.
pub(crate) fn heading_names(contents: &str) -> Vec<String> {
    let mut names: Vec<String> = classified(body_of(contents))
        .into_iter()
        .filter_map(|(_, heading)| heading.map(|(_, text)| text))
        .collect();
    let title = frontmatter_block(contents).and_then(|block| read_scalar_strict(block, "title"));
    if let (Some(first), Some(title)) = (names.first(), title) {
        if heading_text(first) == title {
            names.remove(0);
        }
    }
    names
}

/// A heading's text with its leading heading marks and the `NNNN. ` number
/// prefix the templates write stripped, so it compares against a bare title.
pub(crate) fn heading_text(heading: &str) -> &str {
    let text = heading.trim_start_matches('#').trim();
    text.split_once(". ")
        .filter(|(number, _)| number.len() == 4 && number.chars().all(|c| c.is_ascii_digit()))
        .map_or(text, |(_, rest)| rest)
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

/// `contents` with the section headed `heading` given `text` as its content.
/// The match is the one [`has_section`] uses, so a heading `check` recognizes
/// is a heading found here. An existing section keeps its heading line and
/// loses everything up to the next heading of its own level or higher,
/// deeper subsections included; an absent one is appended at the end, at the
/// level most of the record's sections use (`##` when it has none).
pub(crate) fn replace_or_append_section(contents: &str, heading: &str, text: &str) -> String {
    let (front, body) = contents.split_at(contents.len() - body_of(contents).len());
    let lines = classified(body);
    let sections = section_headings(contents, &lines);
    let text = format!("{}\n", text.trim_end());
    let Some(&(start, level, _)) = sections
        .iter()
        .find(|(_, _, name)| name.eq_ignore_ascii_case(heading))
    else {
        let marks = "#".repeat(dominant_level(&sections));
        return format!("{}\n\n{marks} {heading}\n\n{text}", contents.trim_end());
    };
    let end = sections
        .iter()
        .find(|(index, other, _)| *index > start && *other <= level)
        .map_or(lines.len(), |(index, _, _)| *index);
    let joined = |range: &[(&str, Option<(usize, String)>)]| -> String {
        range.iter().map(|(line, _)| *line).collect()
    };
    let kept = joined(&lines[..=start]);
    let rest = joined(&lines[end..]);
    let gap = if rest.is_empty() { "" } else { "\n" };
    format!("{front}{}\n\n{text}{gap}{rest}", kept.trim_end())
}

/// Every non-title heading as (line index, level, text), the record's own
/// title heading — the first heading, when its text equals the `title`
/// frontmatter — left out as [`heading_names`] leaves it out.
fn section_headings<'a>(
    contents: &str,
    lines: &'a [(&str, Option<(usize, String)>)],
) -> Vec<(usize, usize, &'a str)> {
    let title = frontmatter_block(contents).and_then(|block| read_scalar_strict(block, "title"));
    let mut headings = lines
        .iter()
        .enumerate()
        .filter_map(|(index, (_, heading))| {
            heading
                .as_ref()
                .map(|(level, text)| (index, *level, text.as_str()))
        })
        .collect::<Vec<_>>();
    if let (Some(first), Some(title)) = (headings.first(), title) {
        if heading_text(first.2) == title {
            headings.remove(0);
        }
    }
    headings
}

fn dominant_level(sections: &[(usize, usize, &str)]) -> usize {
    (1..=6)
        .rev()
        .max_by_key(|level| sections.iter().filter(|(_, l, _)| l == level).count())
        .filter(|level| sections.iter().any(|(_, l, _)| l == level))
        .unwrap_or(2)
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
    if line.starts_with('\t') || line.starts_with("    ") {
        return None;
    }
    let trimmed = line.trim();
    let text = trimmed.trim_start_matches('#');
    let marks = trimmed.len() - text.len();
    ((1..=6).contains(&marks) && text.starts_with(' ')).then(|| (marks, text.trim().to_owned()))
}

#[cfg(test)]
mod tests;
