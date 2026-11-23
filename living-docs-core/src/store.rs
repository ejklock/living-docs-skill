//! Hexagonal ports for doc persistence and search (ADR 0002). `living-docs-core`
//! depends on neither port's adapter — `fs-store` (slice S0b1) and the future
//! `db-store` (slice 0002) both live outside this crate and implement these
//! traits without this crate knowing they exist.

use std::io;
use std::path::{Path, PathBuf};

/// Reads and writes the files of a bundle against a backing store rooted at a
/// given directory: its doc records and its reserved files (`index.md`).
/// `fs-store` implements this over `.md` files on disk; a future `db-store`
/// adapter would implement it over SQLite.
///
/// An adapter may refuse a path outside the root it was built for. The
/// refusal is an `io::Error` of kind `PermissionDenied` naming the path, the
/// root and the rule that a record must lie inside the bundle; a caller treats
/// that error as "this file is not the bundle's", never as a missing or
/// empty file. [`is_refusal`] names that error: an operating-system
/// permission error has the same kind but carries an OS error code, so it is
/// not a refusal.
pub trait DocStore {
    /// Enumerates every doc record under `root`, in a stable, deterministic
    /// order.
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>>;

    /// Reads the full contents of the file at `path`, a doc record or a
    /// reserved file.
    fn read(&self, path: &Path) -> io::Result<String>;

    /// Writes `contents` to the file at `path`, a doc record or a reserved
    /// file, creating any missing parent directories.
    fn write(&self, path: &Path, contents: &str) -> io::Result<()>;

    /// Moves the doc record at `from` to `to` in one operation, creating any
    /// missing parent directories. A retitle renames a record's file, and a
    /// two-step write-then-delete can leave two copies of one record in the
    /// tree, so the port asks the adapter for the move itself (ADR 0061).
    /// Callers refuse a destination collision before calling.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Whether `relative`, a path from the repository root, names an existing
    /// entry. The answer is existence only, never contents. `Some(false)` is
    /// for a path inside the repository that does not exist. `None` means
    /// unknown: no repository root is known, or the store refuses the path
    /// (absolute, a `..` segment, or resolving outside the repository), so
    /// the caller reports nothing; the default knows no root.
    fn repository_entry_exists(&self, relative: &Path) -> Option<bool> {
        let _ = relative;
        None
    }
}

/// True when `error` is a store's own refusal of a path: kind
/// `PermissionDenied` constructed by the adapter, with no OS error code. An
/// operating-system permission error returns false.
pub fn is_refusal(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::PermissionDenied && error.raw_os_error().is_none()
}

/// Full-text search over doc records. No adapter exists yet — the FTS5-backed
/// `db-store` adapter lands in slice 0002. This crate declares the contract
/// only; it depends on no implementation of it (ADR 0002).
pub trait SearchIndex {
    /// Returns the paths of doc records matching `query`, ranked by
    /// relevance.
    fn search(&self, query: &str) -> io::Result<Vec<PathBuf>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A read that fails on invalid UTF-8 is not a store refusal, so callers
    /// keep treating it as an ordinary read error.
    #[test]
    fn a_read_failing_on_invalid_utf8_is_not_a_refusal() {
        let dir = std::env::temp_dir().join(format!("store-utf8-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("bad.md");
        std::fs::write(&file, b"title: bad \xff\n").unwrap();
        let error = std::fs::read_to_string(&file).unwrap_err();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(!is_refusal(&error), "{error:?}");
    }

    /// Only a constructed permission error with no OS code is a refusal.
    #[test]
    fn only_a_permission_denied_without_an_os_code_is_a_refusal() {
        let refusal = io::Error::new(io::ErrorKind::PermissionDenied, "outside");
        let os_denied = io::Error::from_raw_os_error(13);
        assert!(is_refusal(&refusal));
        assert!(!is_refusal(&os_denied));
    }
}
