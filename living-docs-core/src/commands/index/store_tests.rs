use super::*;
use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;

/// A minimal in-memory [`DocStore`] test double, proving `collect_records`
/// reads a record's title/status through the port rather than the
/// filesystem — the same double pattern used by `export.rs`/`new.rs`.
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

#[test]
fn compute_returns_the_index_path_and_content_regenerate_would_write_without_touching_disk() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/adr/0001-first.md"),
        "---\ntype: ADR\ntitle: First\nstatus: Accepted\n---\n# First\n".to_string(),
    );
    let store = MapStore { files };

    let (index_path, content) =
        compute(&store, Path::new("/bundle"), "adr", None).expect("compute should succeed");

    assert_eq!(index_path, PathBuf::from("/bundle/adr/index.md"));
    assert_eq!(
        content,
        "# ADRs\n\n## Active\n\n* [0001 — First](0001-first.md) - Accepted\n"
    );
    assert!(
        !index_path.exists(),
        "compute must not write anything to disk"
    );
}

#[test]
fn regenerate_is_a_no_op_when_the_type_directory_does_not_exist() {
    let store = MapStore {
        files: BTreeMap::new(),
    };
    let docs_dir = std::env::temp_dir().join(format!(
        "living-docs-index-regenerate-noop-{}",
        std::process::id()
    ));
    let type_dir = docs_dir.join("research");
    assert!(!type_dir.exists());

    let result = regenerate(&store, &docs_dir, "research", None);

    assert!(result.is_ok());
    assert!(
        !type_dir.exists(),
        "regenerate must not create the type directory when it is absent"
    );
}

#[test]
fn compute_rejects_an_unsupported_doc_type() {
    let store = MapStore {
        files: BTreeMap::new(),
    };

    let result = compute(&store, Path::new("/bundle"), "glossary", None);

    assert!(result.is_err());
}

/// `index constitution` gets its own message, not the unsupported-type
/// one — the type IS supported, it just has no directory index, and the
/// unsupported-type message would list the very token the caller used.
#[test]
fn compute_rejects_an_explicit_constitution_index_with_its_own_message() {
    let store = MapStore {
        files: BTreeMap::new(),
    };

    let err = compute(&store, Path::new("/bundle"), "constitution", None)
        .expect_err("constitution has no directory index");

    assert!(err.contains("constitution.md"), "got: {err}");
    assert!(
        !err.contains("expected one of"),
        "must not reuse the unsupported-type message: {err}"
    );
}

#[test]
fn title_for_record_prefers_a_present_frontmatter_title_over_the_h1_heading() {
    let contents = "---\ntitle: Frontmatter Title\n---\n# ADR 0007 — Heading Title\n";
    let title = title_for_record(contents, Path::new("adr/0007-x.md"), 7);
    assert_eq!(title, "Frontmatter Title");
}

#[test]
fn title_for_record_falls_back_to_the_h1_heading_stripping_the_adr_number_prefix() {
    let contents = "---\ntype: ADR\n---\n# ADR 0007 — Heading Title\n";
    let title = title_for_record(contents, Path::new("adr/0007-x.md"), 7);
    assert_eq!(title, "Heading Title");
}

#[test]
fn title_for_record_falls_back_to_the_h1_heading_stripping_a_bare_numbered_dot_prefix() {
    let contents = "---\ntype: ADR\n---\n# 0007. Heading Title\n";
    let title = title_for_record(contents, Path::new("adr/0007-x.md"), 7);
    assert_eq!(title, "Heading Title");
}

#[test]
fn title_for_record_falls_back_to_the_h1_heading_stripping_a_bare_numbered_dash_prefix() {
    let contents = "---\ntype: ADR\n---\n# 0007 — Heading Title\n";
    let title = title_for_record(contents, Path::new("adr/0007-x.md"), 7);
    assert_eq!(title, "Heading Title");
}

#[test]
fn title_for_record_is_empty_when_neither_frontmatter_nor_h1_carry_a_title() {
    let contents = "---\ntype: ADR\n---\nBody with no heading.\n";
    let title = title_for_record(contents, Path::new("adr/0007-x.md"), 7);
    assert_eq!(title, "");
}

#[test]
fn collect_records_reads_title_and_status_through_the_store() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/adr/0001-first.md"),
        "---\ntype: ADR\ntitle: First\nstatus: Accepted\n---\n# First\n".to_string(),
    );
    let store = MapStore { files };

    let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
        .expect("collect_records should succeed");

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].title, "First");
    assert_eq!(records[0].status, "Accepted");
}

#[test]
fn collect_records_ignores_paths_the_store_lists_outside_the_type_directory() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/adr/0001-in-scope.md"),
        "---\ntype: ADR\ntitle: In Scope\nstatus: Proposed\n---\n# In Scope\n".to_string(),
    );
    files.insert(
        PathBuf::from("/bundle/bdr/0001-other-type.md"),
        "---\ntype: BDR\ntitle: Other Type\nstatus: Draft\n---\n# Other Type\n".to_string(),
    );
    let store = MapStore { files };

    let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
        .expect("collect_records should succeed");

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].filename, "0001-in-scope.md");
}

#[test]
fn collect_records_on_an_empty_store_returns_no_records() {
    let store = MapStore {
        files: BTreeMap::new(),
    };

    let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
        .expect("collect_records should succeed on an empty store");

    assert!(records.is_empty());
}

#[test]
fn collect_records_defaults_to_private_when_visibility_is_absent() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/adr/0001-first.md"),
        "---\ntype: ADR\ntitle: First\nstatus: Accepted\n---\n# First\n".to_string(),
    );
    let store = MapStore { files };

    let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
        .expect("collect_records should succeed");

    assert_eq!(records[0].visibility, "private");
}

#[test]
fn collect_records_reads_an_explicit_visibility_value() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/adr/0001-first.md"),
        "---\ntype: ADR\ntitle: First\nstatus: Accepted\nvisibility: public\n---\n# First\n"
            .to_string(),
    );
    let store = MapStore { files };

    let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
        .expect("collect_records should succeed");

    assert_eq!(records[0].visibility, "public");
}

fn record_with_visibility(visibility: &str) -> Record {
    Record {
        number: 1,
        title: "Title".to_string(),
        status: "Accepted".to_string(),
        filename: "0001-title.md".to_string(),
        visibility: visibility.to_string(),
    }
}

#[test]
fn record_visible_passes_every_record_when_the_filter_is_none() {
    assert!(record_visible(&record_with_visibility("private"), None));
    assert!(record_visible(&record_with_visibility("public"), None));
}

#[test]
fn record_visible_excludes_a_record_outside_the_filter_set() {
    let filter = vec!["public".to_string(), "showcase".to_string()];
    assert!(!record_visible(
        &record_with_visibility("private"),
        Some(&filter)
    ));
}

#[test]
fn record_visible_includes_a_record_inside_the_filter_set() {
    let filter = vec!["public".to_string(), "showcase".to_string()];
    assert!(record_visible(
        &record_with_visibility("public"),
        Some(&filter)
    ));
    assert!(record_visible(
        &record_with_visibility("showcase"),
        Some(&filter)
    ));
}

#[test]
fn record_visible_default_deny_only_admits_private_when_explicitly_requested() {
    let private_filter = vec!["private".to_string()];
    let public_filter = vec!["public".to_string()];
    let absent_visibility = record_with_visibility(DEFAULT_VISIBILITY);

    assert!(record_visible(&absent_visibility, Some(&private_filter)));
    assert!(!record_visible(&absent_visibility, Some(&public_filter)));
}
