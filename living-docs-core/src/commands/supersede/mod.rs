use crate::doc_type::{self, Identity};
use crate::paths;
use crate::store::DocStore;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn run(store: &dyn DocStore, docs_dir: &Path, old: &str, new: &str) -> ExitCode {
    match supersede(store, docs_dir, old, new) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("living-docs supersede: {message}");
            ExitCode::from(2)
        }
    }
}

pub fn supersede(
    store: &dyn DocStore,
    docs_dir: &Path,
    old: &str,
    new: &str,
) -> Result<(), String> {
    let (_, old_number) = parse_record_reference(old)?;
    let (_, new_number) = parse_record_reference(new)?;

    let old_path = find_record(store, docs_dir, old)?;
    let new_path = find_record(store, docs_dir, new)?;

    set_frontmatter_fields(
        store,
        &old_path,
        &[
            ("status", "Superseded".to_string()),
            ("superseded_by", format!("{new_number:04}")),
        ],
    )?;
    set_frontmatter_fields(
        store,
        &new_path,
        &[("supersedes", format!("{old_number:04}"))],
    )?;

    Ok(())
}

pub(crate) fn parse_record_number(arg: &str) -> Result<u32, String> {
    arg.parse()
        .map_err(|_| format!("'{arg}' is not a valid record number"))
}

/// Parses a record reference in either the bare `NNNN` form or the
/// type-qualified `TYPE/NNNN` form (issue 0029/0025), resolving `TYPE`
/// through the registry token map ([`paths::dir_for`]) rather than a
/// hardcoded directory name. Returns the resolved directory alongside the
/// numeric part; `None` for the directory means the reference carries no
/// qualifier and [`find_record`] must resolve it across every type.
pub(crate) fn parse_record_reference(arg: &str) -> Result<(Option<&'static str>, u32), String> {
    match arg.split_once('/') {
        Some((token, number)) => {
            let dir = paths::dir_for(token).ok_or_else(|| {
                format!(
                    "'{token}' is not a valid type; expected one of {}",
                    numbered_type_tokens()
                )
            })?;
            Ok((Some(dir), parse_record_number(number)?))
        }
        None => Ok((None, parse_record_number(arg)?)),
    }
}

fn numbered_type_tokens() -> String {
    doc_type::DOC_TYPES
        .iter()
        .filter(|spec| matches!(spec.identity, Identity::Numbered { .. }))
        .map(|spec| spec.token)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Finds the record `reference` names among every path the active store
/// lists under `docs_dir` — backend agnostic, so a db-mode store's own
/// project-scoped enumeration is honored exactly like a filesystem walk.
/// A type-qualified `TYPE/NNNN` reference resolves within that type's
/// directory only; a bare `NNNN` resolves across every type, failing loudly
/// with every candidate path when more than one type carries that number
/// (issue 0029/0025) rather than silently picking the first match. Shared
/// with `status.rs` and `describe.rs` (lesson 3717: no duplicated
/// record-resolution logic).
pub(crate) fn find_record(
    store: &dyn DocStore,
    docs_dir: &Path,
    reference: &str,
) -> Result<PathBuf, String> {
    let (dir, number) = parse_record_reference(reference)?;
    let prefix = format!("{number:04}-");
    let candidates: Vec<PathBuf> = store
        .list(docs_dir)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|path| matches_record_prefix(path, &prefix))
        .collect();

    match dir {
        Some(dir) => find_in_directory(candidates, dir, number),
        None => find_unqualified(candidates, number),
    }
}

fn find_in_directory(candidates: Vec<PathBuf>, dir: &str, number: u32) -> Result<PathBuf, String> {
    candidates
        .into_iter()
        .find(|path| parent_dir_name(path) == Some(dir))
        .ok_or_else(|| {
            let token = paths::doc_type_for_dir(dir).unwrap_or(dir);
            format!("no record found for {token}/{number:04}")
        })
}

fn find_unqualified(candidates: Vec<PathBuf>, number: u32) -> Result<PathBuf, String> {
    match candidates.len() {
        0 => Err(format!("no record found for {number:04}")),
        1 => Ok(candidates.into_iter().next().expect("length checked above")),
        _ => Err(ambiguous_reference_error(&candidates, number)),
    }
}

/// Names every candidate the store listed for an ambiguous bare `NNNN`
/// (issue 0029/0025) and suggests the `TYPE/NNNN` qualifier, so the caller
/// never has to guess which record the number resolved to.
fn ambiguous_reference_error(candidates: &[PathBuf], number: u32) -> String {
    let paths = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{number:04} is ambiguous across {paths}; specify the type explicitly, e.g. TYPE/{number:04}"
    )
}

fn parent_dir_name(path: &Path) -> Option<&str> {
    path.parent()?.file_name()?.to_str()
}

fn matches_record_prefix(path: &Path, prefix: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".md"))
}

/// Reads the record at `path` once, applies every `(key, value)` pair to its
/// leading frontmatter block in order via [`apply_frontmatter_field`] — a
/// targeted line edit (reusing S2's approach, `new.rs`'s
/// `replace_targeted_value`) rather than a serde round-trip, so comments and
/// the body survive untouched — then writes the result back once. Templates
/// ship most supersede keys as an empty line to fill; when a key is absent
/// entirely (e.g. BDR/PRD templates have no `supersedes` line), it is
/// inserted at the end of the frontmatter block instead. Shared with
/// `status.rs` (lesson 3717: no duplicated frontmatter-mutation logic).
pub(crate) fn set_frontmatter_fields(
    store: &dyn DocStore,
    path: &Path,
    fields: &[(&str, String)],
) -> Result<(), String> {
    let contents = store.read(path).map_err(|e| e.to_string())?;
    let updated = fields
        .iter()
        .try_fold(contents, |acc, (key, value)| {
            apply_frontmatter_field(&acc, key, value)
        })
        .ok_or_else(|| format!("{}: missing frontmatter block", path.display()))?;
    store.write(path, &updated).map_err(|e| e.to_string())
}

/// The shared single-key insert-or-replace frontmatter primitive: replaces
/// `key`'s value via [`set_targeted_value`] when the line is already present
/// in the leading frontmatter block, inserts `key: value` at the block's
/// close when it is absent, and returns `None` when `contents` has no
/// leading `---` fence at all. Used by [`set_frontmatter_fields`] and, for
/// the same insert-or-replace symmetry, by
/// [`crate::commands::new::fill_frontmatter_description`].
pub(crate) fn apply_frontmatter_field(contents: &str, key: &str, value: &str) -> Option<String> {
    let lines: Vec<&str> = contents.lines().collect();
    let close = frontmatter_close_index(&lines)?;
    let prefix = format!("{key}:");

    let mut updated: Vec<String> = lines.iter().map(|&line| line.to_string()).collect();
    match lines[1..close]
        .iter()
        .position(|&line| line.starts_with(&prefix))
    {
        Some(relative_index) => {
            let index = relative_index + 1;
            updated[index] = set_targeted_value(lines[index], &prefix, value);
        }
        None => updated.insert(close, format!("{prefix} {value}")),
    }

    Some(updated.join("\n") + "\n")
}

fn frontmatter_close_index(lines: &[&str]) -> Option<usize> {
    if lines.first() != Some(&"---") {
        return None;
    }
    lines
        .iter()
        .skip(1)
        .position(|&line| line == "---")
        .map(|i| i + 1)
}

/// Replaces the value of a `key: value` frontmatter line, preserving any
/// trailing `# guidance comment` verbatim — mirrors `new.rs::replace_targeted_value`.
fn set_targeted_value(line: &str, prefix: &str, new_value: &str) -> String {
    let rest = line.strip_prefix(prefix).unwrap_or_default();
    match rest.find('#') {
        Some(hash_idx) => format!("{prefix} {new_value} {}", &rest[hash_idx..]),
        None => format!("{prefix} {new_value}"),
    }
}

#[cfg(test)]
mod tests;
