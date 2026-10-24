//! Git lookups the CLI performs on behalf of the verbs that seed a record's
//! owner or timestamp. Every lookup treats a missing `git`, a non-zero exit
//! or an empty answer as absence, so a machine without git still works.

use std::path::Path;
use std::process::Command;

/// Reads one git config value.
fn config(key: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--get", key])
        .output()
        .ok()?;
    non_empty(&output)
}

/// The git identity a record's owner is seeded with: `user.name`, else
/// `user.email`.
pub(crate) fn identity() -> Option<String> {
    config("user.name").or_else(|| config("user.email"))
}

/// The UTC author date of the commit that added `path`, in the format a new
/// record is stamped with. `None` when `path` was never committed or lies
/// outside a git repository.
pub(crate) fn added_at(path: &Path) -> Option<String> {
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

fn non_empty(output: &std::process::Output) -> Option<String> {
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}
