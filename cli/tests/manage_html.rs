//! `manage html` writes the bundle as one local HTML file, taking mermaid.js
//! from `--mermaid`, else the installed hash-verified copy, else the pinned
//! CDN tag, and names the source it used on stderr.

mod common;

use common::{living_docs, run_new, Scratch};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

const PINNED_URL: &str = "https://cdn.jsdelivr.net/npm/mermaid@12.3.0/dist/mermaid.min.js";

struct Project {
    root: Scratch,
}

impl Project {
    fn new() -> Self {
        let root = Scratch::with_subdir("manage-html", "project");
        let docs = root.join("docs");
        fs::create_dir_all(&docs).unwrap();
        assert!(run_new(&docs, "adr", "Cache Strategy Decision")
            .status
            .success());
        assert!(run_new(&docs, "issue", "Flaky Upload Fix").status.success());
        Self { root }
    }

    fn dir(&self) -> PathBuf {
        self.root.to_path_buf()
    }

    fn data_home(&self) -> PathBuf {
        self.root.root().join("data")
    }

    fn installed_file(&self) -> PathBuf {
        self.data_home()
            .join("living-docs/mermaid/12.3.0/mermaid.min.js")
    }

    fn html(&self, args: &[&str]) -> Output {
        living_docs()
            .current_dir(self.dir())
            .env("HOME", self.root.root().join("home"))
            .env("XDG_DATA_HOME", self.data_home())
            .args(["--docs-dir", "docs", "manage", "html"])
            .args(args)
            .output()
            .expect("failed to run living-docs manage html")
    }

    fn default_output(&self) -> PathBuf {
        self.dir().join(".living-docs/html/index.html")
    }
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(snapshot(&path));
        } else {
            found.push((path.clone(), fs::read(&path).unwrap()));
        }
    }
    found.sort();
    found
}

#[test]
fn writes_every_record_title_to_the_default_file() {
    let project = Project::new();

    let output = project.html(&[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let page = fs::read_to_string(project.default_output()).unwrap();
    assert!(page.contains("Cache Strategy Decision"), "{page}");
    assert!(page.contains("Flaky Upload Fix"), "{page}");
}

#[test]
fn running_twice_yields_byte_identical_files() {
    let project = Project::new();

    project.html(&[]);
    let first = fs::read(project.default_output()).unwrap();
    project.html(&[]);
    let second = fs::read(project.default_output()).unwrap();

    assert_eq!(first, second);
}

#[test]
fn the_page_never_contains_the_project_path() {
    let project = Project::new();

    project.html(&[]);

    let page = fs::read_to_string(project.default_output()).unwrap();
    let absolute = project.dir().to_string_lossy().into_owned();
    assert!(!page.contains(&absolute), "page names {absolute}");
}

#[test]
fn out_writes_to_the_given_path_creating_parents() {
    let project = Project::new();

    let output = project.html(&["--out", "deep/er/page.html"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert!(project.dir().join("deep/er/page.html").is_file());
    assert!(!project.default_output().exists());
}

#[test]
fn manage_help_lists_html_and_the_root_verbs_are_unchanged() {
    let manage = living_docs()
        .args(["manage", "--help", "--plain"])
        .output()
        .unwrap();
    assert!(
        stdout_of(&manage).contains("html"),
        "{}",
        stdout_of(&manage)
    );

    let root = living_docs().args(["--help"]).output().unwrap();
    let doc: Value = serde_json::from_str(&stdout_of(&root)).unwrap();
    let verbs: Vec<&str> = doc["commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["name"].as_str())
        .collect();
    assert_eq!(verbs, ["write", "read", "manage", "check", "mcp"]);
}

#[test]
fn without_an_installed_copy_the_page_carries_the_pinned_tag() {
    let project = Project::new();

    let output = project.html(&[]);

    let page = fs::read_to_string(project.default_output()).unwrap();
    assert!(page.contains(PINNED_URL), "{page}");
    assert!(page.contains("integrity="), "{page}");
    assert!(
        stderr_of(&output).contains(PINNED_URL),
        "{}",
        stderr_of(&output)
    );
}

#[test]
fn a_tampered_installed_copy_exits_2_and_writes_no_file() {
    let project = Project::new();
    fs::create_dir_all(project.installed_file().parent().unwrap()).unwrap();
    fs::write(project.installed_file(), "not the pinned bytes").unwrap();

    let output = project.html(&[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr_of(&output).contains("does not match"),
        "{}",
        stderr_of(&output)
    );
    assert!(!project.default_output().exists());
}

#[test]
fn mermaid_path_is_inlined_and_named_on_stderr() {
    let project = Project::new();
    fs::write(project.dir().join("m.js"), "/*local-mermaid-marker*/").unwrap();

    let output = project.html(&["--mermaid", "m.js"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let page = fs::read_to_string(project.default_output()).unwrap();
    assert!(page.contains("/*local-mermaid-marker*/"), "{page}");
    assert!(!page.contains(PINNED_URL), "{page}");
    assert!(
        stderr_of(&output).contains("m.js"),
        "{}",
        stderr_of(&output)
    );
}

#[test]
fn a_missing_mermaid_path_exits_2_naming_it() {
    let project = Project::new();

    let output = project.html(&["--mermaid", "absent-mermaid.js"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr_of(&output).contains("absent-mermaid.js"));
    assert!(!project.default_output().exists());
}

#[test]
fn json_output_names_the_written_path_and_the_mermaid_source() {
    let project = Project::new();

    let output = project.html(&["--json"]);

    let text = stdout_of(&output);
    assert_eq!(text.trim_end().lines().count(), 1, "{text}");
    let doc: Value = serde_json::from_str(&text).unwrap();
    assert!(doc["path"]
        .as_str()
        .unwrap()
        .ends_with(".living-docs/html/index.html"));
    assert!(doc["mermaid"].as_str().unwrap().contains(PINNED_URL));
}

#[test]
fn plain_output_names_the_written_path() {
    let project = Project::new();

    let output = project.html(&["--plain"]);

    assert!(
        stdout_of(&output).contains(".living-docs/html/index.html"),
        "{}",
        stdout_of(&output)
    );
}

#[test]
fn an_unwritable_out_fails_and_leaves_no_temp_file() {
    let project = Project::new();
    let blocked = project.dir().join("blocked");
    fs::create_dir_all(&blocked).unwrap();

    let output = project.html(&["--out", "blocked"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(!stderr_of(&output).is_empty());
    let leftovers: Vec<_> = fs::read_dir(project.dir())
        .unwrap()
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".part"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[test]
fn the_docs_bundle_is_never_written_to() {
    let project = Project::new();
    let before = snapshot(&project.dir().join("docs"));

    project.html(&[]);
    project.html(&["--out", "docs-copy.html"]);

    assert_eq!(snapshot(&project.dir().join("docs")), before);
}
