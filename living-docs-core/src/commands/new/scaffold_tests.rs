use super::*;
use crate::store::DocStore;
use crate::test_support::WritableMapStore as MapStore;
use std::io;
use std::path::{Path, PathBuf};

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
    inner: crate::test_support::MapStore,
}

impl DocStore for StaleListingStore {
    fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.inner.read(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        self.inner.write(path, contents)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, to)
    }
}

#[test]
fn scaffold_refuses_to_clobber_a_path_the_store_already_serves_even_when_listing_omits_it() {
    let (inner, _all_md) = crate::test_support::MapStore::seeded(&[(
        "/bundle/adr/0001-first-decision.md",
        "existing",
    )]);
    let store = StaleListingStore { inner };

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

#[test]
fn scaffold_places_a_nested_record_under_its_dir() {
    let store = MapStore::new();
    let opts = NewOptions {
        dir: Some("billing/invoices"),
        ..Default::default()
    };

    let target = scaffold(
        &store,
        Path::new("/bundle"),
        "domain",
        "Billing Rules",
        &opts,
        "2026-07-17T00:00:00Z",
    )
    .expect("a slug dir is accepted");

    assert_eq!(
        target,
        PathBuf::from("/bundle/context/billing/invoices/billing-rules.md")
    );
}

#[test]
fn scaffold_refuses_a_dir_with_a_parent_segment_and_writes_nothing() {
    let store = MapStore::new();
    let opts = NewOptions {
        dir: Some("a/../b"),
        ..Default::default()
    };

    let error = scaffold(
        &store,
        Path::new("/bundle"),
        "domain",
        "X",
        &opts,
        "2026-07-17T00:00:00Z",
    )
    .expect_err("a parent segment is refused");

    assert!(error.contains(".."), "{error}");
    assert!(store.read(Path::new("/bundle/context/b/x.md")).is_err());
}
