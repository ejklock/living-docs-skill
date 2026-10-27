use crate::callout;
use crate::commands::index::{refresh_for_records, Indexed};
use crate::commands::reference::find_settable_record;
use crate::commands::supersede::set_frontmatter_fields_expecting;
use crate::doc_type::{self, DocTypeSpec, Identity};
use crate::line_ending::{refuse_mixed, LineEndingStore};
use crate::record::{extract_record, format_scalar, TailValue};
use crate::sections;
use crate::store::DocStore;
use std::path::{Path, PathBuf};

mod issue_keys;
mod renumber;
pub(crate) mod retitle;

/// What `set` wrote: the record's path — the new one when a retitle renamed
/// the file — and the other bundle records whose reference to the old
/// filename was rewritten with it. `rewritten` is empty for every
/// key but `title` and `number`.
#[derive(Debug)]
pub struct Applied {
    pub path: PathBuf,
    /// Where the record lived before a retitle renamed it, so the front can
    /// name the search for the references outside the bundle that no scan of
    /// the bundle can reach. `None` when nothing was renamed.
    pub previous: Option<PathBuf>,
    pub rewritten: Vec<PathBuf>,
    /// The index files refreshed after the write, and any that could not be.
    pub indexed: Indexed,
}

/// A reference is a bare `NNNN`, a `TYPE/NNNN`, or — for the types with no
/// number — `view/<slug>`, `architecture/<slug>` or `constitution`
/// ([`find_settable_record`]); those take `description` and `title` only, and
/// `status` and `owner` are refused ([`refuse_uncarried`]).
///
/// Sets one CLI-owned frontmatter field on a record: `status`, `description`,
/// `owner`, `title`, or — on an issue only — `tracker`, `labels`,
/// `blocked_by` ([`issue_keys`]: `tracker` is one scalar, the other two take
/// a comma-separated value written as a flow list). Record resolution
/// ([`find_settable_record`], which extends `find_record`'s bare `NNNN` and
/// type-qualified `TYPE/NNNN` references) and the frontmatter
/// write ([`set_frontmatter_fields_expecting`]) are shared with `supersede`. Only
/// `status` carries a vocabulary constraint (validated against the record's
/// own type, `Superseded` reserved for `supersede`); `description`/`owner`
/// accept any string, quoted via [`format_scalar`]. `title` is the one key
/// that reaches beyond the record's frontmatter — it rewrites the heading,
/// renames the file and repoints every in-bundle reference, and is refused on
/// a record closed for good ([`retitle`]). `number` takes `NNNN` or `next`
/// and does the same for the record's number on any status, refusing a number
/// a record of the type holds and a record in a supersede link
/// ([`renumber`]). Nothing is written when
/// the reference is unresolvable or the value fails validation. Returns what
/// was written — the CLI front renders it as colored text or JSON.
pub fn apply(
    store: &dyn DocStore,
    docs_dir: &Path,
    reference: &str,
    key: &str,
    value: &str,
) -> Result<Applied, String> {
    let (path, named) = find_settable_record(store, docs_dir, reference)?;
    if let Some(spec) = named {
        refuse_uncarried(spec, key)?;
    }
    refuse_mixed(store, &path)?;
    let store: &dyn DocStore = &LineEndingStore::new(store, false);
    let renamed = match key {
        "title" => Some(retitle::apply(store, docs_dir, &path, value)?),
        "number" => Some(renumber::apply(store, docs_dir, &path, value)?),
        _ => None,
    };
    if let Some(renamed) = renamed {
        return settle_renamed(store, docs_dir, path, renamed);
    }
    let (field, value) = resolve_field(store, &path, key, value)?;
    let expected: Vec<(&str, TailValue)> = value.map(|v| (field.0, v)).into_iter().collect();
    set_frontmatter_fields_expecting(store, &path, &[field], &expected)?;
    settle_in_place(store, docs_dir, path)
}

/// Refuses `status` and `owner` on a type whose registry entry does not carry
/// them: an architecture view and the constitution have no owner, and no
/// status the CLI may set.
fn refuse_uncarried(spec: &DocTypeSpec, key: &str) -> Result<(), String> {
    let carries_owner = matches!(spec.identity, Identity::Numbered { .. });
    match key {
        "status" if spec.status_vocabulary.is_empty() => {
            Err(format!("{} records carry no status", spec.frontmatter))
        }
        "owner" if !carries_owner => Err(format!("{} records carry no owner", spec.frontmatter)),
        _ => Ok(()),
    }
}

/// Finishes a retitle or a renumbering: reconciles the callout at the
/// record's new path and refreshes the type's index.
fn settle_renamed(
    store: &dyn DocStore,
    docs_dir: &Path,
    old: PathBuf,
    renamed: retitle::Retitled,
) -> Result<Applied, String> {
    callout::reconcile(store, &renamed.path)?;
    let previous = (renamed.path != old).then_some(old);
    let indexed = refresh_for_records(store, docs_dir, &[renamed.path.as_path()]);
    Ok(Applied {
        path: renamed.path,
        previous,
        rewritten: renamed.rewritten,
        indexed,
    })
}

/// Finishes a write that kept the record where it was: reconciles the
/// callout and refreshes the type's index.
fn settle_in_place(
    store: &dyn DocStore,
    docs_dir: &Path,
    path: PathBuf,
) -> Result<Applied, String> {
    callout::reconcile(store, &path)?;
    let indexed = refresh_for_records(store, docs_dir, &[path.as_path()]);
    Ok(Applied {
        path,
        previous: None,
        rewritten: Vec::new(),
        indexed,
    })
}

/// Replaces or appends one body section of a record — the `section` key of
/// `set` — through [`crate::sections::replace_or_append_section`], so the
/// heading is matched the way `check` matches a required section. The
/// frontmatter is not touched; the callout is reconciled and the type's index
/// refreshed like every other key. A blank or multi-line `heading` is
/// refused before anything is written.
pub fn apply_section(
    store: &dyn DocStore,
    docs_dir: &Path,
    reference: &str,
    heading: &str,
    text: &str,
) -> Result<Applied, String> {
    if heading.trim().is_empty() || heading.contains('\n') {
        return Err("a section heading is one non-empty line of text".to_string());
    }
    let (path, _) = find_settable_record(store, docs_dir, reference)?;
    refuse_mixed(store, &path)?;
    let store: &dyn DocStore = &LineEndingStore::new(store, false);
    let contents = store.read(&path).map_err(|e| e.to_string())?;
    let updated = sections::replace_or_append_section(&contents, heading.trim(), text);
    store.write(&path, &updated).map_err(|e| e.to_string())?;
    settle_in_place(store, docs_dir, path)
}

fn resolve_field(
    store: &dyn DocStore,
    path: &Path,
    key: &str,
    value: &str,
) -> Result<((&'static str, String), Option<TailValue>), String> {
    let plain = |key, value| Ok(((key, value), None));
    match key {
        "status" => {
            let contents = store.read(path).map_err(|e| e.to_string())?;
            let spec = resolve_spec(path, &contents)?;
            validate_status(value, spec)?;
            plain("status", value.to_string())
        }
        "description" => plain("description", format_scalar(value)),
        "owner" => plain("owner", format_scalar(value)),
        "tracker" => issue_field(store, path, "tracker", value),
        "labels" => issue_field(store, path, "labels", value),
        "blocked_by" => issue_field(store, path, "blocked_by", value),
        other => Err(format!(
            "'{other}' is not a settable field; expected one of status, description, owner, tracker, labels, blocked_by, title, number, or section with --from"
        )),
    }
}

fn issue_field(
    store: &dyn DocStore,
    path: &Path,
    key: &'static str,
    value: &str,
) -> Result<((&'static str, String), Option<TailValue>), String> {
    let (key, rendered, intended) = issue_keys::field(store, path, key, value)?;
    Ok(((key, rendered), Some(intended)))
}

/// Resolves the [`DocTypeSpec`] the record at `path` belongs to, from its own
/// `type:` frontmatter, so a status is validated against its own type's
/// vocabulary.
fn resolve_spec(path: &Path, contents: &str) -> Result<&'static DocTypeSpec, String> {
    let doc_type = extract_record(path, contents).doc_type;
    doc_type::spec_for_frontmatter(&doc_type).ok_or_else(|| {
        format!(
            "{}: unrecognized 'type: {doc_type}' frontmatter",
            path.display()
        )
    })
}

fn validate_status(new_status: &str, spec: &DocTypeSpec) -> Result<(), String> {
    if spec.status_vocabulary.contains(&new_status) {
        return Ok(());
    }
    if new_status.eq_ignore_ascii_case("superseded") {
        return Err(
            "'Superseded' must be set via `living-docs supersede <old> <new>`, which also wires the supersedes/superseded_by links".to_string(),
        );
    }
    Err(format!(
        "'{new_status}' is not a valid status for {}; expected one of {}",
        spec.frontmatter,
        spec.status_vocabulary.join(", ")
    ))
}

#[cfg(test)]
mod tests;
