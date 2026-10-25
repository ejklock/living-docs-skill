use crate::callout;
use crate::commands::index::{refresh_for_records, Indexed};
use crate::commands::reference::resolve_reference;
use crate::frontmatter::frontmatter_block;
use crate::line_ending::{refuse_mixed, LineEndingStore};
use crate::record::{
    extract_record, format_flow_list, format_scalar, to_canonical_markdown, TailValue,
};
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
/// paths and the index outcome for the CLI front; [`run`] is the plain-text
/// wrapper. A record with mixed line endings is refused before any write.
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
    if old_path == new_path {
        return Err(format!(
            "{}: a record cannot supersede itself",
            old_path.display()
        ));
    }
    refuse_mixed(store, &old_path)?;
    refuse_mixed(store, &new_path)?;
    let store: &dyn DocStore = &LineEndingStore::new(store, false);

    let old_draft = draft_frontmatter_fields(
        store,
        &old_path,
        &[
            ("status", "Superseded".to_string()),
            ("superseded_by", format!("{new_number:04}")),
        ],
        &[],
    )?;
    let new_draft = draft_frontmatter_fields(
        store,
        &new_path,
        &[("supersedes", supersedes_with(store, &new_path, old_number)?)],
        &[],
    )?;
    store
        .write(&old_path, &old_draft)
        .map_err(|e| e.to_string())?;
    callout::reconcile(store, &old_path)?;
    store
        .write(&new_path, &new_draft)
        .map_err(|e| e.to_string())?;

    let indexed = refresh_for_records(store, docs_dir, &[&old_path, &new_path]);
    Ok(Superseded {
        old: old_path,
        new: new_path,
        indexed,
    })
}

/// The `supersedes` value to write on the replacing record at `path`: the
/// records it already replaces plus `old_number`, rendered as a scalar for
/// one entry and a flow list for several. An `old_number` already listed adds
/// nothing, so repeating a supersede rewrites the same value.
fn supersedes_with(store: &dyn DocStore, path: &Path, old_number: u32) -> Result<String, String> {
    let contents = store.read(path).map_err(|e| e.to_string())?;
    let record = extract_record(path, &contents);
    if guard::has_untyped_supersedes(&record) {
        return Err(format!(
            "{}: 'supersedes' holds an entry that is not a record number; edit the file by hand",
            path.display()
        ));
    }
    let mut listed = record.supersedes.unwrap_or_default();
    let already = listed
        .iter()
        .any(|entry| entry.trim().parse::<u32>().ok() == Some(old_number));
    if !already {
        listed.push(format!("{old_number:04}"));
    }
    Ok(match listed.as_slice() {
        [only] => format_scalar(only),
        many => format_flow_list(many),
    })
}

/// Writes every `(key, value)` pair into the record at `path`: see
/// [`draft_frontmatter_fields`] for the draft and its read-back guard.
pub(crate) fn set_frontmatter_fields(
    store: &dyn DocStore,
    path: &Path,
    fields: &[(&str, String)],
) -> Result<(), String> {
    set_frontmatter_fields_expecting(store, path, fields, &[])
}

/// Like [`set_frontmatter_fields`], and the guard also requires each
/// `expected` key to read back as exactly the value given, not merely as
/// whatever the rendered line parses to.
pub(crate) fn set_frontmatter_fields_expecting(
    store: &dyn DocStore,
    path: &Path,
    fields: &[(&str, String)],
    expected: &[(&str, TailValue)],
) -> Result<(), String> {
    let draft = draft_frontmatter_fields(store, path, fields, expected)?;
    store.write(path, &draft).map_err(|e| e.to_string())
}

/// Reads the record at `path` once, applies every `(key, value)` pair to its
/// leading frontmatter block in order via [`apply_frontmatter_field`] — a
/// targeted line edit, so comments and the body survive untouched — then
/// routes the block through [`canonicalize_frontmatter`]. Nothing is
/// written: a verb touching several records drafts all of them before
/// writing any. A key absent from the record is inserted at the block's close
/// and the canonical pass moves it into position, so `check` reports no
/// non-canonical-frontmatter finding. Shared by `set`, `set`'s retitle and
/// `supersede` so the mutation logic exists once.
///
/// Before returning, the draft is parsed the way the canonical pass parses
/// it ([`extract_record`]) and refused unless every key it did not write
/// reads as before, the body is unchanged (bar a trailing newline the
/// canonical pass appends), each written key reads as its rendered line does
/// on its own, and each `expected` key reads as given.
fn draft_frontmatter_fields(
    store: &dyn DocStore,
    path: &Path,
    fields: &[(&str, String)],
    expected: &[(&str, TailValue)],
) -> Result<String, String> {
    let contents = store.read(path).map_err(|e| e.to_string())?;
    let missing = || format!("{}: missing frontmatter block", path.display());
    let updated = fields
        .iter()
        .try_fold(contents.clone(), |acc, (key, value)| {
            apply_frontmatter_field(&acc, key, value)
        })
        .ok_or_else(missing)?;
    if let Some(key) = unread_key(path, &contents, &updated, fields, expected) {
        let written: Vec<&str> = fields.iter().map(|(k, _)| *k).collect();
        return Err(format!(
            "{}: '{}' cannot be written without changing how '{key}' reads in the record's frontmatter; edit the file by hand",
            path.display(),
            written.join("', '")
        ));
    }
    let canonical = canonicalize_frontmatter(path, &updated).ok_or_else(missing)?;
    match unwritable_key(&updated, &canonical) {
        Some(key) => Err(refusal_message(path, &key)),
        None => Ok(canonical),
    }
}

/// Re-serializes only `contents`' frontmatter block to canonical order via
/// [`extract_record`]/[`to_canonical_markdown`], leaving everything from
/// the closing `---` fence onward byte-identical. `path` only drives
/// typed-identity classification. `None` without a leading frontmatter block.
fn canonicalize_frontmatter(path: &Path, contents: &str) -> Option<String> {
    let block = frontmatter_block(contents)?;
    let tail = &contents["---\n".len() + block.len()..];
    let canonical = to_canonical_markdown(&extract_record(path, contents));
    let canonical_block = frontmatter_block(&canonical)?;
    Some(format!("---\n{canonical_block}{tail}"))
}

/// The shared single-key insert-or-replace frontmatter primitive: replaces
/// `key`'s value via [`set_targeted_value`] when present in the leading
/// block, inserts `key: value` at its close when absent, and returns `None`
/// without a leading `---` fence. Used by [`set_frontmatter_fields`] and by
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

pub(crate) mod guard;
#[cfg(test)]
use guard::body_changed;
use guard::{refusal_message, unread_key, unwritable_key};
#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod tests;
