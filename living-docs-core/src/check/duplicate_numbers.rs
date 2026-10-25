//! Duplicate record numbers: two records of one numbered type that carry the
//! same `NNNN` filename prefix. Parallel branches each allocate the next free
//! number of their own tree, so a merge can leave two records with one number
//! and every reference by number to either becomes ambiguous.

use super::{file_name_str, Reporter};
use crate::commands::next::numeric_prefix;
use crate::doc_type::{self, Identity};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Reports one violation per number shared by two or more records of a
/// numbered type, naming every path (relative to `bundle`) and the
/// `living-docs set <path> number next` repair. The violation is anchored on
/// every member, so a scope holding any one of them keeps it. The type is the record's parent directory and status is
/// ignored, so a closed record collides with an open one. The tool never
/// picks which record moves; the repair keeps the `<path>` placeholder.
pub(crate) fn check_duplicate_numbers(bundle: &Path, all_md: &[PathBuf], reporter: &mut Reporter) {
    for ((_, number), paths) in group_by_number(all_md) {
        if paths.len() < 2 {
            continue;
        }
        let names: Vec<String> = paths
            .iter()
            .map(|p| p.strip_prefix(bundle).unwrap_or(p).display().to_string())
            .collect();
        reporter.report_group(
            &paths,
            format!(
                "records share the number {number:04}: {} — keep one and run `living-docs set <path> number next` on each of the others",
                names.join(", ")
            ),
        );
    }
}

/// The records of every numbered type directory, keyed by that directory and
/// their filename number; paths within a group are sorted so the report is
/// the same whatever order the store listed them in.
fn group_by_number(all_md: &[PathBuf]) -> BTreeMap<(PathBuf, u32), Vec<PathBuf>> {
    let mut groups: BTreeMap<(PathBuf, u32), Vec<PathBuf>> = BTreeMap::new();
    for path in all_md {
        let (Some(parent), Some(number)) = (path.parent(), numeric_prefix(&file_name_str(path)))
        else {
            continue;
        };
        if is_numbered_dir(parent) {
            groups
                .entry((parent.to_path_buf(), number))
                .or_default()
                .push(path.clone());
        }
    }
    groups.values_mut().for_each(|paths| paths.sort());
    groups
}

/// Whether `dir`'s name is the directory of a numbered doc type, the one
/// kind of type whose records carry a number.
fn is_numbered_dir(dir: &Path) -> bool {
    let name = file_name_str(dir);
    matches!(
        doc_type::spec_for_dir(&name).map(|spec| spec.identity),
        Some(Identity::Numbered { .. })
    )
}
