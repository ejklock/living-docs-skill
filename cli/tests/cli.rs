use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("living-docs-cli-test-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn next_on_absent_type_dir_prints_0001() {
    let docs = temp_dir("next-absent");

    let output = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "next", "adr"])
        .output()
        .expect("failed to run living-docs");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0001");

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn next_on_empty_type_dir_prints_0001() {
    let docs = temp_dir("next-empty");
    fs::create_dir_all(docs.join("adr")).unwrap();

    let output = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "next", "adr"])
        .output()
        .expect("failed to run living-docs");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0001");

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn next_increments_past_the_highest_existing_number() {
    let docs = temp_dir("next-existing");
    let adr_dir = docs.join("adr");
    fs::create_dir_all(&adr_dir).unwrap();
    fs::write(adr_dir.join("0001-old.md"), "---\ntype: ADR\n---\n# Old\n").unwrap();

    let output = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "next", "adr"])
        .output()
        .expect("failed to run living-docs");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0002");

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn next_ignores_files_without_the_nnnn_dash_prefix() {
    let docs = temp_dir("next-ignore-others");
    let adr_dir = docs.join("adr");
    fs::create_dir_all(&adr_dir).unwrap();
    fs::write(adr_dir.join("index.md"), "# Index\n").unwrap();
    fs::write(adr_dir.join("0003-current.md"), "---\ntype: ADR\n---\n").unwrap();

    let output = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "next", "adr"])
        .output()
        .expect("failed to run living-docs");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0004");

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn unknown_subcommand_exits_with_code_2() {
    let output = living_docs()
        .arg("bogus")
        .output()
        .expect("failed to run living-docs");

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn missing_required_argument_exits_with_code_2() {
    let output = living_docs()
        .arg("next")
        .output()
        .expect("failed to run living-docs");

    assert_eq!(output.status.code(), Some(2));
}

fn write_record_with_visibility(docs: &Path, dir: &str, filename: &str, visibility: Option<&str>) {
    let type_dir = docs.join(dir);
    fs::create_dir_all(&type_dir).unwrap();
    let visibility_line = visibility
        .map(|v| format!("visibility: {v}\n"))
        .unwrap_or_default();
    let contents =
        format!("---\ntype: ADR\ntitle: {filename}\n{visibility_line}---\n# {filename}\n");
    fs::write(type_dir.join(filename), contents).unwrap();
}

#[test]
fn export_with_visibility_flag_writes_only_matching_records() {
    let docs = temp_dir("export-visibility-filter");
    write_record_with_visibility(&docs, "adr", "0001-public.md", Some("public"));
    write_record_with_visibility(&docs, "adr", "0002-private.md", Some("private"));
    write_record_with_visibility(&docs, "adr", "0003-absent.md", None);
    let out_dir = temp_dir("export-visibility-filter-out");
    fs::remove_dir_all(&out_dir).unwrap();

    let output = living_docs()
        .args([
            "--docs-dir",
            docs.to_str().unwrap(),
            "export",
            out_dir.to_str().unwrap(),
            "--visibility",
            "public,showcase",
        ])
        .output()
        .expect("failed to run living-docs export");

    assert!(output.status.success());
    assert!(out_dir.join("adr/0001-public.md").exists());
    assert!(!out_dir.join("adr/0002-private.md").exists());
    assert!(!out_dir.join("adr/0003-absent.md").exists());

    let _ = fs::remove_dir_all(&docs);
    let _ = fs::remove_dir_all(&out_dir);
}

#[test]
fn export_without_visibility_flag_writes_every_record() {
    let docs = temp_dir("export-visibility-unset");
    write_record_with_visibility(&docs, "adr", "0001-public.md", Some("public"));
    write_record_with_visibility(&docs, "adr", "0002-private.md", Some("private"));
    write_record_with_visibility(&docs, "adr", "0003-absent.md", None);
    let out_dir = temp_dir("export-visibility-unset-out");
    fs::remove_dir_all(&out_dir).unwrap();

    let output = living_docs()
        .args([
            "--docs-dir",
            docs.to_str().unwrap(),
            "export",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run living-docs export");

    assert!(output.status.success());
    assert!(out_dir.join("adr/0001-public.md").exists());
    assert!(out_dir.join("adr/0002-private.md").exists());
    assert!(out_dir.join("adr/0003-absent.md").exists());

    let _ = fs::remove_dir_all(&docs);
    let _ = fs::remove_dir_all(&out_dir);
}

#[test]
fn check_on_missing_bundle_exits_with_code_2() {
    // `check` takes its own positional bundle argument (default `docs`), not the
    // global `--docs-dir` — see cli/tests/check_core.rs for its full behavior.
    let docs = temp_dir("check-missing-bundle");
    let missing = docs.join("nope");

    let output = living_docs()
        .args(["check", missing.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs");

    assert_eq!(output.status.code(), Some(2));

    let _ = fs::remove_dir_all(&docs);
}
