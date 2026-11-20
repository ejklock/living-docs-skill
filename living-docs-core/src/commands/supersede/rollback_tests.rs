use super::supersede;
use crate::store::DocStore;
use crate::test_support::WritableMapStore as MapStore;
use std::cell::Cell;
use std::io;
use std::path::{Path, PathBuf};

const OLD_PATH: &str = "/bundle/adr/0001-old-decision.md";
const NEW_PATH: &str = "/bundle/adr/0002-new-decision.md";
const OLD: &str = "---\ntype: ADR\ntitle: Old Decision\ndescription: The old one.\nstatus: Accepted\n---\n\n# 0001. Old Decision\n\nBody.\n";
const NEW: &str = "---\ntype: ADR\ntitle: New Decision\ndescription: The new one.\nstatus: Accepted\n---\n\n# 0002. New Decision\n\nBody.\n";
const INDEX: &str =
    "# ADR index\n\n- [0001](0001-old-decision.md)\n- [0002](0002-new-decision.md)\n";

type Refuse = Box<dyn Fn(&Path, &str) -> bool>;

/// Refuses the writes `refuse` picks, and counts nothing else: every other
/// write reaches the inner store.
struct FailingStore {
    inner: MapStore,
    refuse: Refuse,
}

impl DocStore for FailingStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        self.inner.list(root)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.inner.read(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        if (self.refuse)(path, contents) {
            return Err(io::Error::other("write refused"));
        }
        self.inner.write(path, contents)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, to)
    }
}

/// Fails every read of the retired record once a write to it was refused.
struct UnreadableAfterRefusal {
    inner: FailingStore,
    broken: Cell<bool>,
}

impl DocStore for UnreadableAfterRefusal {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        self.inner.list(root)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        if self.broken.get() && path == Path::new(OLD_PATH) {
            return Err(io::Error::other("read refused"));
        }
        self.inner.read(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        let written = self.inner.write(path, contents);
        if written.is_err() && path == Path::new(OLD_PATH) {
            self.broken.set(true);
        }
        written
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, to)
    }
}

fn store(refuse: impl Fn(&Path, &str) -> bool + 'static) -> FailingStore {
    FailingStore {
        inner: MapStore::seeded(&[
            (OLD_PATH, OLD),
            (NEW_PATH, NEW),
            ("/bundle/adr/index.md", INDEX),
        ]),
        refuse: Box::new(refuse),
    }
}

fn run(store: &FailingStore) -> Result<super::Superseded, String> {
    supersede(store, Path::new("/bundle"), "adr/0001", "adr/0002")
}

fn assert_untouched(store: &FailingStore) {
    assert_eq!(store.inner.contents(OLD_PATH), OLD);
    assert_eq!(store.inner.contents(NEW_PATH), NEW);
    assert_eq!(store.inner.contents("/bundle/adr/index.md"), INDEX);
}

#[test]
fn a_failing_replacement_write_restores_the_retired_record() {
    let store = store(|path, _| path == Path::new(NEW_PATH));
    let error = run(&store).expect_err("the replacement write fails");
    assert!(error.contains("write refused"), "{error}");
    assert!(error.contains(NEW_PATH), "{error}");
    assert!(!error.contains("rollback incomplete"), "{error}");
    assert_untouched(&store);
}

#[test]
fn a_failing_callout_write_restores_the_retired_record() {
    let store = store(|_, text| text.contains("SUPERSEDED — do not act"));
    let error = run(&store).expect_err("the callout write fails");
    assert!(error.contains("write refused"), "{error}");
    assert_untouched(&store);
}

#[test]
fn a_failing_first_write_changes_nothing() {
    let store = store(|path, _| path == Path::new(OLD_PATH));
    let error = run(&store).expect_err("the first write fails");
    assert!(error.contains(OLD_PATH), "{error}");
    assert!(!error.contains("rollback incomplete"), "{error}");
    assert_untouched(&store);
}

#[test]
fn a_failing_undo_names_the_record_it_could_not_restore() {
    let store = store(|path, text| path == Path::new(NEW_PATH) || text == OLD);
    let error = run(&store).expect_err("the replacement write fails");
    assert!(error.contains("rollback incomplete, check:"), "{error}");
    assert!(error.contains(OLD_PATH), "{error}");
    let old = store.inner.contents(OLD_PATH);
    assert!(old.contains("status: Superseded"), "{old}");
}

#[test]
fn a_supersede_whose_writes_all_succeed_links_both_records() {
    let store = store(|_, _| false);
    run(&store).expect("supersede");
    let old = store.inner.contents(OLD_PATH);
    assert!(old.contains("status: Superseded"), "{old}");
    assert!(old.contains("superseded_by: 0002"), "{old}");
    assert!(old.contains("SUPERSEDED — do not act"), "{old}");
    let new = store.inner.contents(NEW_PATH);
    assert!(new.contains("supersedes: 0001"), "{new}");
}

#[test]
fn an_unreadable_record_after_a_refused_restore_is_named_as_not_restored() {
    let store = UnreadableAfterRefusal {
        inner: store(|path, text| path == Path::new(NEW_PATH) || text == OLD),
        broken: Cell::new(false),
    };
    let error = supersede(&store, Path::new("/bundle"), "adr/0001", "adr/0002")
        .expect_err("the replacement write fails");
    assert!(
        error.contains(&format!("rollback incomplete, check: {OLD_PATH}")),
        "{error}"
    );
}
