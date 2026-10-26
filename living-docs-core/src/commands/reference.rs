//! The record-reference grammar shared by `set` and `supersede`: a bare
//! `NNNN`, a type-qualified `TYPE/NNNN` (the token or the directory name),
//! or the path of a record inside the bundle.

use crate::doc_type::{self, DocTypeSpec, Identity};
use crate::paths;
use crate::store::DocStore;
use std::path::{Component, Path, PathBuf};

mod lookup;

pub(crate) use lookup::find_record;

/// The directory a reference names (`None` when it carries no qualifier) and
/// the record number it points at.
pub(crate) type ParsedReference = (Option<&'static str>, u32);

pub(crate) fn parse_record_number(arg: &str) -> Result<u32, String> {
    arg.parse()
        .map_err(|_| format!("'{arg}' is not a valid record number"))
}

/// Parses a record reference in either the bare `NNNN` form or the
/// type-qualified `TYPE/NNNN` form, resolving `TYPE` through the registry
/// ([`paths::dir_for`]) rather than a hardcoded directory name. A `None`
/// directory means the reference carries no qualifier and [`find_record`]
/// must resolve it across every type.
pub(crate) fn parse_record_reference(arg: &str) -> Result<ParsedReference, String> {
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

/// Parses `arg` as a record path when it ends in `.md`, otherwise as a
/// number or qualified reference; the path form needs `docs_dir` to prove the
/// file lies inside the bundle.
pub(crate) fn resolve_reference(docs_dir: &Path, arg: &str) -> Result<ParsedReference, String> {
    if arg.ends_with(".md") {
        parse_record_path(docs_dir, arg)
    } else {
        parse_record_reference(arg)
    }
}

fn numbered_type_tokens() -> String {
    numbered_types(|spec, _| spec.token)
}

fn numbered_type_dirs() -> String {
    numbered_types(|_, dir| dir)
}

fn numbered_types(pick: impl Fn(&doc_type::DocTypeSpec, &'static str) -> &'static str) -> String {
    doc_type::DOC_TYPES
        .iter()
        .filter_map(|spec| match spec.identity {
            Identity::Numbered { dir } => Some(pick(spec, dir)),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// A record path names its type by the parent directory and its record by the
/// number prefix of the file name. [`find_record`] resolves the path to exactly
/// that file and refuses a path naming no file, even when one record carries
/// the number.
fn parse_record_path(docs_dir: &Path, arg: &str) -> Result<ParsedReference, String> {
    let path = Path::new(arg);
    if path.components().any(|part| part == Component::ParentDir) {
        return Err(format!(
            "'{arg}' contains '..'; a record path must stay inside the bundle"
        ));
    }
    let relative = bundle_relative(docs_dir, path).ok_or_else(|| {
        format!(
            "'{arg}' is outside the bundle {}; a record path must lie under it",
            docs_dir.display()
        )
    })?;
    let [dir_name, file_name] = normal_parts(&relative)[..] else {
        return Err(format!(
            "'{arg}' must be DIR/NNNN-slug.md directly under the bundle"
        ));
    };
    let dir = paths::doc_type_for_dir(dir_name)
        .and_then(paths::dir_for)
        .ok_or_else(|| {
            format!(
                "'{dir_name}' is not a type directory; expected one of {}",
                numbered_type_dirs()
            )
        })?;
    let number = leading_number(file_name).ok_or_else(|| {
        format!("'{arg}' has no record number; the file name must start with NNNN-")
    })?;
    Ok((Some(dir), number))
}

fn bundle_relative(docs_dir: &Path, path: &Path) -> Option<PathBuf> {
    let stripped = path.strip_prefix(docs_dir).ok();
    if !path.is_absolute() {
        return Some(stripped.unwrap_or(path).to_path_buf());
    }
    let canonical = docs_dir.canonicalize().ok();
    stripped
        .or_else(|| path.strip_prefix(canonical.as_deref()?).ok())
        .map(Path::to_path_buf)
}

fn normal_parts(relative: &Path) -> Vec<&str> {
    relative
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect()
}

fn leading_number(file_name: &str) -> Option<u32> {
    let end = file_name
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(file_name.len());
    let (digits, rest) = file_name.split_at(end);
    if digits.is_empty() || !rest.starts_with('-') {
        return None;
    }
    digits.parse().ok()
}

/// [`find_record`] plus the forms `set` alone accepts for the types with no
/// number: `view/<slug>` or `architecture/<slug>` for an architecture view and
/// the bare `constitution`; those also return the type's spec. A slug must
/// already be in slug form, so no reference can step outside the type's
/// directory. Numbered references resolve exactly as [`find_record`] does.
pub(crate) fn find_settable_record(
    store: &dyn DocStore,
    docs_dir: &Path,
    reference: &str,
) -> Result<(PathBuf, Option<&'static DocTypeSpec>), String> {
    let Some((spec, relative)) = named_target(reference)? else {
        return find_record(store, docs_dir, reference).map(|path| (path, None));
    };
    let path = docs_dir.join(relative);
    let listed = store.list(docs_dir).map_err(|e| e.to_string())?;
    if listed.contains(&path) {
        return Ok((path, Some(spec)));
    }
    Err(format!("no record found for {reference}"))
}

fn named_target(reference: &str) -> Result<Option<(&'static DocTypeSpec, PathBuf)>, String> {
    if reference.ends_with(".md") {
        return Ok(None);
    }
    let (token, slug) = match reference.split_once('/') {
        Some((token, slug)) => (token, Some(slug)),
        None => (reference, None),
    };
    let Some(spec) = doc_type::spec_for(token) else {
        return slug.map_or(Ok(None), |_| Err(invalid_type(token)));
    };
    match (spec.identity, slug) {
        (Identity::Singleton { file }, None) => Ok(Some((spec, PathBuf::from(file)))),
        (Identity::Singleton { .. }, Some(_)) => Err(invalid_type(token)),
        (Identity::Named { dir }, Some(slug)) if paths::slugify(slug) == slug => {
            Ok(Some((spec, Path::new(dir).join(format!("{slug}.md")))))
        }
        (Identity::Named { .. }, Some(slug)) => Err(format!("'{slug}' is not a view slug")),
        _ => Ok(None),
    }
}

fn invalid_type(token: &str) -> String {
    let named = doc_type::DOC_TYPES
        .iter()
        .filter_map(|spec| match spec.identity {
            Identity::Named { dir } => Some(format!("{}/<slug>, {dir}/<slug>", spec.token)),
            Identity::Singleton { .. } => Some(spec.token.to_string()),
            Identity::Numbered { .. } => None,
        });
    let named = named.collect::<Vec<_>>().join(", ");
    format!(
        "'{token}' is not a valid type; expected one of {}, or {named}",
        numbered_type_tokens()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_path_resolves_by_directory_and_number_prefix() {
        let parsed = resolve_reference(Path::new("/b/docs"), "issues/0049-any-slug.md");
        assert_eq!(parsed, Ok((Some("issues"), 49)));
    }

    #[test]
    fn a_record_path_under_the_bundle_prefix_resolves() {
        let parsed = resolve_reference(Path::new("bundle"), "bundle/adr/0001-x.md");
        assert_eq!(parsed, Ok((Some("adr"), 1)));
    }

    #[test]
    fn a_record_path_outside_the_bundle_is_refused() {
        let err = resolve_reference(Path::new("/b/docs"), "/other/issues/0049-x.md").unwrap_err();
        assert!(err.contains("outside the bundle"), "got: {err}");
    }

    #[test]
    fn a_record_path_without_a_number_is_refused() {
        let err = resolve_reference(Path::new("/b/docs"), "issues/readme.md").unwrap_err();
        assert!(err.contains("no record number"), "got: {err}");
    }

    #[test]
    fn a_singleton_or_unknown_directory_is_not_a_type_directory() {
        let err = resolve_reference(Path::new("/b/docs"), "architecture/0001-x.md").unwrap_err();
        assert!(err.contains("not a type directory"), "got: {err}");
    }
}
