//! Required-section check — a live record of a doc type whose registry row
//! declares a `Required` section must carry a heading of that name. The
//! rule is the row's (`doc_type::DocTypeSpec::sections`); a retired record is
//! history and is not held to it.

use super::records::{for_each_readable_record, frontmatter_scalar, is_retired_record};
use super::Reporter;
use crate::doc_type::{self, Requirement};
use crate::sections::{has_section, heading_names};
use crate::store::DocStore;
use std::path::PathBuf;

pub(crate) fn check_required_sections(
    store: &dyn DocStore,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    for_each_readable_record(store, all_md, |f, contents| {
        let Some(spec) = frontmatter_scalar(&contents, "type")
            .and_then(|doc_type| doc_type::spec_for_frontmatter(&doc_type))
        else {
            return;
        };
        if is_retired_record(&contents) {
            return;
        }
        let present = heading_names(&contents);
        for section in spec.sections {
            if section.requirement == Requirement::Required && !has_section(&present, section.name)
            {
                reporter.report(
                    f,
                    format!(
                        "missing required section '{}' (a {} record)",
                        section.name, spec.token
                    ),
                );
            }
        }
    });
}
