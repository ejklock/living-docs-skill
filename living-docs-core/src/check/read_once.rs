//! A read-once view of a backing store for one `check` run: the first read of
//! a path asks the store, every later read of it hands back the remembered
//! result, so a dozen checks over the same record cost one read.

use crate::store::DocStore;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

/// A failed read kept so it can be raised again: the kind, the text every
/// check prints, and the operating-system code that tells a refusal by the
/// store from a refusal by the system.
struct Failure {
    kind: io::ErrorKind,
    text: String,
    os_code: Option<i32>,
}

impl Failure {
    fn of(error: &io::Error) -> Self {
        Self {
            kind: error.kind(),
            text: error.to_string(),
            os_code: error.raw_os_error(),
        }
    }

    fn raise(&self) -> io::Error {
        match self.os_code {
            Some(code) => io::Error::from_raw_os_error(code),
            None => io::Error::new(self.kind, self.text.clone()),
        }
    }
}

/// A [`DocStore`] that remembers each path's read result for its own lifetime.
/// `list` goes straight to the inner store, so it must outlive no write:
/// build it per run, over a store nothing writes meanwhile. A check run never
/// writes, so `write` and `rename` refuse instead of reaching the inner store.
pub(super) struct ReadOnceStore<'a> {
    inner: &'a dyn DocStore,
    seen: RefCell<HashMap<PathBuf, Result<String, Failure>>>,
}

impl<'a> ReadOnceStore<'a> {
    pub(super) fn new(inner: &'a dyn DocStore) -> Self {
        Self {
            inner,
            seen: RefCell::new(HashMap::new()),
        }
    }
}

impl DocStore for ReadOnceStore<'_> {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        self.inner.list(root)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        match self.seen.borrow().get(path) {
            Some(Ok(text)) => return Ok(text.clone()),
            Some(Err(failure)) => return Err(failure.raise()),
            None => {}
        }
        let result = self.inner.read(path);
        let kept = match &result {
            Ok(text) => Ok(text.clone()),
            Err(error) => Err(Failure::of(error)),
        };
        self.seen.borrow_mut().insert(path.to_path_buf(), kept);
        result
    }

    fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
        Err(read_only())
    }

    fn rename(&self, _from: &Path, _to: &Path) -> io::Result<()> {
        Err(read_only())
    }
}

fn read_only() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "a check run is read-only: it never writes or renames",
    )
}

#[cfg(test)]
mod tests {
    use super::super::{check_violations, run_all_checks, Advisory, Reporter};
    use super::*;
    use crate::callout;
    use crate::check::kinds;
    use crate::test_support::{MapStore, WritableMapStore};
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::ErrorKind::PermissionDenied;

    /// Counts reads per path and refuses one path the way a confined adapter
    /// refuses a record outside its bundle.
    struct CountingStore {
        inner: MapStore,
        refused: PathBuf,
        reads: RefCell<BTreeMap<PathBuf, usize>>,
    }

    impl DocStore for CountingStore {
        fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
            self.inner.list(root)
        }

        fn read(&self, path: &Path) -> io::Result<String> {
            *self
                .reads
                .borrow_mut()
                .entry(path.to_path_buf())
                .or_default() += 1;
            match path == self.refused {
                true => Err(io::Error::new(PermissionDenied, "outside the bundle")),
                false => self.inner.read(path),
            }
        }

        fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
            Err(io::ErrorKind::Unsupported.into())
        }

        fn rename(&self, _from: &Path, _to: &Path) -> io::Result<()> {
            Err(io::ErrorKind::Unsupported.into())
        }
    }

    /// A bundle directory on disk holding the `index.md` files whose
    /// presence the checks test with the filesystem; the contents live in the
    /// store.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
            fs::create_dir_all(dir.join("adr")).unwrap();
            fs::write(dir.join("index.md"), ROOT_INDEX).unwrap();
            fs::write(dir.join("adr/index.md"), "").unwrap();
            Self(dir)
        }

        fn at(&self, name: &str) -> String {
            self.0.join(name).display().to_string()
        }

        fn store(&self, files: &[(&str, &str)]) -> CountingStore {
            let seed: Vec<(String, &str)> = files.iter().map(|(n, c)| (self.at(n), *c)).collect();
            let seed: Vec<(&str, &str)> = seed.iter().map(|(n, c)| (n.as_str(), *c)).collect();
            CountingStore {
                inner: MapStore::seeded(&seed).0,
                refused: PathBuf::from(self.at("adr/0005-refused.md")),
                reads: RefCell::default(),
            }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const ROOT_INDEX: &str = "---\nokf_version: \"0.1\"\n---\n\n# Docs\n\n* [ADRs](adr/)\n";
    const ADR_INDEX: &str = "# ADRs\n";
    const BODY: &str = "---\ntype: ADR\ntitle: T\n---\n# T\n";

    fn retired(title: &str, callout_target: &str) -> String {
        let line = callout::expected(Some("Superseded"), Some(callout_target)).unwrap();
        format!("---\ntype: ADR\ntitle: {title}\nstatus: Superseded\nsuperseded_by: 0004\n---\n{line}\n\n# {title}\n")
    }

    fn bundle_with_every_read_case(scratch: &Scratch) -> CountingStore {
        let linking = format!("{BODY}\n[old](/adr/0003-old.md)\n");
        let mixed = format!("{BODY}x\r\n");
        let (old, wrong) = (retired("Old", "0004-new.md"), retired("Wrong", "0009-x.md"));
        scratch.store(&[
            ("index.md", ROOT_INDEX),
            ("adr/index.md", ADR_INDEX),
            ("adr/0001-crlf.md", &BODY.replace('\n', "\r\n")),
            ("adr/0002-mixed.md", &mixed),
            ("adr/0003-old.md", &old),
            ("adr/0004-new.md", BODY),
            ("adr/0005-refused.md", BODY),
            ("adr/0006-link.md", &linking),
            ("adr/0007-wrong.md", &wrong),
        ])
    }

    /// A run reads each path from the store at most once, the index files
    /// included, and judges what it reads as before: a CRLF record in LF form,
    /// a mixed record flagged once, a refused record reported once, a retired
    /// record's callout checked against its successor.
    #[test]
    fn a_check_run_reads_each_path_once_and_judges_it_unchanged() {
        let scratch = Scratch::new("read-once-judges");
        let store = bundle_with_every_read_case(&scratch);

        let violations = check_violations(&store, &scratch.0);

        assert_eq!(store.reads.borrow().iter().find(|(_, n)| **n > 1), None);
        assert!(store
            .reads
            .borrow()
            .contains_key(&scratch.0.join("adr/index.md")));
        let about = |name: &str, message: &str| {
            let hit =
                |(file, text): &&(String, String)| file.ends_with(name) && text.contains(message);
            violations.iter().filter(hit).count()
        };
        assert_eq!(about("0005-refused.md", "outside the bundle"), 1);
        assert_eq!(about("0002-mixed.md", "mixed line endings"), 1);
        assert_eq!(about("0001-crlf.md", "line endings"), 0);
        assert_eq!(about("0003-old.md", "has no matching record"), 0);
        assert_eq!(about("0003-old.md", "callout"), 0);
        assert_eq!(about("0007-wrong.md", "callout"), 1);
    }

    /// A record linking a superseded record is advised once about the move.
    #[test]
    fn a_record_linking_a_superseded_record_is_advised_once() {
        let scratch = Scratch::new("read-once-advice");
        let store = bundle_with_every_read_case(&scratch);
        let mut reporter = Reporter::new();

        run_all_checks(&store, &scratch.0, &mut reporter, false, true);

        let moved = |a: &&Advisory| {
            a.kind == kinds::MOVED_SOURCE
                && a.file.ends_with("0006-link.md")
                && a.message.contains("0003-old.md")
        };
        assert_eq!(reporter.advisories.iter().filter(moved).count(), 1);
    }

    #[test]
    fn a_bundle_with_no_records_reports_no_violation() {
        let scratch = Scratch::new("read-once-empty");
        let store = scratch.store(&[("index.md", ROOT_INDEX), ("adr/index.md", ADR_INDEX)]);
        let (index, regenerated) =
            crate::commands::index::compute(&store, &scratch.0, "adr").unwrap();
        fs::write(index, regenerated).unwrap();

        assert_eq!(check_violations(&store, &scratch.0), vec![]);
    }

    /// A write or a rename is refused with the read-only reason and leaves
    /// the backing store as it was.
    #[test]
    fn a_write_or_rename_through_the_view_is_refused_and_changes_nothing() {
        let backing = WritableMapStore::seeded(&[("/b/a.md", "kept")]);
        let view = ReadOnceStore::new(&backing);

        let errors = [
            view.write(Path::new("/b/a.md"), "changed").unwrap_err(),
            view.write(Path::new("/b/new.md"), "created").unwrap_err(),
            view.rename(Path::new("/b/a.md"), Path::new("/b/z.md"))
                .unwrap_err(),
        ];

        assert!(errors
            .iter()
            .all(|e| e.to_string().contains("check run is read-only")));
        assert_eq!(backing.contents("/b/a.md"), "kept");
        assert_eq!(backing.contents("/b/new.md"), "");
        assert_eq!(backing.contents("/b/z.md"), "");
    }

    /// A remembered failure is raised again as the same kind and text, and an
    /// operating-system failure stays one, so a refusal by the store is never
    /// confused with a refusal by the system.
    #[test]
    fn a_remembered_failure_is_raised_again_with_its_kind_text_and_os_code() {
        let scratch = Scratch::new("read-once-failure");
        let store = bundle_with_every_read_case(&scratch);
        let view = ReadOnceStore::new(&store);
        let path = scratch.0.join("adr/0005-refused.md");
        let first = view.read(&path).unwrap_err();
        let again = view.read(&path).unwrap_err();

        assert_eq!(
            (again.kind(), again.to_string()),
            (first.kind(), first.to_string())
        );
        assert_eq!(again.raw_os_error(), None);
        assert_eq!(store.reads.borrow()[&path], 1);
        let os = Failure::of(&io::Error::from_raw_os_error(13)).raise();
        assert_eq!(
            (os.raw_os_error(), os.kind()),
            (Some(13), io::ErrorKind::PermissionDenied)
        );
    }
}
