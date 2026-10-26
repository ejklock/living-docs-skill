//! Resolves a parsed record reference against the records a store lists.

use super::{bundle_relative, normal_parts, resolve_reference};
use crate::paths;
use crate::store::DocStore;
use std::path::{Path, PathBuf};

/// Finds the record `reference` names among every path the active store
/// lists under `docs_dir` — backend agnostic, so a db-mode store's own
/// project-scoped enumeration is honored exactly like a filesystem walk.
/// A type-qualified reference resolves within that type's directory, a bare
/// `NNNN` across every type; both fail loudly with every candidate path when
/// more than one record carries the number rather than silently picking the
/// first match. A record path names exactly that file: when no such file
/// exists it is refused, naming the record or records that carry its number,
/// and never resolves to another record. Whether a listed record lies inside
/// the bundle is the store's to judge when the caller reads or writes it.
pub(crate) fn find_record(
    store: &dyn DocStore,
    docs_dir: &Path,
    reference: &str,
) -> Result<PathBuf, String> {
    let (dir, number) = resolve_reference(docs_dir, reference)?;
    let prefix = format!("{number:04}-");
    let candidates: Vec<PathBuf> = store
        .list(docs_dir)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|path| matches_record_prefix(path, &prefix))
        .collect();

    match dir {
        Some(dir) => {
            let file = exact_file_name(docs_dir, reference);
            find_in_directory(candidates, dir, number, file.as_deref(), reference)
        }
        None => find_unqualified(candidates, number),
    }
}

/// The file name a record path ends in, `None` for a number reference.
fn exact_file_name(docs_dir: &Path, reference: &str) -> Option<String> {
    if !reference.ends_with(".md") {
        return None;
    }
    let relative = bundle_relative(docs_dir, Path::new(reference))?;
    normal_parts(&relative)
        .last()
        .map(|name| (*name).to_owned())
}

fn find_in_directory(
    candidates: Vec<PathBuf>,
    dir: &str,
    number: u32,
    file: Option<&str>,
    reference: &str,
) -> Result<PathBuf, String> {
    let mut same: Vec<PathBuf> = candidates
        .into_iter()
        .filter(|path| parent_dir_name(path) == Some(dir))
        .collect();
    let token = paths::doc_type_for_dir(dir).unwrap_or(dir);
    if let Some(exact) = file.and_then(|name| same.iter().find(|p| file_name(p) == Some(name))) {
        return Ok(exact.clone());
    }
    match (same.len(), file) {
        (0, _) => Err(format!("no record found for {token}/{number:04}")),
        (_, Some(_)) => Err(format!(
            "no record found for {reference}; {} carries {token}/{number:04}",
            display_all(&same)
        )),
        (1, None) => Ok(same.remove(0)),
        (_, None) => Err(ambiguous_in_type_error(&same, token, number)),
    }
}

fn find_unqualified(mut candidates: Vec<PathBuf>, number: u32) -> Result<PathBuf, String> {
    match candidates.as_slice() {
        [] => Err(format!("no record found for {number:04}")),
        [_] => Ok(candidates.remove(0)),
        _ => Err(ambiguous_reference_error(&candidates, number)),
    }
}

fn display_all(candidates: &[PathBuf]) -> String {
    candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Names every candidate the store listed for an ambiguous bare `NNNN` and
/// suggests the `TYPE/NNNN` qualifier, so the caller never has to guess which
/// record the number resolved to.
fn ambiguous_reference_error(candidates: &[PathBuf], number: u32) -> String {
    format!(
        "{number:04} is ambiguous across {}; specify the type explicitly, e.g. TYPE/{number:04}",
        display_all(candidates)
    )
}

/// Names every record of one type that carries the number and shows the path
/// form, the only reference that tells them apart.
fn ambiguous_in_type_error(candidates: &[PathBuf], token: &str, number: u32) -> String {
    format!(
        "{token}/{number:04} is ambiguous across {}; name one by its path, e.g. {}",
        display_all(candidates),
        candidates[0].display()
    )
}

fn file_name(path: &Path) -> Option<&str> {
    path.file_name()?.to_str()
}

fn parent_dir_name(path: &Path) -> Option<&str> {
    path.parent()?.file_name()?.to_str()
}

fn matches_record_prefix(path: &Path, prefix: &str) -> bool {
    file_name(path).is_some_and(|name| name.starts_with(prefix) && name.ends_with(".md"))
}
