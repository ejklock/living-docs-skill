//! The bundle-root `index.md`: written once when missing, then only ever
//! extended with one link line per type index it does not yet reach. An
//! existing line is never rewritten or reordered, so hand framing survives.

use crate::check::{normpath, reachable_index_set};
use crate::doc_type::{self, DocTypeSpec, Identity};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

const ROOT_FRONTMATTER: &str = "---\nokf_version: \"0.1\"\n---\n\n# Documentation\n\n## Trail\n\n";

/// What to do about a missing root when no type index exists yet to link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoTypeIndex {
    /// Leave the root unwritten: `index` only writes a root it can link.
    WriteNothing,
    /// Write a link-less root: a record with no type index of its own
    /// (the constitution) still needs a root for the bundle to pass `check`.
    WriteLinkless,
}

/// The directory a type's index lives in, `None` for a singleton.
fn type_dir(spec: &DocTypeSpec) -> Option<&'static str> {
    match spec.identity {
        Identity::Numbered { dir } | Identity::Named { dir } => Some(dir),
        Identity::Singleton { .. } => None,
    }
}

/// The `* [Heading](dir/)` line that makes a type index reachable.
fn link_line(spec: &DocTypeSpec, dir: &str) -> String {
    format!("* [{}]({dir}/)\n", spec.index_heading)
}

/// Types whose directory index exists on disk, in registry order.
fn present_types<'a>(
    docs_dir: &Path,
    specs: impl Iterator<Item = &'a DocTypeSpec>,
) -> Vec<(&'a DocTypeSpec, &'static str)> {
    specs
        .filter_map(|spec| Some((spec, type_dir(spec)?)))
        .filter(|(_, dir)| docs_dir.join(dir).join("index.md").is_file())
        .collect()
}

/// Makes the bundle root reach every type index in `touched` (every present
/// type index when the root is missing): creates a missing root, appends one
/// link line per unreached index to a present one, and returns the root's
/// path only when it wrote. A root that is not a regular file, or cannot be
/// read, is an error naming it; nothing already on disk is removed. A missing
/// root with no type index to link follows `no_type_index`.
pub(crate) fn ensure_root_index(
    docs_dir: &Path,
    touched: &[&str],
    no_type_index: NoTypeIndex,
) -> Result<Option<PathBuf>, String> {
    let root = docs_dir.join("index.md");
    match fs::metadata(&root) {
        Ok(meta) if meta.is_file() => append_unreached(docs_dir, &root, touched),
        Ok(_) => Err(format!("{} is not a regular file", root.display())),
        Err(e) if e.kind() == ErrorKind::NotFound => create_root(docs_dir, &root, no_type_index),
        Err(e) => Err(format!("{}: {e}", root.display())),
    }
}

fn create_root(
    docs_dir: &Path,
    root: &Path,
    no_type_index: NoTypeIndex,
) -> Result<Option<PathBuf>, String> {
    let linked = present_types(docs_dir, doc_type::DOC_TYPES.iter());
    if linked.is_empty() && no_type_index == NoTypeIndex::WriteNothing {
        return Ok(None);
    }
    let links: String = linked
        .iter()
        .map(|(spec, dir)| link_line(spec, dir))
        .collect();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root)
        .map_err(|e| format!("{}: {e}", root.display()))?;
    file.write_all(format!("{ROOT_FRONTMATTER}{links}").as_bytes())
        .map_err(|e| format!("{}: {e}", root.display()))?;
    Ok(Some(root.to_path_buf()))
}

/// The text to append to `existing`: `missing` preceded by a newline when the
/// file lacks a final one, in the file's own line ending.
fn appended_text(existing: &str, missing: &str) -> String {
    let separator = if existing.is_empty() || existing.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    let addition = format!("{separator}{missing}");
    if existing.contains("\r\n") {
        addition.replace('\n', "\r\n")
    } else {
        addition
    }
}

fn append_unreached(
    docs_dir: &Path,
    root: &Path,
    touched: &[&str],
) -> Result<Option<PathBuf>, String> {
    let existing = fs::read_to_string(root).map_err(|e| format!("{}: {e}", root.display()))?;
    let reached = reachable_index_set(docs_dir, root);
    let specs = touched.iter().filter_map(|token| doc_type::spec_for(token));
    let missing: String = present_types(docs_dir, specs)
        .iter()
        .filter(|(_, dir)| {
            let index = docs_dir.join(dir).join("index.md");
            !reached.contains(&normpath(&index.to_string_lossy()))
        })
        .map(|(spec, dir)| link_line(spec, dir))
        .collect();
    if missing.is_empty() {
        return Ok(None);
    }
    let addition = appended_text(&existing, &missing);
    let mut file = OpenOptions::new()
        .append(true)
        .open(root)
        .map_err(|e| format!("{}: {e}", root.display()))?;
    file.write_all(addition.as_bytes())
        .map_err(|e| format!("{}: {e}", root.display()))?;
    Ok(Some(root.to_path_buf()))
}
