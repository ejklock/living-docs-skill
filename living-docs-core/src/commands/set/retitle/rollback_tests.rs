use crate::commands::set::apply;
use crate::store::DocStore;
use crate::test_support::WritableMapStore as MapStore;
use std::cell::Cell;
use std::io;
use std::path::{Path, PathBuf};

const ADR_PATH: &str = "/bundle/adr/0001-a-decision.md";
const ADR: &str = "---\ntype: ADR\ntitle: A Decision\ndescription: A decision.\nstatus: Proposed\n---\n\n# 0001. A Decision\n\nBody.\n";
const INDEX: &str = "# ADR index\n\n- [0001](0001-a-decision.md)\n";
const REFERRER: &str = "See [the decision](../adr/0001-a-decision.md).\n";
const NOTES: [&str; 3] = [
    "/bundle/notes/a.md",
    "/bundle/notes/b.md",
    "/bundle/notes/c.md",
];

struct FlakyStore {
    inner: MapStore,
    writes_before_failure: Cell<usize>,
    rename_fails: bool,
    stays_failed: bool,
}

impl DocStore for FlakyStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        self.inner.list(root)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.inner.read(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        let left = self.writes_before_failure.get();
        if left == 0 {
            if !self.stays_failed {
                self.writes_before_failure.set(usize::MAX);
            }
            return Err(io::Error::other("disk full"));
        }
        self.writes_before_failure.set(left - 1);
        self.inner.write(path, contents)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        if self.rename_fails {
            return Err(io::Error::other("rename refused"));
        }
        self.inner.rename(from, to)
    }
}

fn flaky(writes_before_failure: usize, rename_fails: bool) -> FlakyStore {
    let mut seed = vec![(ADR_PATH, ADR), ("/bundle/adr/index.md", INDEX)];
    seed.extend(NOTES.iter().map(|path| (*path, REFERRER)));
    flaky_over(&seed, writes_before_failure, rename_fails)
}

fn flaky_over(
    seed: &[(&str, &str)],
    writes_before_failure: usize,
    rename_fails: bool,
) -> FlakyStore {
    FlakyStore {
        inner: MapStore::seeded(seed),
        writes_before_failure: Cell::new(writes_before_failure),
        rename_fails,
        stays_failed: false,
    }
}

fn snapshot(store: &FlakyStore) -> Vec<(PathBuf, String)> {
    let mut files: Vec<_> = store
        .list(Path::new("/"))
        .unwrap_or_default()
        .into_iter()
        .map(|path| {
            let text = store.inner.contents(&path.to_string_lossy());
            (path, text)
        })
        .collect();
    files.sort();
    files
}

fn assert_restored(store: &FlakyStore, key: &str, value: &str, message: &str) {
    assert_restored_ref(store, "adr/0001", key, value, message);
}

fn assert_restored_ref(store: &FlakyStore, reference: &str, key: &str, value: &str, message: &str) {
    let before = snapshot(store);
    let error = apply(store, Path::new("/bundle"), reference, key, value)
        .expect_err("the failing store refuses");
    assert!(error.contains(message), "{error}");
    assert_eq!(snapshot(store), before, "bundle changed");
}

#[test]
fn renumber_with_a_referrer_write_failing_after_the_rename_restores_the_bundle() {
    assert_restored(&flaky(3, false), "number", "0005", "disk full");
}

#[test]
fn retitle_with_a_referrer_write_failing_after_the_rename_restores_the_bundle() {
    let store = flaky(4, false);
    assert_restored(&store, "title", "Cache is write-through", "disk full");
}

#[test]
fn renumber_with_a_failing_rename_restores_the_bundle() {
    assert_restored(&flaky(99, true), "number", "0005", "rename refused");
}

#[test]
fn retitle_with_a_failing_rename_restores_the_bundle() {
    let store = flaky(99, true);
    assert_restored(&store, "title", "Cache is write-through", "rename refused");
}

const BLOCKER: &str =
    "---\ntype: Issue\ntitle: Blocker\ndescription: d\nstatus: open\n---\n\n# 0001. Blocker\n";
const BLOCKED: &str = "---\ntype: Issue\ntitle: Blocked\ndescription: d\nstatus: open\nblocked_by: [1]\n---\n\n# 0002. Blocked\n";

#[test]
fn renumber_with_a_failing_reference_rewrite_of_a_dependent_blocked_by_restores_the_bundle() {
    let seed = [
        ("/bundle/issues/0001-blocker.md", BLOCKER),
        ("/bundle/issues/0002-blocked.md", BLOCKED),
    ];
    let store = flaky_over(&seed, 1, false);
    assert_restored_ref(&store, "issue/0001", "number", "0005", "disk full");
}

#[test]
fn renumber_rewrites_a_dependent_blocked_by_when_nothing_fails() {
    let seed = [
        ("/bundle/issues/0001-blocker.md", BLOCKER),
        ("/bundle/issues/0002-blocked.md", BLOCKED),
    ];
    let store = flaky_over(&seed, 99, false);
    apply(&store, Path::new("/bundle"), "issue/0001", "number", "0005").expect("renumber");
    let text = store.inner.contents("/bundle/issues/0002-blocked.md");
    assert!(text.contains("blocked_by: [5]"), "{text}");
}

#[test]
fn a_failure_at_the_moved_record_write_changes_nothing() {
    assert_restored(&flaky(0, false), "number", "0005", "disk full");
}

#[test]
fn renumber_with_a_failing_reference_rewrite_and_rollback_names_the_files_it_left_changed() {
    let mut store = flaky(3, false);
    store.stays_failed = true;
    let error =
        apply(&store, Path::new("/bundle"), "adr/0001", "number", "0005").expect_err("writes fail");
    assert!(
        error.starts_with("disk full; rollback incomplete, check: "),
        "{error}"
    );
    assert!(error.contains("/bundle/adr/index.md"), "{error}");
}
