//! The in-bundle references a retitle repoints after it renames a record's
//! file, and the rule that decides which mentions are the renamed file's.

use crate::check::links::resolve_destination;
use crate::store::DocStore;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::ops::Range;
use std::path::{Path, PathBuf};

/// Rewrites every bundle record that names the old filename, the renamed
/// record's own body included, so no link is broken by the rename. When no
/// other bundle file shares the old basename, every whole-name mention is
/// rewritten — prose naming the file as well as link destinations. When
/// another file does (a view and a root file both called `constitution.md`),
/// only a markdown link whose destination resolves, by the rule `check`
/// applies to links, to the renamed file is repointed, and a bare prose
/// mention is left alone as ambiguous.
pub(super) fn rewrite_references(
    store: &dyn DocStore,
    docs_dir: &Path,
    old_path: &Path,
    new_path: &Path,
) -> Result<Vec<PathBuf>, String> {
    let (Some(old_name), Some(new_name)) = (file_name(old_path), file_name(new_path)) else {
        return Ok(Vec::new());
    };
    let listed = store.list(docs_dir).map_err(|e| e.to_string())?;
    let shared = listed
        .iter()
        .any(|p| p != new_path && p != old_path && file_name(p).as_deref() == Some(&old_name));
    let bundle = docs_dir.to_string_lossy();
    let old_resolved = old_path.strip_prefix(docs_dir).ok().and_then(|relative| {
        resolve_destination("", &format!("/{}", relative.to_string_lossy()), &bundle)
    });
    let mut rewritten = Vec::new();
    for path in listed {
        let contents = store.read(&path).map_err(|e| e.to_string())?;
        let updated = match (&old_resolved, shared) {
            (Some(old_resolved), true) => {
                let file = path.to_string_lossy();
                let at = (&*file, &*bundle, old_resolved.as_str());
                repoint_links(&contents, at, (&old_name, &new_name))
            }
            (_, true) => contents.clone(),
            (_, false) => replace_bounded(&contents, &old_name, &new_name, |before, _| {
                !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '-')
            }),
        };
        if updated != contents {
            store.write(&path, &updated).map_err(|e| e.to_string())?;
            rewritten.push(path);
        }
    }
    Ok(rewritten)
}

/// Repoints each link destination in `contents` that resolves to the renamed
/// file. `at` is the linking file, the bundle root and the renamed file's old
/// normalized path; `names` is the old and new file name.
fn repoint_links(contents: &str, at: (&str, &str, &str), names: (&str, &str)) -> String {
    let (file, bundle, old_resolved) = at;
    let (old, new) = names;
    let mut edits = Vec::new();
    for (span, decoded) in destinations(contents) {
        if resolve_destination(file, &decoded, bundle).as_deref() != Some(old_resolved) {
            continue;
        }
        let raw = &contents[span.clone()];
        let (target, anchor) = raw.split_at(raw.find('#').unwrap_or(raw.len()));
        if let Some(stem) = target.strip_suffix(old) {
            edits.push((span, format!("{stem}{new}{anchor}")));
        }
    }
    edits.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    let mut updated = contents.to_owned();
    for (span, text) in edits {
        updated.replace_range(span, &text);
    }
    updated
}

/// Every link and image destination in `contents` as the byte range it
/// occupies in the source and its decoded value: inline destinations and
/// reference definitions, never link text, titles or prose that repeats a
/// destination.
fn destinations(contents: &str) -> Vec<(Range<usize>, String)> {
    let parser = Parser::new(contents).into_offset_iter();
    let mut found: Vec<_> = parser
        .reference_definitions()
        .iter()
        .filter_map(|(_, def)| {
            let label_end = contents[def.span.clone()].find("]:")?;
            let span = raw_destination(contents, def.span.start + label_end + 2);
            Some((span, def.dest.to_string()))
        })
        .collect();
    found.extend(inline_destinations(contents, parser));
    found
}

/// The inline `[text](destination)` and `![alt](destination)` destinations
/// among `events`. The destination follows the link text, whose end is the
/// last end offset among the events nested in the link; a reference-style
/// link has no `](` there and takes its destination from its definition.
fn inline_destinations<'a>(
    contents: &str,
    events: impl Iterator<Item = (Event<'a>, Range<usize>)>,
) -> Vec<(Range<usize>, String)> {
    let mut found = Vec::new();
    let mut open: Vec<(String, usize)> = Vec::new();
    for (event, range) in events {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                open.push((dest_url.into_string(), range.start + 1));
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                open.push((dest_url.into_string(), range.start + 2));
            }
            Event::End(TagEnd::Link | TagEnd::Image) => {
                let Some((dest, text_end)) = open.pop() else {
                    continue;
                };
                if contents[text_end..range.end].starts_with("](") {
                    found.push((raw_destination(contents, text_end + 2), dest));
                }
                if let Some(parent) = open.last_mut() {
                    parent.1 = parent.1.max(range.end);
                }
            }
            _ => {
                if let Some(link) = open.last_mut() {
                    link.1 = link.1.max(range.end);
                }
            }
        }
    }
    found
}

/// The byte range of the destination whose source starts at or after `from`
/// (past any spaces and newlines): the text between `<` and `>`, or the run
/// up to the first whitespace or unbalanced `)`, honouring backslash escapes.
fn raw_destination(contents: &str, from: usize) -> Range<usize> {
    let rest = &contents[from..];
    let start = from + rest.len() - rest.trim_start().len();
    let angled = contents[start..].starts_with('<');
    let body = start + usize::from(angled);
    let (mut depth, mut escaped) = (0_usize, false);
    for (offset, c) in contents[body..].char_indices() {
        let end = body + offset;
        match (escaped, c) {
            (true, _) => escaped = false,
            (false, '\\') => escaped = true,
            (false, '>') if angled => return body..end,
            (false, '(') => depth += 1,
            (false, ')') if depth == 0 && !angled => return body..end,
            (false, ')') => depth = depth.saturating_sub(1),
            (false, c) if c.is_whitespace() && !angled => return body..end,
            _ => {}
        }
    }
    body..contents.len()
}

/// Replaces each occurrence of `old` in `contents` with `new` where `bounds`,
/// given the characters just before and after the occurrence, accepts it — so
/// a slug that ends another record's slug (`rings.md` inside `strings.md`) is
/// never rewritten by accident.
fn replace_bounded(
    contents: &str,
    old: &str,
    new: &str,
    bounds: impl Fn(Option<char>, Option<char>) -> bool,
) -> String {
    let mut out = String::with_capacity(contents.len());
    let mut copied = 0;
    for (at, _) in contents.match_indices(old) {
        let end = at + old.len();
        let before = contents[..at].chars().next_back();
        if bounds(before, contents[end..].chars().next()) {
            out.push_str(&contents[copied..at]);
            out.push_str(new);
            copied = end;
        }
    }
    out.push_str(&contents[copied..]);
    out
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
}
