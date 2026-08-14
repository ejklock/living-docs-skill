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
use crate::frontmatter::frontmatter_block;
use crate::paths::doc_type_for_dir;
use crate::record::{extract_record, to_canonical_markdown};
use crate::store::DocStore;
use std::path::{Path, PathBuf};

const NON_CANONICAL_MESSAGE: &str =
    "non-canonical (hand-written?) frontmatter — run `living-docs fmt` or author via the CLI verbs";

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
        let canonical_block = frontmatter_block(&canonical).unwrap_or_default();
        if on_disk_block != canonical_block {
            reporter.report(path, NON_CANONICAL_MESSAGE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MapStore;
    use std::collections::BTreeMap;
    use std::path::Path;

    fn store_with(path: &str, contents: &str) -> (MapStore, Vec<PathBuf>) {
        let mut files = BTreeMap::new();
        files.insert(PathBuf::from(path), contents.to_owned());
        let all_md = vec![PathBuf::from(path)];
        (MapStore { files }, all_md)
    }

    #[test]
    fn frontmatter_block_slices_between_the_fences() {
        assert_eq!(
            frontmatter_block("---\ntype: ADR\n---\n\nBody.\n"),
            Some("type: ADR")
        );
    }

    #[test]
    fn frontmatter_block_is_none_without_a_leading_fence() {
        assert_eq!(frontmatter_block("# No frontmatter\n"), None);
    }

    #[test]
    fn check_canonical_frontmatter_accepts_an_already_canonical_record() {
        let canonical = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nBody.\n";
        let (store, all_md) = store_with("/bundle/adr/0001-doc.md", canonical);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

        assert!(reporter.into_violations().is_empty());
    }

    #[test]
    fn check_canonical_frontmatter_flags_a_trailing_yaml_comment() {
        let commented = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.  # a comment\n---\n\n# Quokka Caching\n\nBody.\n";
        let (store, all_md) = store_with("/bundle/adr/0001-doc.md", commented);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

        let violations = reporter.into_violations();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].1.contains("living-docs fmt"));
    }

    #[test]
    fn check_canonical_frontmatter_flags_reordered_keys() {
        let reordered = "---\ntitle: Quokka Caching\ntype: ADR\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nBody.\n";
        let (store, all_md) = store_with("/bundle/adr/0001-doc.md", reordered);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

        assert_eq!(reporter.into_violations().len(), 1);
    }

    #[test]
    fn check_canonical_frontmatter_flags_extra_spacing_around_a_value() {
        let spaced = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.   \n---\n\n# Quokka Caching\n\nBody.\n";
        let (store, all_md) = store_with("/bundle/adr/0001-doc.md", spaced);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

        assert_eq!(reporter.into_violations().len(), 1);
    }

    #[test]
    fn check_canonical_frontmatter_skips_records_outside_cli_owned_directories() {
        let outside_dir = "reference";
        assert!(
            crate::doc_type::spec_for_dir(outside_dir).is_none(),
            "fixture premise broken: `{outside_dir}` is now a registry-owned directory — pick another",
        );

        let commented = "---\ntype: Research\ntitle: Field Notes  # a comment\n---\n\nBody.\n";
        for path in [
            format!("/bundle/{outside_dir}/0001-notes.md"),
            "/bundle/0001-notes.md".to_string(),
        ] {
            let (store, all_md) = store_with(&path, commented);
            let mut reporter = Reporter::new();

            check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

            assert!(reporter.into_violations().is_empty());
        }
    }

    #[test]
    fn check_canonical_frontmatter_flags_every_cli_owned_directory() {
        let commented = "---\ntype: ADR\ntitle: X  # comment\n---\n\nBody.\n";
        for spec in crate::doc_type::DOC_TYPES {
            let crate::doc_type::Identity::Numbered { dir } = spec.identity else {
                continue;
            };
            let path = format!("/bundle/{dir}/0001-doc.md");
            let (store, all_md) = store_with(&path, commented);
            let mut reporter = Reporter::new();

            check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

            assert_eq!(reporter.into_violations().len(), 1);
        }
    }

    /// Guards the fixture premise the same way
    /// `check_canonical_frontmatter_skips_records_outside_cli_owned_directories`
    /// guards its own: the singleton filename these tests hardcode is driven
    /// off `doc_type::DOC_TYPES`, so a renamed row fails this assertion
    /// loudly instead of the fixture silently exercising the wrong path.
    fn assert_constitution_md_is_still_the_registered_singleton() {
        let spec = crate::doc_type::spec_for("constitution")
            .expect("fixture premise broken: `constitution` is no longer a registered token");
        assert_eq!(
            spec.identity,
            crate::doc_type::Identity::Singleton {
                file: "constitution.md"
            },
            "fixture premise broken: the constitution row no longer names constitution.md — update this fixture",
        );
    }

    #[test]
    fn check_canonical_frontmatter_flags_a_hand_written_bundle_root_singleton() {
        assert_constitution_md_is_still_the_registered_singleton();
        let commented = "---\ntype: Constitution\ntitle: X  # comment\n---\n\nBody.\n".to_string();
        let (store, all_md) = store_with("/bundle/constitution.md", &commented);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

        let violations = reporter.into_violations();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].1.contains("living-docs fmt"));
    }

    #[test]
    fn check_canonical_frontmatter_does_not_flag_the_singleton_filename_nested_in_a_subdirectory() {
        assert_constitution_md_is_still_the_registered_singleton();
        let commented = "---\ntype: Constitution\ntitle: X  # comment\n---\n\nBody.\n".to_string();
        let (store, all_md) = store_with("/bundle/reference/constitution.md", &commented);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

        assert!(reporter.into_violations().is_empty());
    }

    #[test]
    fn check_canonical_frontmatter_accepts_the_canonical_bundle_root_singleton() {
        assert_constitution_md_is_still_the_registered_singleton();
        let path = Path::new("/bundle/constitution.md");
        let hand_written = "---\ntype: Constitution\ntitle: X\n---\n\nBody.\n";
        let canonical = to_canonical_markdown(&extract_record(path, hand_written));
        let (store, all_md) = store_with("/bundle/constitution.md", &canonical);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

        assert!(reporter.into_violations().is_empty());
    }

    #[test]
    fn check_canonical_frontmatter_skips_a_record_with_no_frontmatter_block() {
        let (store, all_md) = store_with("/bundle/adr/notes.md", "# Just a heading\n\nBody.\n");
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

        assert!(reporter.into_violations().is_empty());
    }

    #[test]
    fn check_canonical_frontmatter_skips_reserved_files() {
        let commented = "---\ntype: ADR\ntitle: X  # comment\n---\n\nBody.\n";
        let (store, all_md) = store_with("/bundle/adr/index.md", commented);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

        assert!(reporter.into_violations().is_empty());
    }

    #[test]
    fn check_canonical_frontmatter_accepts_the_same_record_after_a_fmt_pass() {
        let commented = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.  # a comment\n---\n\n# Quokka Caching\n\nBody.\n";
        let path = Path::new("/bundle/adr/0001-doc.md");
        let fmt_pass = to_canonical_markdown(&extract_record(path, commented));
        let (store, all_md) = store_with("/bundle/adr/0001-doc.md", &fmt_pass);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

        assert!(reporter.into_violations().is_empty());
    }
}
