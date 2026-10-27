#![allow(dead_code)]

use std::ffi::OsStr;
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

/// Runs `check` with `--plain`, so callers asserting on stdout's text shape
/// keep working under ADR 0060's JSON-off-a-TTY default; `check_json`
/// exercises the JSON shape directly where that is the point of the test.
pub fn run_check(bundle: &Path) -> Output {
    living_docs()
        .args(["check", bundle.to_str().unwrap(), "--plain"])
        .output()
        .expect("failed to run living-docs check")
}

/// Runs `check --plain --advisories`, so callers asserting on the wording of
/// an individual advisory see the full list rather than the per-kind summary.
pub fn run_check_advisories(bundle: &Path) -> Output {
    living_docs()
        .args(["check", bundle.to_str().unwrap(), "--plain", "--advisories"])
        .output()
        .expect("failed to run living-docs check")
}

pub fn run_new(docs_dir: &Path, doc_type: &str, title: &str) -> Output {
    living_docs()
        .args([
            "--docs-dir",
            docs_dir.to_str().unwrap(),
            "new",
            doc_type,
            title,
        ])
        .output()
        .expect("failed to run living-docs new")
}

pub fn run_supersede(docs_dir: &Path, old: &str, new: &str) -> Output {
    living_docs()
        .args([
            "--docs-dir",
            docs_dir.to_str().unwrap(),
            "supersede",
            old,
            new,
        ])
        .output()
        .expect("failed to run living-docs supersede")
}

/// Scaffolds two fresh ADR records ("Old Decision" then "New Decision") and
/// supersedes the first with the second, asserting every step succeeded.
/// Returns `supersede`'s own [`Output`] for any further assertion the
/// caller needs on top of the shared preamble.
pub fn new_two_adrs_and_supersede(docs_dir: &Path) -> Output {
    assert!(run_new(docs_dir, "adr", "Old Decision").status.success());
    assert!(run_new(docs_dir, "adr", "New Decision").status.success());
    let output = run_supersede(docs_dir, "0001", "0002");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

pub fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// A scratch directory tree removed when the guard drops, including while a
/// panicking test unwinds. It derefs to the path tests work in, which is the
/// tree's root or a directory inside it; the whole root is what gets removed.
pub struct Scratch {
    root: PathBuf,
    path: PathBuf,
}

impl Scratch {
    /// Creates a unique directory named after `name`; the guard derefs to it.
    pub fn new(name: &str) -> Scratch {
        Scratch::with_subdir(name, "")
    }

    /// Creates a unique root named after `name` holding `subdir`; the guard
    /// derefs to `subdir` while dropping removes the root.
    pub fn with_subdir(name: &str, subdir: &str) -> Scratch {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("{name}-{}-{nanos}-{serial}", std::process::id()));
        let path = if subdir.is_empty() {
            root.clone()
        } else {
            root.join(subdir)
        };
        fs::create_dir_all(&path).unwrap();
        Scratch { root, path }
    }

    /// The directory the guard removes on drop.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for Scratch {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<OsStr> for Scratch {
    fn as_ref(&self) -> &OsStr {
        self.path.as_os_str()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Builds a scratch bundle directory unique to this process run, scoped by
/// `suite` (the calling test file) and `label` (the individual test case).
/// The guard derefs to the `docs` directory and removes its parent.
pub fn temp_bundle(suite: &str, label: &str) -> Scratch {
    Scratch::with_subdir(&format!("living-docs-{suite}-test-{label}"), "docs")
}

/// Regenerates every type index of `bundle` with the CLI, so a fixture's
/// indexes are the ones `index` writes and `check` finds no drift in them.
pub fn index_bundle(bundle: &Path) {
    let output = living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap(), "index", "--plain"])
        .output()
        .expect("failed to run living-docs index");
    assert!(output.status.success(), "index failed");
}

pub fn write(bundle: &Path, rel: &str, contents: &str) {
    let path = bundle.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

/// Fixtures live under `skills/living-docs/tests/fixtures` relative to the repo
/// root; `CARGO_MANIFEST_DIR` anchors this at compile time regardless of the
/// working directory `cargo test` is invoked from.
pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .join("skills/living-docs/tests/fixtures")
        .join(name)
        .join("docs")
}
