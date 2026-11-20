//! `path-missing` advisory: a live record names, in inline code, a
//! repository-relative path that no longer exists. Only existence is probed,
//! through the store port; a store that knows no repository root keeps the
//! rule silent. It never fails the gate.

use super::records::{for_each_readable_record, frontmatter_scalar, is_retired_record};
use super::{kinds, Reporter};
use crate::doc_type;
use crate::record::strip_frontmatter;
use crate::store::DocStore;
use pulldown_cmark::{Event, Parser};
use std::path::{Path, PathBuf};

pub(crate) fn check_path_missing(
    store: &dyn DocStore,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    for_each_readable_record(store, all_md, |f, contents| {
        if !is_examined(&contents) {
            return;
        }
        for path in candidate_paths(strip_frontmatter(&contents)) {
            if !names_a_repository_path(store, &path) {
                continue;
            }
            if store.repository_entry_exists(Path::new(&path)) == Some(false) {
                reporter.advise(
                    f,
                    kinds::PATH_MISSING,
                    format!("names `{path}`, which does not exist in the repository"),
                );
            }
        }
    });
}

/// A record of a registry type that its type does not treat as retired.
fn is_examined(contents: &str) -> bool {
    let registered = frontmatter_scalar(contents, "type")
        .is_some_and(|declared| doc_type::spec_for_frontmatter(&declared).is_some());
    registered && !is_retired_record(contents)
}

/// Whether the first segment of `path` is an entry of the repository root,
/// which tells a path from a record reference or a word pair.
fn names_a_repository_path(store: &dyn DocStore, path: &str) -> bool {
    path.split('/')
        .next()
        .is_some_and(|first| store.repository_entry_exists(Path::new(first)) == Some(true))
}

/// Every distinct path-shaped inline-code token of `body`, in document order.
/// Fenced blocks are code blocks to the parser, so they never surface.
fn candidate_paths(body: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for event in Parser::new(body) {
        let Event::Code(token) = event else {
            continue;
        };
        if let Some(path) = path_of(&token) {
            if !found.contains(&path) {
                found.push(path);
            }
        }
    }
    found
}

/// The path a token spells, or `None`: a leading `./` and a trailing
/// `:LINE[:COL]` dropped, only `A-Z a-z 0-9 _ . / @ + ~ -`, a `/` inside,
/// and neither a leading `/` nor a `..` segment, since those never name an
/// entry below the repository root.
fn path_of(token: &str) -> Option<String> {
    let token = token.strip_prefix("./").unwrap_or(token);
    let path = without_position(token);
    let in_charset = path
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "_./@+~-".contains(c));
    let relative = !path.starts_with('/') && path.split('/').all(|part| part != "..");
    (in_charset && relative && path.contains('/')).then(|| path.to_owned())
}

/// `token` without a trailing `:LINE` or `:LINE:COL`.
fn without_position(token: &str) -> &str {
    let mut rest = token;
    for _ in 0..2 {
        match rest.rsplit_once(':') {
            Some((head, digits))
                if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) =>
            {
                rest = head;
            }
            _ => break,
        }
    }
    rest
}

#[cfg(test)]
mod tests;
