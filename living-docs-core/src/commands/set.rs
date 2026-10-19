use crate::callout;
use crate::commands::index::{refresh_for_records, Indexed};
use crate::commands::supersede::{find_record, set_frontmatter_fields};
use crate::doc_type::{self, DocTypeSpec};
use crate::record::{extract_record, format_scalar};
use crate::sections;
use crate::store::DocStore;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod retitle;

/// What `set` wrote: the record's path — the new one when a retitle renamed
/// the file — and the other bundle records whose reference to the old
/// filename was rewritten with it. `rewritten` is empty for every
/// key but `title`.
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

pub fn run(
    store: &dyn DocStore,
    docs_dir: &Path,
    reference: &str,
    key: &str,
    value: &str,
) -> ExitCode {
    match apply(store, docs_dir, reference, key, value) {
        Ok(_) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("living-docs set: {message}");
            ExitCode::from(2)
        }
    }
}

/// Sets one CLI-owned frontmatter field on a record: `status`, `description`,
/// `owner`, or `title`. Record resolution ([`find_record`], accepting a bare
/// `NNNN` or a type-qualified `TYPE/NNNN` reference) and the frontmatter
/// write ([`set_frontmatter_fields`]) are shared with `supersede`. Only
/// `status` carries a vocabulary constraint (validated against the record's
/// own type, `Superseded` reserved for `supersede`); `description`/`owner`
/// accept any string, quoted via [`format_scalar`]. `title` is the one key
/// that reaches beyond the record's frontmatter — it rewrites the heading,
/// renames the file and repoints every in-bundle reference, and is refused on
/// a record closed for good ([`retitle`]). Nothing is written when
/// the reference is unresolvable or the value fails validation. Returns what
/// was written — the CLI front renders it as colored text or JSON;
/// [`run`] is the plain-text-always convenience wrapper.
pub fn apply(
    store: &dyn DocStore,
    docs_dir: &Path,
    reference: &str,
    key: &str,
    value: &str,
) -> Result<Applied, String> {
    let path = find_record(store, docs_dir, reference)?;
    if key == "title" {
        let retitled = retitle::apply(store, docs_dir, &path, value)?;
        callout::reconcile(store, &retitled.path)?;
        let previous = (retitled.path != path).then_some(path);
        let indexed = refresh_for_records(store, docs_dir, &[retitled.path.as_path()]);
        return Ok(Applied {
            path: retitled.path,
            previous,
            rewritten: retitled.rewritten,
            indexed,
        });
    }
    let field = resolve_field(store, &path, key, value)?;
    set_frontmatter_fields(store, &path, &[field])?;
    settle_in_place(store, docs_dir, path)
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
    let path = find_record(store, docs_dir, reference)?;
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
) -> Result<(&'static str, String), String> {
    match key {
        "status" => {
            let contents = store.read(path).map_err(|e| e.to_string())?;
            let spec = resolve_spec(path, &contents)?;
            validate_status(value, spec)?;
            Ok(("status", value.to_string()))
        }
        "description" => Ok(("description", format_scalar(value))),
        "owner" => Ok(("owner", format_scalar(value))),
        other => Err(format!(
            "'{other}' is not a settable field; expected one of status, description, owner, title, or section with --from"
        )),
    }
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
