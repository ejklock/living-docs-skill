//! The undo journal a verb that writes several records shares: each write is
//! journaled with the text it replaces, so a failure restores the records
//! already changed.

use crate::store::DocStore;
use std::path::{Path, PathBuf};

/// A step that reached the store, with what undoes it.
pub(crate) enum Step {
    Wrote(PathBuf, String),
    Renamed(PathBuf, PathBuf),
}

/// Writes `text` to `path`, journaling the text it replaces in `done` before
/// the write runs. A record already holding `text` is left alone.
pub(crate) fn journaled_write(
    store: &dyn DocStore,
    path: &Path,
    text: &str,
    done: &mut Vec<Step>,
) -> Result<(), String> {
    let before = store.read(path).map_err(|e| e.to_string())?;
    if before == text {
        return Ok(());
    }
    done.push(Step::Wrote(path.to_path_buf(), before));
    store.write(path, text).map_err(|e| e.to_string())
}

/// Like [`journaled_write`], with a failure prefixed by the path it concerns.
pub(crate) fn named_write(
    store: &dyn DocStore,
    path: &Path,
    text: &str,
    done: &mut Vec<Step>,
) -> Result<(), String> {
    journaled_write(store, path, text, done).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes `before` back to `path`; a record that already holds `before`
/// counts as restored even when the write is refused, as a write that failed
/// before changing anything leaves it.
fn restored_text(store: &dyn DocStore, path: &Path, before: &str) -> bool {
    store.write(path, before).is_ok() || store.read(path).is_ok_and(|now| now == before)
}

/// Reverses `done` newest first and returns the paths it could not restore,
/// the destination for a rename that would not go back.
pub(crate) fn undo(store: &dyn DocStore, done: Vec<Step>) -> Vec<PathBuf> {
    let mut stuck = Vec::new();
    for step in done.into_iter().rev() {
        let (restored, at) = match step {
            Step::Wrote(written, before) => (restored_text(store, &written, &before), written),
            Step::Renamed(from, to) => (store.rename(&to, &from).is_ok(), to),
        };
        if !restored {
            stuck.push(at);
        }
    }
    stuck
}

/// Undoes `done` and returns `error`, followed by the paths the undo could
/// not restore when it was incomplete.
pub(crate) fn rolled_back(store: &dyn DocStore, error: String, done: Vec<Step>) -> String {
    let stuck = undo(store, done);
    if stuck.is_empty() {
        return error;
    }
    let names: Vec<_> = stuck.iter().map(|p| p.display().to_string()).collect();
    format!("{error}; rollback incomplete, check: {}", names.join(", "))
}
