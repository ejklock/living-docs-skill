use crate::commands::set::apply;
use crate::store::DocStore;
use crate::test_support::WritableMapStore as MapStore;
use std::io;
use std::path::{Path, PathBuf};

const ADR_PATH: &str = "/bundle/adr/0001-a-decision.md";
const ADR: &str = "---\ntype: ADR\ntitle: A Decision\ndescription: A decision.\nstatus: Proposed\n---\n\n# 0001. A Decision\n\nBody.\n";
const INDEX: &str = "# ADR index\n\n- [0001](0001-a-decision.md)\n";
const REFERRER: &str = "See [the decision](../adr/0001-a-decision.md).\n";

struct UnreadableStore {
    inner: MapStore,
    unreadable: &'static str,
}

impl DocStore for UnreadableStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        self.inner.list(root)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        if path == Path::new(self.unreadable) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "unreadable"));
        }
        self.inner.read(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        self.inner.write(path, contents)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, to)
    }
}

fn bundle(referrers: &[&str], unreadable: &'static str) -> UnreadableStore {
    let mut seed = vec![(ADR_PATH, ADR), ("/bundle/adr/index.md", INDEX)];
    seed.extend(referrers.iter().map(|path| (*path, REFERRER)));
    UnreadableStore {
        inner: MapStore::seeded(&seed),
        unreadable,
    }
}

fn snapshot(store: &UnreadableStore) -> Vec<(PathBuf, String)> {
    let listed = store.list(Path::new("/")).unwrap_or_default();
    let mut files: Vec<_> = listed
        .into_iter()
        .map(|path| {
            let text = store.inner.contents(&path.to_string_lossy());
            (path, text)
        })
        .collect();
    files.sort();
    files
}

fn assert_refused_untouched(store: &UnreadableStore, key: &str, value: &str) {
    let before = snapshot(store);
    let error = apply(store, Path::new("/bundle"), "adr/0001", key, value)
        .expect_err("an unreadable referrer fails the rewrite");
    assert!(error.contains("unreadable"), "{error}");
    assert_eq!(snapshot(store), before, "bundle changed");
}

const NOTES: [&str; 3] = [
    "/bundle/notes/a.md",
    "/bundle/notes/b.md",
    "/bundle/notes/c.md",
];

#[test]
fn renumber_with_a_failing_reference_rewrite_on_the_first_referrer_changes_nothing() {
    let store = bundle(&NOTES, NOTES[0]);
    assert_refused_untouched(&store, "number", "0005");
}

#[test]
fn renumber_with_a_failing_reference_rewrite_on_the_last_referrer_changes_nothing() {
    let store = bundle(&NOTES, NOTES[2]);
    assert_refused_untouched(&store, "number", "0005");
    assert!(store
        .read(Path::new("/bundle/adr/0005-a-decision.md"))
        .is_err());
}

#[test]
fn renumber_with_a_failing_reference_rewrite_keeps_the_original_file() {
    let store = bundle(&NOTES, NOTES[2]);
    assert_refused_untouched(&store, "number", "next");
    assert_eq!(store.inner.contents(ADR_PATH), ADR);
}

#[test]
fn renumber_without_referrers_still_renumbers() {
    let store = bundle(&[], "/bundle/none.md");
    let applied =
        apply(&store, Path::new("/bundle"), "adr/0001", "number", "0005").expect("renumber");
    assert_eq!(
        applied.path,
        PathBuf::from("/bundle/adr/0005-a-decision.md")
    );
    assert!(store.read(Path::new(ADR_PATH)).is_err());
}

#[test]
fn retitle_with_a_failing_reference_rewrite_on_the_last_referrer_changes_nothing() {
    let store = bundle(&NOTES, NOTES[2]);
    assert_refused_untouched(&store, "title", "Cache is write-through");
    assert!(store
        .read(Path::new("/bundle/adr/0001-cache-is-write-through.md"))
        .is_err());
}

#[test]
fn retitle_with_a_failing_reference_rewrite_on_the_first_referrer_changes_nothing() {
    let store = bundle(&NOTES, NOTES[0]);
    assert_refused_untouched(&store, "title", "Cache is write-through");
}

#[test]
fn retitle_without_a_failure_rewrites_every_reference() {
    let store = bundle(&NOTES, "/bundle/none.md");
    let applied = apply(
        &store,
        Path::new("/bundle"),
        "adr/0001",
        "title",
        "Cache is write-through",
    )
    .expect("retitle");
    assert_eq!(applied.rewritten.len(), 4, "{:?}", applied.rewritten);
    let text = store.inner.contents("/bundle/notes/b.md");
    assert!(text.contains("0001-cache-is-write-through.md"), "{text}");
}
