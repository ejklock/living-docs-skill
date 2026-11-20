use crate::doc_type::{spec_for_dir, DocTypeSpec};
use std::path::Path;

/// The type of the record at `path`, read from the directory that holds it.
fn type_of(path: &Path) -> Option<&'static DocTypeSpec> {
    let dir = path.parent()?.file_name()?.to_str()?;
    spec_for_dir(dir)
}

/// Refuses a supersede between records of different types: the link is a bare
/// record number, which resolves only inside one type's series. The error
/// names both types.
pub(super) fn require_same_type(old: &Path, new: &Path) -> Result<(), String> {
    match (type_of(old), type_of(new)) {
        (Some(a), Some(b)) if a.frontmatter != b.frontmatter => Err(format!(
            "{}: a {} cannot be superseded by a {} ({}); supersede links records of one type",
            old.display(),
            a.frontmatter,
            b.frontmatter,
            new.display()
        )),
        _ => Ok(()),
    }
}
