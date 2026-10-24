use std::fs;
use std::path::Path;
use std::process::Command;

/// A `git` invocation rooted at `dir`.
pub(crate) fn git_in(dir: &Path) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(dir);
    command
}

/// Whether the git repository holding `dir` runs the doc-gate at commit:
/// its pre-commit file (under `core.hooksPath` when set, else `.git/hooks`)
/// mentions `living-docs` and `check`. `None` outside a git repository.
pub(crate) fn doc_gate_armed(dir: &Path) -> Option<bool> {
    let out = git_in(dir)
        .args(["rev-parse", "--git-path", "hooks"])
        .output()
        .ok()
        .filter(|out| out.status.success())?;
    let hooks = String::from_utf8_lossy(&out.stdout);
    // git prints the path relative to `dir` unless it is already absolute.
    let hook = fs::read_to_string(dir.join(hooks.trim()).join("pre-commit")).unwrap_or_default();
    Some(hook.contains("living-docs") && hook.contains("check"))
}
