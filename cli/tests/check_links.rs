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
        .join(format!("living-docs-links-test-{label}-{nanos}"))
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
fn fixture_01_fenced_example_link_is_not_followed_and_stays_clean() {
    let output = run_check(&fixture("01-fence-link-clean"));
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected clean, got:\n{stdout}"
    );
    assert!(!stdout.contains("broken link"));
}

#[test]
fn fixture_02_inline_link_outside_a_fence_is_reported_broken() {
    let output = run_check(&fixture("02-fence-link-dirty"));
    let stdout = stdout_of(&output);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout.contains("broken link"),
        "expected a broken link violation, got:\n{stdout}"
    );
}

#[test]
fn fixture_03_every_link_form_resolves_clean() {
    let output = run_check(&fixture("03-link-forms"));
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected clean, got:\n{stdout}"
    );
    assert!(!stdout.contains("broken link"));
}

#[test]
fn fixture_08_reference_style_broken_link_is_reported_broken() {
    let output = run_check(&fixture("08-reference-link-broken"));
    let stdout = stdout_of(&output);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout.contains("broken link"),
        "expected a broken link violation, got:\n{stdout}"
    );
}

#[test]
fn fixture_09_okf_canonical_stays_clean_with_link_checking_active() {
    let output = run_check(&fixture("09-okf-canonical"));
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected clean, got:\n{stdout}"
    );
    assert!(!stdout.contains("broken link"));
    assert!(stdout.contains("no invariant violations"));
}

#[test]
fn broken_image_destination_is_reported_broken() {
    let bundle = temp_bundle("image");
    write(&bundle, "index.md", "# Index\n\n- [Foo](foo.md)\n");
    write(
        &bundle,
        "foo.md",
        "---\ntype: Reference\n---\n# Foo\n\n![missing](./no-such.png)\n",
    );

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout.contains("broken link"),
        "expected a broken link violation for the image, got:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn bundle_relative_link_to_an_existing_file_is_clean() {
    let bundle = temp_bundle("bundle-relative");
    write(
        &bundle,
        "index.md",
        "# Index\n\n- [Foo](foo.md)\n- [A](a/index.md)\n",
    );
    write(
        &bundle,
        "foo.md",
        "---\ntype: Reference\n---\n# Foo\n\n[a](/a/target.md)\n",
    );
    write(&bundle, "a/index.md", "# A\n\n- [Target](target.md)\n");
    write(
        &bundle,
        "a/target.md",
        "---\ntype: Reference\n---\n# Target\n",
    );

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected clean, got:\n{stdout}"
    );
    assert!(!stdout.contains("broken link"));

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn external_and_anchor_only_links_are_never_flagged() {
    let bundle = temp_bundle("external");
    write(&bundle, "index.md", "# Index\n\n- [Foo](foo.md)\n");
    write(
        &bundle,
        "foo.md",
        "---\ntype: Reference\n---\n# Foo\n\n[site](https://example.com/missing)\n[mail](mailto:a@b.com)\n[phone](tel:+15551234567)\n[anchor](#nowhere)\n",
    );

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected clean, got:\n{stdout}"
    );
    assert!(!stdout.contains("broken link"));

    let _ = fs::remove_dir_all(&bundle);
}
