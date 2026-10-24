//! Flags every record whose line breaks are both CRLF and bare LF. The other
//! passes judge such a record on its LF form, so this one is the only
//! finding it earns, and the pre-commit gate blocks it until the author
//! saves the file with one ending. Reserved files and prose without
//! frontmatter are not records and are left alone.

use super::records::{has_frontmatter, is_reserved};
use super::{file_name_str, Reporter};
use crate::line_ending::{ending_of, Ending, MIXED_MESSAGE};
use crate::store::DocStore;
use std::path::{Path, PathBuf};

/// Reports each record in `all_md` with mixed line endings. `store` must hand
/// out the bytes as they are on disk, not their LF form.
pub(crate) fn check_line_endings(
    store: &dyn DocStore,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    for path in all_md {
        if is_reserved(&file_name_str(path)) {
            continue;
        }
        let Ok(contents) = store.read(path) else {
            continue;
        };
        if has_frontmatter(&contents) && ending_of(&contents) == Ending::Mixed {
            reporter.report(Path::new(path), MIXED_MESSAGE);
        }
    }
}
