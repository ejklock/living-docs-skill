use crate::commands::set::apply;
use crate::store::DocStore;
use crate::test_support::WritableMapStore as MapStore;
use std::io;
use std::path::{Path, PathBuf};

const ADR_PATH: &str = "/bundle/adr/0001-a-decision.md";
const STALE: &str = "---\ntype: ADR\ntitle: A Decision\ndescription: A decision.\nstatus: Proposed\n---\n\n> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.\n\n# 0001. A Decision\n\nBody.\n";
const CLEAN: &str = "---\ntype: ADR\ntitle: A Decision\ndescription: A decision.\nstatus: Proposed\n---\n\n# 0001. A Decision\n\nBody.\n";
const INDEX: &str = "# ADR index\n\n- [0001](0001-a-decision.md)\n";

/// Refuses every write of a record text that has no callout: the text a
/// reconcile of a stale callout produces.
struct CalloutFailingStore {
    inner: MapStore,
    refuse_calloutless: bool,
}

impl DocStore for CalloutFailingStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        self.inner.list(root)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.inner.read(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        let record = contents.starts_with("---\ntype: ADR");
        if self.refuse_calloutless && record && !contents.contains("DEPRECATED") {
            return Err(io::Error::other("callout write refused"));
        }
        self.inner.write(path, contents)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, to)
    }
}

fn store(record: &str, refuse_calloutless: bool) -> CalloutFailingStore {
    CalloutFailingStore {
        inner: MapStore::seeded(&[(ADR_PATH, record), ("/bundle/adr/index.md", INDEX)]),
        refuse_calloutless,
    }
}

fn snapshot(store: &CalloutFailingStore) -> Vec<(PathBuf, String)> {
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

fn assert_refused_untouched(key: &str, value: &str) {
    let store = store(STALE, true);
    let before = snapshot(&store);
    let error = apply(&store, Path::new("/bundle"), "adr/0001", key, value)
        .expect_err("the callout write fails");
    assert!(error.contains("callout write refused"), "{error}");
    assert_eq!(snapshot(&store), before, "bundle changed");
}

#[test]
fn a_failing_callout_write_rolls_back_the_renumber() {
    assert_refused_untouched("number", "0005");
}

#[test]
fn a_failing_callout_write_rolls_back_the_renaming_retitle() {
    assert_refused_untouched("title", "Cache is write-through");
}

#[test]
fn a_renumber_keeps_a_callout_that_needs_no_write() {
    let store = store(CLEAN, false);
    apply(&store, Path::new("/bundle"), "adr/0001", "number", "0005").expect("renumber");
    let expected = CLEAN.replace("# 0001.", "# 0005.");
    assert_eq!(
        store.inner.contents("/bundle/adr/0005-a-decision.md"),
        expected
    );
}

#[test]
fn a_renumber_of_a_deprecated_record_writes_its_callout() {
    let record = CLEAN.replace("Proposed", "Deprecated");
    let store = store(&record, false);
    apply(&store, Path::new("/bundle"), "adr/0001", "number", "0005").expect("renumber");
    let expected = STALE
        .replace("Proposed", "Deprecated")
        .replace("# 0001.", "# 0005.");
    assert_eq!(
        store.inner.contents("/bundle/adr/0005-a-decision.md"),
        expected
    );
}

#[test]
fn a_retitle_that_keeps_its_filename_reconciles_the_callout() {
    let store = store(STALE, false);
    apply(
        &store,
        Path::new("/bundle"),
        "adr/0001",
        "title",
        "A Decision",
    )
    .expect("retitle");
    assert_eq!(store.inner.contents(ADR_PATH), CLEAN);
}

const UNWRITABLE: &str = "---\ntype: ADR\ntitle: A Decision\ndescription: A decision.\nstatus: Proposed\ntags: [a, [b]]\n---\n\n> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.\n\n# 0001. A Decision\n\nBody.\n";

fn assert_unwritable_refused(key: &str, value: &str) {
    let store = store(UNWRITABLE, false);
    let before = snapshot(&store);
    let error = apply(&store, Path::new("/bundle"), "adr/0001", key, value)
        .expect_err("the record has no canonical form");
    assert!(
        error.contains("'tags' cannot be written canonically"),
        "{error}"
    );
    assert_eq!(snapshot(&store), before, "bundle changed");
}

#[test]
fn a_renumber_of_a_record_with_no_canonical_form_is_refused_untouched() {
    assert_unwritable_refused("number", "0005");
}

#[test]
fn a_renaming_retitle_of_a_record_with_no_canonical_form_is_refused_untouched() {
    assert_unwritable_refused("title", "Cache is write-through");
}
