//! Unfilled-placeholder check: a template slot left unfilled is a
//! mechanical defect, so `check` reports `{{PLACEHOLDER}}` as an invariant
//! violation. A slot may carry its authoring hint after the name
//! (`{{CONTEXT: the forces at play}}`), so the guidance lives in the slot
//! and disappears with it. Placeholders shown inside code formatting are
//! ignored, so a record that documents the slot syntax is not flagged. The
//! `description` frontmatter value is held to the same standard: left as a
//! doc type's template sentence, it is an unfilled slot too.

use super::{file_name_str, records, Reporter};
use crate::store::DocStore;
use crate::{doc_type, frontmatter, paths, record};
use std::path::{Path, PathBuf};

pub(crate) fn check_placeholders(
    store: &dyn DocStore,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    for f in all_md {
        if records::is_reserved(&file_name_str(f)) {
            continue;
        }
        let Ok(contents) = store.read(f) else {
            continue;
        };
        let body = record::extract_record(f, &contents).body;
        if has_slot(&without_code(&body)) {
            reporter.report(f, "unfilled {{PLACEHOLDER}} — fill the slot or remove it");
        }
        if has_template_description(&contents) {
            reporter.report(f, placeholder_description_message(f, &contents));
        }
    }
}

/// Whether the record's `description` is, once trimmed, the description a
/// registered doc type's template scaffolds. A description that only quotes
/// that text among other words, or a record with no description, is not.
fn has_template_description(contents: &str) -> bool {
    let Some(description) = frontmatter::read_scalar_from_str(contents, "description") else {
        return false;
    };
    let description = description.trim();
    doc_type::DOC_TYPES.iter().any(|spec| {
        frontmatter::read_scalar_from_str(spec.template, "description")
            .is_some_and(|placeholder| placeholder.trim() == description)
    })
}

fn placeholder_description_message(path: &Path, contents: &str) -> String {
    match record_reference(path, contents) {
        Some(reference) => format!(
            "description is still its template placeholder — run `living-docs write set {reference} description --from -`"
        ),
        None if has_registered_type(contents) => "description is still its template placeholder — the record sits outside the place `write new` gives its type, so no `write set` reference reaches it; edit its `description` by hand or move it there".to_string(),
        None => "description is still its template placeholder — the record has no registered `type`, so no `write set` reference reaches it; set a registered type, or edit its `description` by hand".to_string(),
    }
}

fn has_registered_type(contents: &str) -> bool {
    frontmatter::read_scalar_from_str(contents, "type")
        .and_then(|value| doc_type::spec_for_frontmatter(&value))
        .is_some()
}

/// The reference `write set` accepts for the record, by its doc type's
/// identity: `TYPE/NNNN` for a numbered record, `TYPE/slug` for a named one
/// and the bare token for a singleton, so the message's fix can be pasted as
/// written. A record of no known type, or one outside the form `write new`
/// gives its type (nested below the type directory, or a named record whose
/// file stem is not a slug), has none: a `TYPE/…` reference for it would be
/// refused, or would reach another record.
fn record_reference(path: &Path, contents: &str) -> Option<String> {
    let spec = frontmatter::read_scalar_from_str(contents, "type")
        .and_then(|value| doc_type::spec_for_frontmatter(&value));
    let name = file_name_str(path);
    let stem = name.trim_end_matches(".md");
    let number: String = name.chars().take_while(char::is_ascii_digit).collect();
    match spec.map(|spec| (spec.token, spec.identity)) {
        Some((token, doc_type::Identity::Numbered { dir }))
            if !number.is_empty() && sits_in(path, dir) =>
        {
            Some(format!("{token}/{number}"))
        }
        Some((token, doc_type::Identity::Named { dir } | doc_type::Identity::Nested { dir }))
            if sits_in(path, dir) && paths::slugify(stem) == stem =>
        {
            Some(format!("{token}/{stem}"))
        }
        Some((token, doc_type::Identity::Singleton { .. })) => Some(token.to_string()),
        _ => None,
    }
}

/// Whether `path` lies directly in a directory named `dir`.
fn sits_in(path: &Path, dir: &str) -> bool {
    path.parent()
        .and_then(Path::file_name)
        .is_some_and(|parent| parent == dir)
}

/// `text` with fenced ``` blocks and inline `code` spans removed, so a record
/// that legitimately *shows* placeholder syntax inside code formatting is not
/// mistaken for one that left a slot unfilled.
fn without_code(text: &str) -> String {
    let mut out = String::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            out.push_str(&strip_inline_code(line));
            out.push('\n');
        }
    }
    out
}

fn strip_inline_code(line: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for ch in line.chars() {
        if ch == '`' {
            in_code = !in_code;
        } else if !in_code {
            out.push(ch);
        }
    }
    out
}

/// Whether `text` holds a slot: `{{`, one or more characters that are none
/// of `{`, `}` or a line break, then `}}`.
fn has_slot(text: &str) -> bool {
    text.match_indices('{').any(|(start, _)| {
        let Some(rest) = text[start..].strip_prefix("{{") else {
            return false;
        };
        let name = rest.find(['{', '}', '\n']).unwrap_or(rest.len());
        name > 0 && rest[name..].starts_with("}}")
    })
}

#[cfg(test)]
mod tests;
