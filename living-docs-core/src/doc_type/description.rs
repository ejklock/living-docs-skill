use super::DOC_TYPES;
use crate::frontmatter;

/// Whether `description`, once trimmed, is the description sentence a
/// registered doc type's template scaffolds, so a record still carrying it
/// has not written its own.
pub fn is_template_description(description: &str) -> bool {
    let description = description.trim();
    DOC_TYPES.iter().any(|spec| {
        frontmatter::read_scalar_from_str(spec.template, "description")
            .is_some_and(|placeholder| placeholder.trim() == description)
    })
}
