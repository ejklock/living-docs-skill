//! The one rename path a retitle and a renumbering share: every rewrite is
//! drafted, then applied with an undo journal.

use super::references::plan_references;
use crate::callout::draft_reconciled;
use crate::commands::supersede::draft_frontmatter_fields;
use crate::store::DocStore;
use std::io;
use std::path::{Path, PathBuf};

/// Moves a record's file to `target` and repoints every bundle record that
/// names the old filename, the moved record's own body included; the one
/// rename path a retitle and a renumbering share. `own` is the moved record's
/// body as the caller edited it, and `blocked` the `blocked_by` value to
/// write into each listed record, drafted with the references so they commit
/// together. Every rewrite is drafted before any file changes, so a record
/// that cannot be read leaves the bundle untouched; a write or the rename
/// that fails afterwards is undone, file by file, and the error is returned,
/// followed by the paths the undo could not restore when it was incomplete.
/// The moved record's callout is reconciled with its status in the same
/// draft, so it is written with the record and undone with it. Returns the
/// records it rewrote.
pub(in crate::commands::set) fn relocate(
    store: &dyn DocStore,
    docs_dir: &Path,
    (path, target): (&Path, &Path),
    (own, blocked): (&str, &[(PathBuf, String)]),
) -> Result<Vec<PathBuf>, String> {
    let plan = plan_references(store, docs_dir, (path, target), own)?;
    let plan = with_blocked_by(store, plan, blocked)?;
    let moved = plan
        .iter()
        .find(|(planned, _)| planned == target)
        .map_or(own, |(_, text)| text.as_str());
    let reconciled = draft_reconciled(store, target, moved)?;
    let moved = reconciled.as_deref().unwrap_or(moved);
    let mut done = Vec::new();
    if let Err(error) = apply_plan(store, (path, target), moved, &plan, &mut done) {
        let stuck = undo(store, (path, target), done);
        return Err(match stuck.is_empty() {
            true => error,
            false => format!("{error}; rollback incomplete, check: {}", join(&stuck)),
        });
    }
    Ok(plan.into_iter().map(|(planned, _)| planned).collect())
}

fn join(paths: &[PathBuf]) -> String {
    let names: Vec<_> = paths.iter().map(|p| p.display().to_string()).collect();
    names.join(", ")
}

/// A store that reads the drafted text of the records in `drafted` and the
/// inner store's text of every other record; it never writes.
struct Drafted<'a> {
    inner: &'a dyn DocStore,
    drafted: &'a [(PathBuf, String)],
}

impl DocStore for Drafted<'_> {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        self.inner.list(root)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        match self.drafted.iter().find(|(drafted, _)| drafted == path) {
            Some((_, text)) => Ok(text.clone()),
            None => self.inner.read(path),
        }
    }

    fn write(&self, _: &Path, _: &str) -> io::Result<()> {
        Err(io::Error::other("a draft store is read-only"))
    }

    fn rename(&self, _: &Path, _: &Path) -> io::Result<()> {
        Err(io::Error::other("a draft store is read-only"))
    }
}

/// `plan` with each `blocked` record's `blocked_by` drafted on top of its
/// planned text, so a record that also names the old filename keeps both
/// rewrites.
fn with_blocked_by(
    store: &dyn DocStore,
    mut plan: Vec<(PathBuf, String)>,
    blocked: &[(PathBuf, String)],
) -> Result<Vec<(PathBuf, String)>, String> {
    for (other, rendered) in blocked {
        let field = [("blocked_by", rendered.clone())];
        let view = Drafted {
            inner: store,
            drafted: &plan,
        };
        let text = draft_frontmatter_fields(&view, other, &field, &[])?;
        match plan.iter_mut().find(|(planned, _)| planned == other) {
            Some(entry) => entry.1 = text,
            None => plan.push((other.clone(), text)),
        }
    }
    Ok(plan)
}

/// A step of [`apply_plan`] that reached the store, with what undoes it.
enum Step {
    Wrote(PathBuf, String),
    Renamed,
}

/// Writes the moved record at its old path, renames it, then writes every
/// other planned record, journaling each step in `done` before it runs.
fn apply_plan(
    store: &dyn DocStore,
    (path, target): (&Path, &Path),
    moved: &str,
    plan: &[(PathBuf, String)],
    done: &mut Vec<Step>,
) -> Result<(), String> {
    journaled_write(store, path, moved, done)?;
    store.rename(path, target).map_err(|e| e.to_string())?;
    done.push(Step::Renamed);
    for (planned, text) in plan.iter().filter(|(planned, _)| planned != target) {
        journaled_write(store, planned, text, done)?;
    }
    Ok(())
}

fn journaled_write(
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

/// Reverses `done` newest first and returns the paths it could not restore,
/// the target for a rename that would not go back.
fn undo(store: &dyn DocStore, (path, target): (&Path, &Path), done: Vec<Step>) -> Vec<PathBuf> {
    let mut stuck = Vec::new();
    for step in done.into_iter().rev() {
        let (restored, at) = match step {
            Step::Wrote(written, before) => (store.write(&written, &before), written),
            Step::Renamed => (store.rename(target, path), target.to_path_buf()),
        };
        if restored.is_err() {
            stuck.push(at);
        }
    }
    stuck
}
