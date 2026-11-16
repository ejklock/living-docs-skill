use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard};

mod common;
use common::Scratch;

const VERSION: &str = "12.3.0";
const URL: &str = "https://cdn.jsdelivr.net/npm/mermaid@12.3.0/dist/mermaid.min.js";

const WRITE_WRONG_BYTES: &str = r#"out=""
while [ $# -gt 0 ]; do
  if [ "$1" = "-o" ]; then out="$2"; fi
  shift
done
printf 'not the pinned bytes' > "$out""#;

fn destination(data_home: &Path) -> PathBuf {
    data_home.join("living-docs/mermaid").join(VERSION)
}

/// Writing an executable while another thread forks can make that exec fail
/// with ETXTBSY, so stub writes and process spawns take turns.
fn serial() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn stub_curl(bin: &Path, script_body: &str) {
    let _guard = serial();
    fs::create_dir_all(bin).unwrap();
    let path = bin.join("curl");
    fs::write(&path, format!("#!/bin/sh\n{script_body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn install(scratch: &Path, path_dirs: &Path, args: &[&str]) -> Output {
    let _guard = serial();
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
        .args(["manage", "install", "mermaid"])
        .args(args)
        .env("PATH", path_dirs)
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("HOME", scratch.join("home"))
        .output()
        .expect("failed to run living-docs install mermaid")
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                found.extend(files_under(&path));
            } else {
                found.push(path);
            }
        }
    }
    found
}

#[test]
fn a_download_whose_hash_differs_from_the_pinned_one_fails_and_leaves_no_file() {
    let scratch = Scratch::new("install-mermaid-mismatch");
    let bin = scratch.join("bin");
    stub_curl(&bin, WRITE_WRONG_BYTES);

    let output = install(&scratch, &bin, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("SHA-256"), "stderr: {stderr}");
    assert!(stderr.contains("mismatch"), "stderr: {stderr}");
    let left = files_under(&scratch.join("data"));
    assert!(left.is_empty(), "files left behind: {left:?}");
}

#[test]
fn a_missing_curl_fails_naming_curl() {
    let scratch = Scratch::new("install-mermaid-no-curl");
    let empty = scratch.join("empty-bin");
    fs::create_dir_all(&empty).unwrap();

    let output = install(&scratch, &empty, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("curl"), "stderr: {stderr}");
    assert!(files_under(&scratch.join("data")).is_empty());
}

#[test]
fn a_failing_curl_fails_and_leaves_no_file() {
    let scratch = Scratch::new("install-mermaid-curl-fails");
    let bin = scratch.join("bin");
    stub_curl(&bin, "exit 22");

    let output = install(&scratch, &bin, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("curl"), "stderr: {stderr}");
    let left = files_under(&scratch.join("data"));
    assert!(left.is_empty(), "files left behind: {left:?}");
}

#[test]
fn dry_run_prints_the_url_and_destination_and_writes_nothing() {
    let scratch = Scratch::new("install-mermaid-dry-run");
    let bin = scratch.join("bin");
    stub_curl(&bin, "exit 99");

    let output = install(&scratch, &bin, &["--dry-run"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains(URL), "stdout: {stdout}");
    let expected = destination(&scratch.join("data")).join("mermaid.min.js");
    assert!(
        stdout.contains(expected.to_str().unwrap()),
        "stdout: {stdout}"
    );
    assert!(!scratch.join("data").exists());
}

#[test]
fn the_install_help_lists_the_mermaid_target() {
    let _guard = serial();
    let output = Command::new(env!("CARGO_BIN_EXE_living-docs"))
        .args(["--plain", "manage", "install", "--help"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout
            .lines()
            .any(|line| line.trim_start().starts_with("mermaid")),
        "stdout: {stdout}"
    );
    let about = stdout.lines().next().unwrap_or_default();
    assert!(
        about.contains("skills") && about.contains("hooks") && about.contains("mermaid"),
        "about line: {about}"
    );
    assert!(about.contains("user data directory"), "about line: {about}");
}
