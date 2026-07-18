use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

/// Fixtures live under `skills/living-docs/tests/fixtures` relative to the repo
/// root; `CARGO_MANIFEST_DIR` anchors this at compile time regardless of the
/// working directory `cargo test` is invoked from.
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .join("skills/living-docs/tests/fixtures")
        .join(name)
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("living-docs-mermaid-test-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_mermaid_only(path: &Path) -> Output {
    living_docs()
        .args(["check", "--mermaid-only", path.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs check --mermaid-only")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn fixture_10_valid_diagrams_pass_clean() {
    let output = run_mermaid_only(&fixture("10-mermaid-valid"));
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected clean, got:\n{stdout}\n{}",
        stderr_of(&output)
    );
    assert!(!stdout.contains("FAIL"));
}

#[test]
fn fixture_11_invalid_diagram_fails_with_a_file_line_pointer() {
    let output = run_mermaid_only(&fixture("11-mermaid-invalid"));
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected a parse failure, got:\n{stdout}"
    );
    assert!(
        stdout.contains("FAIL"),
        "expected a FAIL line, got:\n{stdout}"
    );
    assert!(
        stdout.contains("doc.md:"),
        "expected a file:line pointer, got:\n{stdout}"
    );
}

#[test]
fn mermaid_only_with_no_fences_is_clean_without_requiring_docker() {
    let bundle = temp_dir("no-fences");
    fs::write(bundle.join("plain.md"), "# Plain\n\nNo diagrams here.\n").unwrap();

    let output = run_mermaid_only(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected clean, got:\n{stdout}"
    );
    assert!(
        stdout.contains("OK: 0 diagram(s)"),
        "expected an empty-sweep summary, got:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&bundle);
}

fn run_mermaid_only_without_docker_on_path(path: &Path) -> Output {
    living_docs()
        .args(["check", "--mermaid-only", path.to_str().unwrap()])
        .env("PATH", "/nonexistent")
        .output()
        .expect("failed to run living-docs check --mermaid-only")
}

#[test]
fn mermaid_only_validates_without_docker_on_path() {
    let valid = run_mermaid_only_without_docker_on_path(&fixture("10-mermaid-valid"));
    assert_eq!(
        valid.status.code(),
        Some(0),
        "expected clean with docker off PATH, got:\n{}\n{}",
        stdout_of(&valid),
        stderr_of(&valid)
    );

    let invalid = run_mermaid_only_without_docker_on_path(&fixture("11-mermaid-invalid"));
    let invalid_stdout = stdout_of(&invalid);
    assert_eq!(
        invalid.status.code(),
        Some(1),
        "expected a parse failure with docker off PATH, got:\n{invalid_stdout}"
    );
    assert!(
        invalid_stdout.contains("doc.md:"),
        "expected a file:line pointer, got:\n{invalid_stdout}"
    );
}
