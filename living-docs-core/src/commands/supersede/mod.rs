use crate::callout;
use crate::commands::index::{refresh_for_records, Indexed};
use crate::commands::reference::resolve_reference;
use crate::frontmatter::frontmatter_block;
use crate::record::{extract_record, to_canonical_markdown};
use crate::store::DocStore;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub(crate) use crate::commands::reference::find_record;
#[cfg(test)]
use crate::commands::reference::{parse_record_number, parse_record_reference};

pub fn run(store: &dyn DocStore, docs_dir: &Path, old: &str, new: &str) -> ExitCode {
    match supersede(store, docs_dir, old, new) {
        Ok(_) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("living-docs supersede: {message}");
            ExitCode::from(2)
        }
    }
}

/// The two records `supersede` linked and what refreshing their indexes did.
#[derive(Debug)]
pub struct Superseded {
    pub old: PathBuf,
    pub new: PathBuf,
    pub indexed: Indexed,
}

/// Wires both link directions and writes the retired-record callout on
/// `old`, then refreshes the index of each type touched. Returns the two
/// resolved paths and the index outcome — the CLI front renders this as
/// colored text or JSON; [`run`] is the plain-text-always
/// convenience wrapper over it.
pub fn supersede(
    store: &dyn DocStore,
    docs_dir: &Path,
    old: &str,
    new: &str,
) -> Result<Superseded, String> {
    let (_, old_number) = resolve_reference(docs_dir, old)?;
    let (_, new_number) = resolve_reference(docs_dir, new)?;

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
    callout::reconcile(store, &old_path)?;
    set_frontmatter_fields(
        store,
        &new_path,
        &[("supersedes", format!("{old_number:04}"))],
    )?;

    let indexed = refresh_for_records(store, docs_dir, &[&old_path, &new_path]);
    Ok(Superseded {
        old: old_path,
        new: new_path,
        indexed,
    })
}

/// Reads the record at `path` once, applies every `(key, value)` pair to its
/// leading frontmatter block in order via [`apply_frontmatter_field`] — a
/// targeted line edit (reusing S2's approach, `new.rs`'s
/// `replace_targeted_value`) rather than a serde round-trip, so comments and
/// the body survive untouched — then routes the whole frontmatter block
/// through [`crate::record::canonicalize_frontmatter`] before writing it
/// back once (ADR 0048). Templates ship most supersede keys as an empty line
/// to fill; when a key is absent entirely (e.g. the PRD template has no
/// `supersedes` line), the targeted edit inserts it at the block's close,
/// and the canonical pass then moves it into its fixed position — so an
/// inserted or changed key always lands in canonical order and `living-docs
/// check` reports no non-canonical-frontmatter finding afterward. Shared
/// with `status.rs`, `describe.rs`, `owner.rs` (lesson 3717: no duplicated
/// frontmatter-mutation logic).
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
    let canonical = canonicalize_frontmatter(path, &updated)
        .ok_or_else(|| format!("{}: missing frontmatter block", path.display()))?;
    store.write(path, &canonical).map_err(|e| e.to_string())
}

/// Re-serializes only `contents`' frontmatter block to canonical order via
/// [`extract_record`]/[`to_canonical_markdown`], leaving everything from
/// the closing `---` fence onward byte-identical (ADR 0048). `path` only
/// drives typed-identity classification, never emitted, so it has no
/// bearing on the output. `None` without a leading frontmatter block.
fn canonicalize_frontmatter(path: &Path, contents: &str) -> Option<String> {
    let block = frontmatter_block(contents)?;
    let tail = &contents["---\n".len() + block.len()..];
    let canonical = to_canonical_markdown(&extract_record(path, contents));
    let canonical_block = frontmatter_block(&canonical)?;
    Some(format!("---\n{canonical_block}{tail}"))
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
mod store_tests;
#[cfg(test)]
mod tests;
