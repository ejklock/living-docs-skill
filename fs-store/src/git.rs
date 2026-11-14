//! Git lookups that seed a record's owner or timestamp, shared by every front
//! that writes records. Every lookup treats a missing `git`, a non-zero exit
//! or an empty answer as absence, so a machine without git still works.

use living_docs_core::doc_type::spec_for;
use std::path::Path;
use std::process::{Command, Output};

/// Reads one git config value as seen from `dir`, or from the nearest
/// existing directory above it.
fn config(dir: &Path, key: &str) -> Option<String> {
    let dir = dir
        .ancestors()
        .find(|ancestor| ancestor.is_dir())
        .unwrap_or_else(|| Path::new("."));
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["config", "--get", key])
        .output()
        .ok()?;
    non_empty(&output)
}

/// The git identity a record's owner is seeded with, as configured for the
/// repository holding `dir`: `user.name`, else `user.email`.
pub fn identity(dir: &Path) -> Option<String> {
    config(dir, "user.name").or_else(|| config(dir, "user.email"))
}

/// The owner for a new record of `doc_type` in the bundle at `dir`: the git
/// identity when the type's registry row requires an owner and the caller
/// gave none, else `None`.
pub fn seeded_owner(dir: &Path, doc_type: &str, given: Option<&str>) -> Option<String> {
    if given.is_some() || !spec_for(doc_type).is_some_and(|spec| spec.requires_owner) {
        return None;
    }
    identity(dir)
}

/// The UTC author date of the commit that added `path`, in the format a new
/// record is stamped with. `None` when `path` was never committed or lies
/// outside a git repository.
pub fn added_at(path: &Path) -> Option<String> {
    let dir = path.parent().filter(|dir| !dir.as_os_str().is_empty());
    let name = path.file_name()?;
    let output = Command::new("git")
        .arg("-C")
        .arg(dir.unwrap_or_else(|| Path::new(".")))
        .args([
            "log",
            "--diff-filter=A",
            "--follow",
            "--date=format-local:%Y-%m-%dT%H:%M:%SZ",
            "--format=%ad",
            "--",
        ])
        .arg(name)
        .env("TZ", "UTC")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .map(str::to_owned)
}

fn non_empty(output: &Output) -> Option<String> {
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}
