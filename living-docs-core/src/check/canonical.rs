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
use super::{file_name_str, Reporter};
use crate::record::{extract_record, to_canonical_markdown};
use crate::store::DocStore;
use std::path::PathBuf;

const NON_CANONICAL_MESSAGE: &str =
    "non-canonical (hand-written?) frontmatter — run `living-docs fmt` or author via the CLI verbs";

/// Flags every non-reserved record in `all_md` whose on-disk frontmatter
/// block differs from its canonical re-serialization. A record carrying no
/// frontmatter block at all is the existing untyped-doc check's concern and
/// is skipped here.
pub(crate) fn check_canonical_frontmatter(
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

/// The raw text between the opening `---` fence and its closing `---`,
/// mirroring `crate::record`'s own private frontmatter-slicing helper —
/// duplicated rather than exposed across the crate boundary because it is a
/// three-line string slice, not the canonical model itself.
fn frontmatter_block(contents: &str) -> Option<&str> {
    let rest = contents.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::io;
    use std::path::Path;

    struct MapStore {
        files: BTreeMap<PathBuf, String>,
    }

    impl DocStore for MapStore {
        fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(self
                .files
                .keys()
                .filter(|path| path.starts_with(root))
                .cloned()
                .collect())
        }

        fn read(&self, path: &Path) -> io::Result<String> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
        }

        fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
            Ok(())
        }
    }

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

        check_canonical_frontmatter(&store, &all_md, &mut reporter);

        assert!(reporter.into_violations().is_empty());
    }

    #[test]
    fn check_canonical_frontmatter_flags_a_trailing_yaml_comment() {
        let commented = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.  # a comment\n---\n\n# Quokka Caching\n\nBody.\n";
        let (store, all_md) = store_with("/bundle/adr/0001-doc.md", commented);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, &all_md, &mut reporter);

        let violations = reporter.into_violations();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].1.contains("living-docs fmt"));
    }

    #[test]
    fn check_canonical_frontmatter_flags_reordered_keys() {
        let reordered = "---\ntitle: Quokka Caching\ntype: ADR\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nBody.\n";
        let (store, all_md) = store_with("/bundle/adr/0001-doc.md", reordered);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, &all_md, &mut reporter);

        assert_eq!(reporter.into_violations().len(), 1);
    }

    #[test]
    fn check_canonical_frontmatter_flags_extra_spacing_around_a_value() {
        let spaced = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.   \n---\n\n# Quokka Caching\n\nBody.\n";
        let (store, all_md) = store_with("/bundle/adr/0001-doc.md", spaced);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, &all_md, &mut reporter);

        assert_eq!(reporter.into_violations().len(), 1);
    }

    #[test]
    fn check_canonical_frontmatter_skips_a_record_with_no_frontmatter_block() {
        let (store, all_md) = store_with("/bundle/notes.md", "# Just a heading\n\nBody.\n");
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, &all_md, &mut reporter);

        assert!(reporter.into_violations().is_empty());
    }

    #[test]
    fn check_canonical_frontmatter_skips_reserved_files() {
        let commented = "---\ntype: ADR\ntitle: X  # comment\n---\n\nBody.\n";
        let (store, all_md) = store_with("/bundle/index.md", commented);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, &all_md, &mut reporter);

        assert!(reporter.into_violations().is_empty());
    }

    #[test]
    fn check_canonical_frontmatter_accepts_the_same_record_after_a_fmt_pass() {
        let commented = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.  # a comment\n---\n\n# Quokka Caching\n\nBody.\n";
        let path = Path::new("/bundle/adr/0001-doc.md");
        let fmt_pass = to_canonical_markdown(&extract_record(path, commented));
        let (store, all_md) = store_with("/bundle/adr/0001-doc.md", &fmt_pass);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, &all_md, &mut reporter);

        assert!(reporter.into_violations().is_empty());
    }
}
