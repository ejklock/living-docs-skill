//! Hexagonal ports for doc persistence and search (ADR 0002). `living-docs-core`
//! depends on neither port's adapter — `fs-store` (slice S0b1) and the future
//! `db-store` (slice 0002) both live outside this crate and implement these
//! traits without this crate knowing they exist.

use std::io;
use std::path::{Path, PathBuf};

/// Reads and writes doc records against a backing store rooted at a given
/// directory. `fs-store` implements this over `.md` files on disk; a future
/// `db-store` adapter implements it over SQLite (slice 0002).
pub trait DocStore {
    /// Enumerates every doc record under `root`, in a stable, deterministic
    /// order.
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>>;

    /// Reads the full contents of the doc record at `path`.
    fn read(&self, path: &Path) -> io::Result<String>;

    /// Writes `contents` to the doc record at `path`, creating any missing
    /// parent directories.
    fn write(&self, path: &Path, contents: &str) -> io::Result<()>;
}

/// Full-text search over doc records. No adapter exists yet — the FTS5-backed
/// `db-store` adapter lands in slice 0002. This crate declares the contract
/// only; it depends on no implementation of it (ADR 0002).
pub trait SearchIndex {
    /// Returns the paths of doc records matching `query`, ranked by
    /// relevance.
    fn search(&self, query: &str) -> io::Result<Vec<PathBuf>>;
}
