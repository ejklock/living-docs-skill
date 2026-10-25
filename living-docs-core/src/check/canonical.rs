//! ADR 0019 canonical round-trip check (slice S3): a record whose on-disk
//! frontmatter block does not byte-equal the frontmatter block of its own
//! canonical re-serialization (`crate::record::extract_record` ->
//! `crate::record::to_canonical_markdown`) was hand-written rather than
//! produced by a CLI verb, so it is flagged with `living-docs fmt` named as
//! the remediation. Only the frontmatter block is compared — the body is out
//! of scope (the S2 `normalize_frontmatter_gap` lesson: `extract_record`
//! leaves a leading newline on the body that never needs reconciling for a
//! frontmatter-only comparison). The check verifies canonical form (key
//! order, spacing, quoting), never values: an author-owned value round-trips
//! untouched as long as its formatting was already canonical.

use super::records::is_reserved;
use super::{file_name_str, is_bundle_singleton, Reporter};
use crate::commands::supersede::guard::unwritable_key;
use crate::frontmatter::frontmatter_block;
use crate::paths::doc_type_for_dir;
use crate::record::{extract_record, to_canonical_markdown};
use crate::store::DocStore;
use std::path::{Path, PathBuf};

const NON_CANONICAL_MESSAGE: &str =
    "non-canonical (hand-written?) frontmatter — run `living-docs fmt` or author via the CLI verbs";

const UNWRITABLE_MESSAGE: &str = "'{key}' cannot be written canonically; edit the file by hand";

/// True when the record sits directly inside one of the CLI-owned type
/// directories (`paths::doc_type_for_dir` — ADR 0020's scope, applied to the
/// check layer by ADR 0022). Only those records are scaffolded by `new`, so
/// only they can be expected to byte-match canonical serialization.
pub(crate) fn in_cli_owned_dir(path: &Path) -> bool {
    path.parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .and_then(doc_type_for_dir)
        .is_some()
}

/// Flags every non-reserved record the CLI can produce byte-for-byte whose
/// on-disk frontmatter block differs from its canonical re-serialization: a
/// record inside a CLI-owned type directory (`new`'s numbered types,
/// including `research` since ADR 0026), and a bundle-root registry
/// [`crate::doc_type::Identity::Singleton`] file such as `constitution.md`
/// (ADR 0026 decision point 7). A record outside both — a hand-authored
/// bundle-root note, or the same filename nested in a subdirectory — is out
/// of scope, and a record carrying no frontmatter block at all is the
/// existing untyped-doc check's concern; both are skipped here.
pub(crate) fn check_canonical_frontmatter(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    for path in all_md {
        let owned = in_cli_owned_dir(path) || is_bundle_singleton(bundle, path);
        if is_reserved(&file_name_str(path)) || !owned {
            continue;
        }
        let Ok(contents) = store.read(path) else {
            continue;
        };
        let Some(on_disk_block) = frontmatter_block(&contents) else {
            continue;
        };
        let canonical = to_canonical_markdown(&extract_record(path, &contents));
        if let Some(key) = unwritable_key(&contents, &canonical) {
            reporter.report(path, UNWRITABLE_MESSAGE.replace("{key}", &key));
            continue;
        }
        let canonical_block = frontmatter_block(&canonical).unwrap_or_default();
        if on_disk_block != canonical_block {
            reporter.report(path, NON_CANONICAL_MESSAGE);
        }
    }
}

#[cfg(test)]
mod tests;
