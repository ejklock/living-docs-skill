use super::*;
use crate::record::{extract_record, to_canonical_markdown};
use crate::test_support::MapStore;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn file_name_str_returns_the_basename() {
    assert_eq!(file_name_str(Path::new("docs/adr/index.md")), "index.md");
}

#[test]
fn reporter_with_no_violations_reports_clean_and_exits_zero() {
    let reporter = Reporter::new();
    let code = reporter.finish(3);
    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
}

struct ScratchBundle {
    root: PathBuf,
}

impl ScratchBundle {
    fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("living-docs-check-mod-{label}-{nanos}"));
        fs::create_dir_all(root.join("adr")).expect("create scratch adr dir");
        fs::write(
            root.join("index.md"),
            "# Index\n\n- [ADRs](/adr/index.md)\n",
        )
        .expect("write scratch root index");
        fs::write(
            root.join("adr").join("index.md"),
            "# ADR Index\n\n- [Doc](/adr/0001-doc.md)\n- [Other](/adr/0002-other.md)\n",
        )
        .expect("write scratch adr index");
        Self { root }
    }
}

impl Drop for ScratchBundle {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A `DocStore` that reads and enumerates real files on disk, mirroring
/// `fs-store`'s adapter closely enough to exercise `is_bundle_singleton`'s
/// rooting invariant against a genuine filesystem rather than `MapStore`'s
/// in-memory fixture.
struct RealFsStore;

impl DocStore for RealFsStore {
    fn list(&self, root: &Path) -> std::io::Result<Vec<PathBuf>> {
        Ok(collect_md_files(root))
    }

    fn read(&self, path: &Path) -> std::io::Result<String> {
        fs::read_to_string(path)
    }

    fn write(&self, path: &Path, contents: &str) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)
    }
}

/// Pins the invariant `is_bundle_singleton`'s docblock names: an unlisted
/// bundle-root singleton stays exempt from every `check` invariant
/// whether the caller passes the bundle directly or reaches the same
/// directory through a symlink, because both `store.list` and
/// `is_bundle_singleton` build their paths from whatever root they were
/// handed rather than a resolved one.
#[cfg(unix)]
#[test]
fn is_bundle_singleton_stays_exempt_regardless_of_the_path_form_the_caller_used() {
    let bundle = ScratchBundle::new("singleton-symlink-invariant");
    fs::write(bundle.root.join("adr").join("index.md"), "# ADR Index\n")
        .expect("clear scratch adr index of phantom links");

    let singleton_path = bundle.root.join("constitution.md");
    let hand_written = "---\ntype: Constitution\ntitle: Root of Trust\n---\n\nBody.\n";
    let canonical = to_canonical_markdown(&extract_record(&singleton_path, hand_written));
    fs::write(&singleton_path, &canonical).expect("write scratch constitution.md");

    let direct_violations = check_violations(&RealFsStore, &bundle.root);
    assert!(
        direct_violations.is_empty(),
        "direct violations: {direct_violations:?}"
    );

    let symlink_root = std::env::temp_dir().join(format!(
        "living-docs-check-mod-singleton-symlink-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos()
    ));
    std::os::unix::fs::symlink(&bundle.root, &symlink_root)
        .expect("create symlink to scratch bundle");

    let via_symlink_violations = check_violations(&RealFsStore, &symlink_root);
    let _ = fs::remove_file(&symlink_root);

    assert!(
        via_symlink_violations.is_empty(),
        "via-symlink violations: {via_symlink_violations:?}"
    );
}

/// The positive direction `check_canonical_frontmatter_flags_every_cli_owned_directory`
/// and its `canonical.rs` siblings cannot instrument: since their fixture
/// paths are built from `spec` too, a premise guard is the only thing
/// that would fail after a registry rename, not the behavioral
/// assertion. Building both the matching and non-matching paths from
/// `spec` here means a rename keeps this test green for the *new* name,
/// proving `is_bundle_singleton` follows the registry rather than a
/// literal filename.
#[test]
fn is_bundle_singleton_follows_the_registry_not_a_literal_filename() {
    let bundle = Path::new("/bundle");
    let mut exercised = 0;

    for spec in doc_type::DOC_TYPES {
        let Identity::Singleton { file } = spec.identity else {
            continue;
        };
        exercised += 1;

        assert!(
            is_bundle_singleton(bundle, &bundle.join(file)),
            "{file} at the bundle root must be its own singleton"
        );
        assert!(
            !is_bundle_singleton(bundle, &bundle.join("not-a-singleton.md")),
            "a sibling filename must never match {file}'s singleton slot"
        );
        assert!(
            !is_bundle_singleton(bundle, &bundle.join("reference").join(file)),
            "{file} nested one level down must not match the bundle-root singleton"
        );
    }

    assert!(
        exercised > 0,
        "no Identity::Singleton row in DOC_TYPES — this test would pass vacuously"
    );
}

/// A record served only by the store (never written to disk) must still
/// pass `check` — proving content reads route through `DocStore::read`
/// rather than falling back to `std::fs::read_to_string` at the same
/// path.
#[test]
fn run_reads_record_content_through_the_store_even_when_no_file_backs_it_on_disk() {
    let bundle = ScratchBundle::new("store-only-record");
    let mut files = BTreeMap::new();
    files.insert(
        bundle.root.join("adr").join("0001-doc.md"),
        "---\ntype: ADR\ntitle: Doc\ndescription: \"\"\n---\n\n# Doc\n\nBody.\n".to_string(),
    );
    let store = MapStore { files };

    let code = run(&store, &bundle.root);

    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
}

/// A supersede target that a real file on disk would satisfy, but that
/// the active store never enumerates, must still fail `check` — proving
/// the sibling lookup is driven by the store's own enumeration
/// (`DocStore::list`), not a filesystem re-scan.
#[test]
fn run_reports_a_supersede_target_the_store_omits_even_though_a_same_named_file_exists_on_disk()
{
    let bundle = ScratchBundle::new("store-omits-target");
    fs::write(
        bundle.root.join("adr").join("0002-other.md"),
        "---\ntype: ADR\ntitle: Other\n---\n# Other\n\nBody.\n",
    )
    .expect("write on-disk supersede target");
    let mut files = BTreeMap::new();
    files.insert(
        bundle.root.join("adr").join("0001-doc.md"),
        "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0002\n---\n# Doc\n\nBody.\n"
            .to_string(),
    );
    let store = MapStore { files };

    let code = run(&store, &bundle.root);

    assert_ne!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
}
