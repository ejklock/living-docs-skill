use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn run_check(bundle: &Path) -> Output {
    living_docs()
        .args(["check", bundle.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs check")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Fixtures live under `skills/living-docs/tests/fixtures` relative to the repo
/// root; `CARGO_MANIFEST_DIR` anchors this at compile time regardless of the
/// working directory `cargo test` is invoked from.
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .join("skills/living-docs/tests/fixtures")
        .join(name)
        .join("docs")
}

fn temp_bundle(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir()
        .join(format!("living-docs-check-test-{label}-{nanos}"))
        .join("docs");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(bundle: &Path, rel: &str, contents: &str) {
    let path = bundle.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

#[test]
fn fixture_04_quoted_and_commented_frontmatter_is_clean() {
    let output = run_check(&fixture("04-frontmatter-quoted-commented"));
    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout_of(&output);
    assert!(stdout.contains("no invariant violations"));
    assert!(!stdout.contains("non-empty 'type'"));
}

#[test]
fn fixture_06_block_scalar_type_is_clean() {
    let output = run_check(&fixture("06-block-scalar-ok"));
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout_of(&output).contains("no invariant violations"));
}

#[test]
fn fixture_05_nested_key_trap_is_a_type_violation() {
    let output = run_check(&fixture("05-nested-key-trap"));
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout_of(&output).contains("non-empty 'type'"));
}

#[test]
fn fixture_07_supersede_broken_reports_has_no_matching_record() {
    let output = run_check(&fixture("07-supersede-broken"));
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout_of(&output).contains("has no matching record"));
}

#[test]
fn fixture_09_okf_canonical_bundle_is_clean() {
    let output = run_check(&fixture("09-okf-canonical"));
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout_of(&output).contains("no invariant violations"));
}

#[test]
fn missing_bundle_exits_with_code_2() {
    let bundle = temp_bundle("missing").join("does-not-exist");
    let output = run_check(&bundle);
    assert_eq!(output.status.code(), Some(2));

    let _ = fs::remove_dir_all(bundle.parent().unwrap());
}

#[test]
fn orphan_file_and_unreachable_index_are_both_reported() {
    let bundle = temp_bundle("graph");
    write(&bundle, "index.md", "# Index\n\n- [A](a/index.md)\n");
    write(
        &bundle,
        "orphan.md",
        "---\ntype: Reference\n---\n# Orphan\n",
    );
    write(&bundle, "a/index.md", "# A\n\n- [Concept](concept.md)\n");
    write(
        &bundle,
        "a/concept.md",
        "---\ntype: Reference\n---\n# Concept\n",
    );
    write(&bundle, "b/index.md", "# B\n");

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout.contains("orphan (invariant 3)"),
        "expected an orphan violation, got:\n{stdout}"
    );
    assert!(
        stdout.contains("not reachable from"),
        "expected an unreachable-index violation, got:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn constitution_md_is_exempt_from_the_directory_index_listing_requirement() {
    let bundle = temp_bundle("constitution");
    write(&bundle, "index.md", "# Index\n");
    write(
        &bundle,
        "constitution.md",
        "---\ntype: Constitution\n---\n# Constitution\n",
    );

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "constitution.md must not be flagged as an orphan:\n{stdout}"
    );
    assert!(stdout.contains("no invariant violations"));

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn non_root_index_with_frontmatter_is_a_violation() {
    let bundle = temp_bundle("index-format");
    write(&bundle, "index.md", "# Index\n\n- [A](a/index.md)\n");
    write(&bundle, "a/index.md", "---\ntype: ADR\n---\n# A\n");

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout.contains("must not carry frontmatter"));

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn root_index_frontmatter_without_okf_version_is_a_violation() {
    let bundle = temp_bundle("root-okf-missing");
    write(&bundle, "index.md", "---\ntitle: Docs\n---\n# Index\n");

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout.contains("lacks okf_version"));

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn root_index_frontmatter_with_okf_version_is_clean() {
    let bundle = temp_bundle("root-okf-present");
    write(&bundle, "index.md", "---\nokf_version: 1\n---\n# Index\n");

    let output = run_check(&bundle);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout_of(&output).contains("no invariant violations"));

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn supersede_status_is_case_insensitive_and_a_valid_chain_is_clean() {
    let bundle = temp_bundle("supersede-clean");
    write(
        &bundle,
        "index.md",
        "# Index\n\n- [Old](0001-old.md)\n- [New](0002-new.md)\n",
    );
    write(
        &bundle,
        "0001-old.md",
        "---\ntype: ADR\nstatus: superseded\nsuperseded_by: \"0002\"\n---\n# Old\n",
    );
    write(&bundle, "0002-new.md", "---\ntype: ADR\n---\n# New\n");

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected a clean supersede chain, got:\n{stdout}"
    );
    assert!(stdout.contains("no invariant violations"));

    let _ = fs::remove_dir_all(&bundle);
}
