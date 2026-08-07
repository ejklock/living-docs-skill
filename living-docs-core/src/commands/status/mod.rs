use crate::commands::supersede::{find_record, set_frontmatter_fields};
use crate::doc_type::{self, DocTypeSpec};
use crate::record::extract_record;
use crate::store::DocStore;
use std::path::Path;
use std::process::ExitCode;

pub fn run(store: &dyn DocStore, docs_dir: &Path, reference: &str, new_status: &str) -> ExitCode {
    match status(store, docs_dir, reference, new_status) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("living-docs status: {message}");
            ExitCode::from(2)
        }
    }
}

/// Sets a record's `status:` frontmatter field, reusing `supersede`'s
/// collision-aware record-resolution ([`find_record`], accepting either a
/// bare `NNNN` or a type-qualified `TYPE/NNNN` reference — issue 0029/0025)
/// and frontmatter-mutation ([`set_frontmatter_fields`]) helpers rather than
/// duplicating them (lesson 3717). `new_status` is validated against the
/// record's own resolved [`DocTypeSpec::status_vocabulary`] (ADR 0029)
/// before any write, so an invalid value never reaches the frontmatter
/// writer or partially mutates a file.
fn status(
    store: &dyn DocStore,
    docs_dir: &Path,
    reference: &str,
    new_status: &str,
) -> Result<(), String> {
    let path = find_record(store, docs_dir, reference)?;
    let contents = store.read(&path).map_err(|e| e.to_string())?;
    let spec = resolve_spec(&path, &contents)?;
    validate_status(new_status, spec)?;
    set_frontmatter_fields(store, &path, &[("status", new_status.to_string())])
}

/// Resolves the [`DocTypeSpec`] the record at `path` belongs to, from its own
/// `type:` frontmatter — never a fixed global assumption — so `validate_status`
/// checks a record against its own type's vocabulary (ADR 0029).
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
