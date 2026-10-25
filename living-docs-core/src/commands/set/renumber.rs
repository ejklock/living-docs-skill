//! The `number` key of `set`: one transaction over a numbered record's
//! heading number, its filename and every in-bundle reference to the old
//! filename. Any status is accepted, since two closed records can collide;
//! a record that takes part in a supersede link is refused until the link
//! can be renumbered with it.

use super::resolve_spec;
use super::retitle::{number_prefix, relocate, replace_heading_number, Retitled};
use crate::commands::next::{next_number_from_store, numeric_prefix};
use crate::doc_type::Identity;
use crate::record::extract_record;
use crate::store::DocStore;
use std::path::{Path, PathBuf};

pub(crate) fn apply(
    store: &dyn DocStore,
    docs_dir: &Path,
    path: &Path,
    value: &str,
) -> Result<Retitled, String> {
    let contents = store.read(path).map_err(|e| e.to_string())?;
    let spec = resolve_spec(path, &contents)?;
    let Identity::Numbered { dir } = spec.identity else {
        return Err(format!("{} records carry no number", spec.frontmatter));
    };
    let old = number_prefix(path)?;
    refuse_supersede_link(store, docs_dir, path, &contents, &old)?;
    let new = choose_number(store, docs_dir, dir, value)?;
    let target = target_path(path, &old, &new)?;

    if let Some(updated) = replace_heading_number(&contents, &old, &new) {
        store.write(path, &updated).map_err(|e| e.to_string())?;
    }
    let rewritten = relocate(store, docs_dir, path, &target)?;
    Ok(Retitled {
        path: target,
        rewritten,
    })
}

/// The four-digit number the record takes: the next free one of the type for
/// `next`, otherwise `value` as given when no record of the type holds it.
fn choose_number(
    store: &dyn DocStore,
    docs_dir: &Path,
    dir: &str,
    value: &str,
) -> Result<String, String> {
    if value == "next" {
        let next = next_number_from_store(store, docs_dir, dir).map_err(|e| e.to_string())?;
        return Ok(format!("{next:04}"));
    }
    if value.len() != 4 || !value.chars().all(|c| c.is_ascii_digit()) || value == "0000" {
        return Err(format!(
            "'{value}' is not a record number; give NNNN (four digits, not 0000) or next"
        ));
    }
    let taken = same_type_records(store, docs_dir, dir)?
        .into_iter()
        .find(|path| file_number(path).as_deref() == Some(value));
    match taken {
        Some(holder) => Err(format!(
            "{value} is already taken by {}; choose a free number or `next`",
            holder.display()
        )),
        None => Ok(value.to_owned()),
    }
}

fn same_type_records(
    store: &dyn DocStore,
    docs_dir: &Path,
    dir: &str,
) -> Result<Vec<PathBuf>, String> {
    let listed = store.list(docs_dir).map_err(|e| e.to_string())?;
    Ok(listed
        .into_iter()
        .filter(|path| {
            path.parent()
                .and_then(Path::file_name)
                .is_some_and(|n| n == dir)
        })
        .collect())
}

fn file_number(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    numeric_prefix(name).map(|number| format!("{number:04}"))
}

/// The record's path with `new` in place of its `old` number prefix, the slug
/// kept.
fn target_path(path: &Path, old: &str, new: &str) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let slug = name
        .strip_prefix(old)
        .ok_or_else(|| format!("{}: filename carries no NNNN- number", path.display()))?;
    Ok(path.with_file_name(format!("{new}{slug}")))
}

/// Refuses a record that carries a supersede link of its own or that another
/// record of its type names in one, since the link holds the old number.
fn refuse_supersede_link(
    store: &dyn DocStore,
    docs_dir: &Path,
    path: &Path,
    contents: &str,
    old: &str,
) -> Result<(), String> {
    let own = extract_record(path, contents);
    for (field, value) in link_fields(&own) {
        if !value.is_empty() {
            return Err(own_link_error(path, field));
        }
    }
    refuse_naming_record(store, docs_dir, path, old)
}

/// Refuses when another record of the same type names `old` in a supersede
/// link.
fn refuse_naming_record(
    store: &dyn DocStore,
    docs_dir: &Path,
    path: &Path,
    old: &str,
) -> Result<(), String> {
    let dir = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|n| n.to_str());
    for other in same_type_records(store, docs_dir, dir.unwrap_or_default())? {
        if other == path {
            continue;
        }
        let text = store.read(&other).map_err(|e| e.to_string())?;
        let record = extract_record(&other, &text);
        for (field, value) in link_fields(&record) {
            if value
                .iter()
                .any(|v| v.trim_matches(['"', '\'', ' ']) == old)
            {
                return Err(format!(
                    "{} names this record in {field}; the supersede link must be renumbered by hand for now",
                    other.display()
                ));
            }
        }
    }
    Ok(())
}

/// Each supersede link field with every value it carries; an absent key or an
/// empty list carries none.
fn link_fields(record: &crate::record::ExtractedRecord) -> [(&'static str, Vec<&str>); 2] {
    let supersedes = record.supersedes.iter().flatten().map(String::as_str);
    let superseded_by = record.superseded_by.iter().map(String::as_str);
    [
        ("supersedes", supersedes.collect()),
        ("superseded_by", superseded_by.collect()),
    ]
}

fn own_link_error(path: &Path, field: &str) -> String {
    format!(
        "{} carries {field}; the supersede link must be renumbered by hand for now",
        path.display()
    )
}
