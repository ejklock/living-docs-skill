use super::*;
use std::io::ErrorKind;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let base = std::env::temp_dir().join(format!("fs-store-test-{label}-{nanos}"));
        fs::create_dir_all(&base).expect("create temp test dir");
        let path = base.canonicalize().expect("canonicalize temp test dir");
        Self { path }
    }

    fn bundle(&self) -> PathBuf {
        let bundle = self.path.join("bundle");
        fs::create_dir_all(bundle.join("adr")).expect("create bundle");
        bundle
    }

    fn outside(&self) -> PathBuf {
        let outside = self.path.join("outside");
        fs::create_dir_all(&outside).expect("create outside dir");
        outside
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(unix)]
fn link(target: &Path, at: &Path) {
    std::os::unix::fs::symlink(target, at).expect("create symlink");
}

fn assert_refused(result: io::Result<impl std::fmt::Debug>, path: &Path, bundle: &Path) {
    let error = result.expect_err("the store must refuse a path outside the bundle");
    assert_eq!(error.kind(), ErrorKind::PermissionDenied, "{error}");
    let message = error.to_string();
    assert!(message.contains(&path.display().to_string()), "{message}");
    assert!(message.contains(&bundle.display().to_string()), "{message}");
    assert!(
        message.contains("a record must lie inside the bundle"),
        "{message}"
    );
}

#[test]
fn list_enumerates_only_md_files_in_sorted_order() {
    let temp = TempTestDir::new("list-sorted");
    let root = temp.path.clone();
    fs::create_dir_all(root.join("nested")).expect("create nested dir");
    fs::write(root.join("b.md"), "b").expect("write b.md");
    fs::write(root.join("a.md"), "a").expect("write a.md");
    fs::write(root.join("notes.txt"), "not markdown").expect("write notes.txt");
    fs::write(root.join("nested").join("c.md"), "c").expect("write nested c.md");

    let found = FsStore::rooted(&root)
        .list(&root)
        .expect("list should succeed");

    assert_eq!(
        found,
        vec![
            root.join("a.md"),
            root.join("b.md"),
            root.join("nested").join("c.md"),
        ]
    );
}

#[test]
fn list_on_missing_root_returns_empty_result() {
    let temp = TempTestDir::new("list-missing-root");
    let missing_root = temp.path.join("does-not-exist");

    let found = FsStore::rooted(&missing_root)
        .list(&missing_root)
        .expect("missing root should be a lenient empty list, not an error");

    assert!(found.is_empty());
}

#[test]
fn write_then_read_round_trips_contents() {
    let temp = TempTestDir::new("round-trip");
    let bundle = temp.bundle();
    let target = bundle.join("doc.md");
    let store = FsStore::rooted(&bundle);

    store
        .write(&target, "# Title\n\nBody text.\n")
        .expect("write should succeed");
    let read_back = store.read(&target).expect("read should succeed");

    assert_eq!(read_back, "# Title\n\nBody text.\n");
}

#[test]
fn write_creates_missing_parent_directories() {
    let temp = TempTestDir::new("write-parents");
    let bundle = temp.bundle();
    let target = bundle.join("adr").join("nested").join("0001-title.md");

    FsStore::rooted(&bundle)
        .write(&target, "content")
        .expect("write should create missing parents and succeed");

    assert_eq!(
        fs::read_to_string(&target).expect("read written file"),
        "content"
    );
}

#[test]
fn a_bundle_that_does_not_exist_yet_is_created_by_the_first_write() {
    let temp = TempTestDir::new("fresh-bundle");
    let bundle = temp.path.join("fresh").join("bundle");
    let target = bundle.join("adr").join("0001-x.md");

    FsStore::rooted(&bundle)
        .write(&target, "first")
        .expect("a missing bundle is created, not refused");

    assert_eq!(fs::read_to_string(&target).expect("read"), "first");
}

#[cfg(unix)]
#[test]
fn a_symlinked_record_pointing_inside_the_bundle_reads_and_writes() {
    let temp = TempTestDir::new("link-inside");
    let bundle = temp.bundle();
    fs::write(bundle.join("adr/0001-real.md"), "real").expect("write real");
    let alias = bundle.join("adr/0002-link.md");
    link(&bundle.join("adr/0001-real.md"), &alias);
    let store = FsStore::rooted(&bundle);

    assert_eq!(store.read(&alias).unwrap(), "real");
    store.write(&alias, "changed").unwrap();
    assert_eq!(
        fs::read_to_string(bundle.join("adr/0001-real.md")).unwrap(),
        "changed"
    );
}

#[cfg(unix)]
#[test]
fn a_symlinked_record_pointing_outside_is_refused_and_its_target_kept() {
    let temp = TempTestDir::new("link-outside");
    let bundle = temp.bundle();
    let target = temp.outside().join("secret.md");
    fs::write(&target, "outside bytes").expect("write outside");
    let record = bundle.join("adr/0001-link.md");
    link(&target, &record);
    let store = FsStore::rooted(&bundle);

    assert_refused(store.read(&record), &record, &bundle);
    assert_refused(store.write(&record, "overwritten"), &record, &bundle);

    assert_eq!(fs::read_to_string(&target).unwrap(), "outside bytes");
}

#[cfg(unix)]
#[test]
fn a_type_directory_linked_outside_refuses_a_nested_write_and_creates_nothing() {
    let temp = TempTestDir::new("dir-outside");
    let bundle = temp.path.join("bundle");
    fs::create_dir_all(&bundle).unwrap();
    let outside = temp.outside();
    link(&outside, &bundle.join("adr"));
    let record = bundle.join("adr/nested/0001-x.md");

    assert_refused(
        FsStore::rooted(&bundle).write(&record, "x"),
        &record,
        &bundle,
    );

    assert!(!outside.join("nested").exists());
}

#[cfg(unix)]
#[test]
fn a_rename_into_a_directory_linked_outside_is_refused_and_the_source_kept() {
    let temp = TempTestDir::new("rename-into-outside");
    let bundle = temp.bundle();
    let outside = temp.outside();
    link(&outside, &bundle.join("issues"));
    let source = bundle.join("adr/0001-x.md");
    fs::write(&source, "x").unwrap();
    let destination = bundle.join("issues/0001-x.md");

    assert_refused(
        FsStore::rooted(&bundle).rename(&source, &destination),
        &destination,
        &bundle,
    );

    assert!(source.is_file());
    assert!(!outside.join("0001-x.md").exists());
}

#[cfg(unix)]
#[test]
fn a_rename_whose_source_links_outside_is_refused() {
    let temp = TempTestDir::new("rename-from-outside");
    let bundle = temp.bundle();
    let target = temp.outside().join("secret.md");
    fs::write(&target, "outside bytes").unwrap();
    let source = bundle.join("adr/0001-link.md");
    link(&target, &source);
    let destination = bundle.join("adr/0002-moved.md");

    assert_refused(
        FsStore::rooted(&bundle).rename(&source, &destination),
        &source,
        &bundle,
    );

    assert!(fs::symlink_metadata(&source).is_ok());
    assert!(!destination.exists());
    assert_eq!(fs::read_to_string(&target).unwrap(), "outside bytes");
}

#[test]
fn a_missing_path_that_climbs_out_with_parent_components_is_refused() {
    let temp = TempTestDir::new("dotdot");
    let bundle = temp.bundle();
    let escape = bundle.join("../escape.md");
    let store = FsStore::rooted(&bundle);

    assert_refused(store.write(&escape, "x"), &escape, &bundle);
    assert!(!temp.path.join("escape.md").exists());

    let hidden = bundle.join("missing/../../escape.md");
    assert_refused(store.write(&hidden, "x"), &hidden, &bundle);
    assert!(!temp.path.join("escape.md").exists());
}

fn assert_dangling_refused(result: io::Result<()>, record: &Path) {
    let error = result.expect_err("a dangling link must be refused");
    assert_eq!(error.kind(), ErrorKind::PermissionDenied, "{error}");
    let message = error.to_string();
    let expected = [
        (record.display().to_string(), true),
        ("dangling link cannot be judged".to_string(), true),
        ("resolves outside".to_string(), false),
    ];
    for (needle, present) in expected {
        assert_eq!(message.contains(&needle), present, "{message}");
    }
}

#[cfg(unix)]
#[test]
fn a_dangling_symlink_record_is_refused_as_dangling_whatever_its_target() {
    let temp = TempTestDir::new("dangling");
    let bundle = temp.bundle();
    let store = FsStore::rooted(&bundle);
    let targets = [
        (temp.outside().join("not-yet.md"), "adr/0001-dangling.md"),
        (bundle.join("adr/0042-future.md"), "adr/0004-dangling.md"),
    ];
    for (target, name) in targets {
        let record = bundle.join(name);
        link(&target, &record);

        assert_dangling_refused(store.read(&record).map(|_| ()), &record);
        assert_dangling_refused(store.write(&record, "x"), &record);
        assert!(!target.exists());
    }
}

#[cfg(unix)]
#[test]
fn a_bundle_reached_through_a_symlinked_root_reads_and_writes() {
    let temp = TempTestDir::new("linked-root");
    let bundle = temp.bundle();
    let alias = temp.path.join("alias");
    link(&bundle, &alias);
    let store = FsStore::rooted(&alias);
    let record = alias.join("adr/0001-x.md");

    store.write(&record, "via alias").unwrap();

    assert_eq!(store.read(&record).unwrap(), "via alias");
    assert_eq!(
        fs::read_to_string(bundle.join("adr/0001-x.md")).unwrap(),
        "via alias"
    );
}
