use super::*;
use crate::store::DocStore;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

/// A minimal in-memory [`DocStore`] test double, so `scaffold`'s tests
/// need no filesystem at all — `living-docs-core` depends on no
/// concrete adapter (issue 0006 slice 0006-D2).
struct MapStore {
    files: RefCell<BTreeMap<PathBuf, String>>,
}

impl MapStore {
    fn new() -> Self {
        Self {
            files: RefCell::new(BTreeMap::new()),
        }
    }

    fn seeded(seed: &[(&str, &str)]) -> Self {
        let files = seed
            .iter()
            .map(|(path, contents)| (PathBuf::from(path), (*contents).to_string()))
            .collect();
        Self {
            files: RefCell::new(files),
        }
    }
}

impl DocStore for MapStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(self
            .files
            .borrow()
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect())
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.files
            .borrow()
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), contents.to_string());
        Ok(())
    }
}

fn opts<'a>(description: Option<&'a str>, kind: Option<&'a str>) -> NewOptions<'a> {
    NewOptions {
        description,
        kind,
        ..Default::default()
    }
}

#[test]
fn scaffold_allocates_number_one_in_an_empty_type_directory() {
    let store = MapStore::new();

    let target = scaffold(
        &store,
        Path::new("/bundle"),
        "adr",
        "First Decision",
        &opts(None, None),
        "2026-07-17T00:00:00Z",
    )
    .expect("scaffold should succeed");

    assert_eq!(target, PathBuf::from("/bundle/adr/0001-first-decision.md"));
}

#[test]
fn scaffold_writes_a_singleton_constitution_with_no_number_or_slug() {
    let store = MapStore::new();

    let target = scaffold(
        &store,
        Path::new("/bundle"),
        "constitution",
        "Acme Constitution",
        &opts(None, None),
        "2026-07-17T00:00:00Z",
    )
    .expect("scaffold should succeed");

    assert_eq!(target, PathBuf::from("/bundle/constitution.md"));
    let persisted = store
        .read(&target)
        .expect("scaffold must persist through DocStore::write");
    assert!(persisted.contains("type: Constitution"));
    assert!(persisted.contains("title: Acme Constitution"));
}

#[test]
fn scaffold_refuses_a_second_constitution() {
    let store = MapStore::seeded(&[("/bundle/constitution.md", "existing content")]);

    let err = scaffold(
        &store,
        Path::new("/bundle"),
        "constitution",
        "Acme Constitution",
        &opts(None, None),
        "2026-07-17T00:00:00Z",
    )
    .expect_err("a second constitution must be refused");

    assert!(err.contains("already exists"), "got: {err}");
    assert_eq!(
        store.read(Path::new("/bundle/constitution.md")).unwrap(),
        "existing content"
    );
}

#[test]
fn scaffold_allocates_max_existing_number_plus_one_through_next_number_from_store() {
    let store = MapStore::seeded(&[
        ("/bundle/adr/0001-first.md", "content"),
        ("/bundle/adr/0004-fourth.md", "content"),
    ]);

    let target = scaffold(
        &store,
        Path::new("/bundle"),
        "adr",
        "Fifth Decision",
        &opts(None, None),
        "2026-07-17T00:00:00Z",
    )
    .expect("scaffold should succeed");

    assert_eq!(target, PathBuf::from("/bundle/adr/0005-fifth-decision.md"));
}

#[test]
fn scaffold_persists_the_filled_record_through_the_stores_write_method() {
    let store = MapStore::new();

    let target = scaffold(
        &store,
        Path::new("/bundle"),
        "adr",
        "Persisted Decision",
        &opts(None, None),
        "2026-07-17T00:00:00Z",
    )
    .expect("scaffold should succeed");

    let persisted = store
        .read(&target)
        .expect("scaffold must persist through DocStore::write");
    assert!(persisted.contains("type: ADR"));
    assert!(persisted.contains("status: Proposed"));
    assert!(persisted.contains("timestamp: 2026-07-17T00:00:00Z"));
    assert!(persisted.contains("title: Persisted Decision"));
}

#[test]
fn scaffold_seeds_the_description_placeholder_when_none_is_given() {
    let store = MapStore::new();

    let target = scaffold(
        &store,
        Path::new("/bundle"),
        "adr",
        "Placeholder Description",
        &opts(None, None),
        "2026-07-17T00:00:00Z",
    )
    .expect("scaffold should succeed");

    let persisted = store
        .read(&target)
        .expect("scaffold must persist through DocStore::write");
    assert!(
        persisted.contains("description: <One sentence"),
        "got: {persisted}"
    );
}

#[test]
fn scaffold_writes_the_given_description_when_some_is_passed() {
    let store = MapStore::new();

    let target = scaffold(
        &store,
        Path::new("/bundle"),
        "adr",
        "Described Decision",
        &opts(Some("A concise sentence describing the change."), None),
        "2026-07-17T00:00:00Z",
    )
    .expect("scaffold should succeed");

    let persisted = store
        .read(&target)
        .expect("scaffold must persist through DocStore::write");
    assert!(
        persisted.contains("description: A concise sentence describing the change."),
        "got: {persisted}"
    );
    assert!(!persisted.contains("<One sentence"));
}

/// `list` deliberately omits the record `read` still serves, simulating
/// a store whose enumeration and lookup can disagree — proving the
/// clobber guard checks `DocStore::read` directly rather than trusting
/// `DocStore::list`'s allocation to have already ruled the path out.
struct StaleListingStore {
    files: BTreeMap<PathBuf, String>,
}

impl DocStore for StaleListingStore {
    fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(Vec::new())
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
fn scaffold_refuses_to_clobber_a_path_the_store_already_serves_even_when_listing_omits_it() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/adr/0001-first-decision.md"),
        "existing".to_string(),
    );
    let store = StaleListingStore { files };

    let err = scaffold(
        &store,
        Path::new("/bundle"),
        "adr",
        "First Decision",
        &opts(None, None),
        "2026-07-17T00:00:00Z",
    )
    .expect_err("clobbering an existing store record must fail");

    assert!(err.contains("already exists"), "got: {err}");
}

#[test]
fn scaffold_writes_a_named_view_at_its_slug_with_the_kind_filled() {
    let store = MapStore::new();

    let target = scaffold(
        &store,
        Path::new("docs"),
        "view",
        "Container View",
        &opts(None, Some("container")),
        "2026-08-14T00:00:00Z",
    )
    .expect("a view scaffold must succeed");

    assert_eq!(target, PathBuf::from("docs/architecture/container-view.md"));
    let contents = store.read(&target).expect("scaffold must write the view");
    assert!(contents.starts_with("---\ntype: Architecture View\n"));
    assert!(contents.contains("kind: container\n"));
}
