//! Line endings at the store edge: a record whose every line break is CRLF is
//! read as its LF form and written back with CRLF on every line, so no parser
//! below this layer ever sees a carriage return. A record mixing CRLF and bare
//! LF is refused by [`refuse_mixed`] when a verb drafts it, passes through
//! byte-for-byte when a verb merely scans it, and is judged on its LF form by
//! `check`.

use crate::store::DocStore;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ending {
    Lf,
    Crlf,
    Mixed,
}

/// The line ending `text` uses: CRLF when every line break is CRLF, mixed
/// when CRLF and a bare LF both occur, LF otherwise.
pub(crate) fn ending_of(text: &str) -> Ending {
    let crlf = text.matches("\r\n").count();
    match (crlf, text.matches('\n').count()) {
        (0, _) => Ending::Lf,
        (crlf, lf) if crlf == lf => Ending::Crlf,
        _ => Ending::Mixed,
    }
}

/// What a verb's refusal and `check`'s violation say about a record whose line
/// breaks are both CRLF and bare LF.
pub(crate) const MIXED_MESSAGE: &str =
    "the record has mixed line endings (CRLF and bare LF); make them all LF or all CRLF first";

/// Refuses `path` when it has mixed line endings, so a verb stops before its
/// first write rather than rewriting a record it cannot restore faithfully.
/// A record that cannot be read is left to the caller's own read to report.
pub(crate) fn refuse_mixed(store: &dyn DocStore, path: &Path) -> Result<(), String> {
    match store.read(path) {
        Ok(text) if ending_of(&text) == Ending::Mixed => {
            Err(format!("{}: {MIXED_MESSAGE}", path.display()))
        }
        _ => Ok(()),
    }
}

/// A [`DocStore`] view that hands out every CRLF record in its LF form and
/// writes a record back in the line ending it was read with (LF for a record
/// it never read). A record with mixed endings is handed out unchanged, or in
/// its LF form with `mixed_as_lf`, and is never converted on write.
pub(crate) struct LineEndingStore<'a> {
    inner: &'a dyn DocStore,
    mixed_as_lf: bool,
    seen: RefCell<HashMap<PathBuf, Ending>>,
}

impl<'a> LineEndingStore<'a> {
    pub(crate) fn new(inner: &'a dyn DocStore, mixed_as_lf: bool) -> Self {
        Self {
            inner,
            mixed_as_lf,
            seen: RefCell::new(HashMap::new()),
        }
    }
}

impl DocStore for LineEndingStore<'_> {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        self.inner.list(root)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        let text = self.inner.read(path)?;
        let ending = ending_of(&text);
        self.seen.borrow_mut().insert(path.to_path_buf(), ending);
        let as_lf = ending == Ending::Crlf || (ending == Ending::Mixed && self.mixed_as_lf);
        Ok(if as_lf {
            text.replace("\r\n", "\n")
        } else {
            text
        })
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        let crlf = self.seen.borrow().get(path) == Some(&Ending::Crlf);
        if crlf {
            self.inner.write(path, &contents.replace('\n', "\r\n"))
        } else {
            self.inner.write(path, contents)
        }
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::WritableMapStore as MapStore;

    #[test]
    fn ending_of_tells_lf_crlf_and_mixed_apart() {
        assert_eq!(ending_of("a\nb\n"), Ending::Lf);
        assert_eq!(ending_of("no break"), Ending::Lf);
        assert_eq!(ending_of("a\r\nb\r\n"), Ending::Crlf);
        assert_eq!(ending_of("a\r\nb\n"), Ending::Mixed);
    }

    #[test]
    fn a_crlf_record_reads_as_lf_and_writes_back_as_crlf() {
        let store = MapStore::seeded(&[("/b/a.md", "x\r\ny\r\n")]);
        let view = LineEndingStore::new(&store, false);
        let text = view.read(Path::new("/b/a.md")).unwrap();
        assert_eq!(text, "x\ny\n");
        view.write(Path::new("/b/a.md"), "x\nz\n").unwrap();
        assert_eq!(store.contents("/b/a.md"), "x\r\nz\r\n");
    }

    #[test]
    fn an_unread_path_is_written_as_given() {
        let store = MapStore::seeded(&[]);
        let view = LineEndingStore::new(&store, false);
        view.write(Path::new("/b/new.md"), "x\ny\n").unwrap();
        assert_eq!(store.contents("/b/new.md"), "x\ny\n");
    }

    #[test]
    fn a_mixed_record_passes_through_unchanged_unless_read_as_lf() {
        let store = MapStore::seeded(&[("/b/a.md", "x\r\ny\n")]);
        let raw = LineEndingStore::new(&store, false);
        assert_eq!(raw.read(Path::new("/b/a.md")).unwrap(), "x\r\ny\n");
        raw.write(Path::new("/b/a.md"), "x\r\nz\n").unwrap();
        assert_eq!(store.contents("/b/a.md"), "x\r\nz\n");
        let lf = LineEndingStore::new(&store, true);
        assert_eq!(lf.read(Path::new("/b/a.md")).unwrap(), "x\nz\n");
    }

    #[test]
    fn refuse_mixed_names_only_a_mixed_record() {
        let store = MapStore::seeded(&[("/b/m.md", "x\r\ny\n"), ("/b/c.md", "x\r\ny\r\n")]);
        let err = refuse_mixed(&store, Path::new("/b/m.md")).expect_err("mixed");
        assert!(err.contains("mixed line endings") && err.contains("m.md"));
        assert!(refuse_mixed(&store, Path::new("/b/c.md")).is_ok());
        assert!(refuse_mixed(&store, Path::new("/b/missing.md")).is_ok());
    }
}
