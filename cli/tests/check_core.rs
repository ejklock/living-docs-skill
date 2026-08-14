use living_docs_core::doc_type::{self, Identity};
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

fn run_fmt(bundle: &Path) -> Output {
    living_docs()
        .args(["fmt", bundle.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs fmt")
}

fn run_new(docs_dir: &Path, doc_type: &str, title: &str) -> Output {
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

fn run_brief(docs_dir: &Path, doc_type: &str, title: &str) -> Output {
    living_docs()
        .args([
            "--docs-dir",
            docs_dir.to_str().unwrap(),
            "brief",
            doc_type,
            title,
        ])
        .output()
        .expect("failed to run living-docs brief")
}

fn singleton_token() -> &'static str {
    doc_type::DOC_TYPES
        .iter()
        .find(|spec| matches!(spec.identity, Identity::Singleton { .. }))
        .expect("registry must carry at least one singleton row")
        .token
}

/// The fixture's `type` value is spread across three files as a double-quoted,
/// single-quoted, and trailing-commented scalar to prove the type-extraction
/// invariant tolerates all three forms. Its docs sit at the bundle root, so
/// the ADR 0019 canonical round-trip check does not apply (ADR 0022 scopes it
/// to CLI-owned type directories) and the bundle checks clean.
#[test]
fn fixture_04_quoted_and_commented_frontmatter_parses_type_and_checks_clean() {
    let output = run_check(&fixture("04-frontmatter-quoted-commented"));
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(0), "got:\n{stdout}");
    assert!(!stdout.contains("non-empty 'type'"));
    assert!(!stdout.contains("living-docs fmt"));
}

/// The fixture's `type: |\n  ADR\n` block scalar proves the type-extraction
/// invariant tolerates YAML block-scalar syntax; at the bundle root it is
/// outside the canonical check's ADR 0022 scope and the bundle checks clean.
#[test]
fn fixture_06_block_scalar_type_parses_and_checks_clean() {
    let output = run_check(&fixture("06-block-scalar-ok"));
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(0), "got:\n{stdout}");
    assert!(!stdout.contains("non-empty 'type'"));
    assert!(!stdout.contains("living-docs fmt"));
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
        "---\ntype: Constitution\ntitle: Constitution\ndescription: \"\"\n---\n\n# Constitution\n",
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
fn invalid_visibility_value_fails_and_correcting_it_to_the_domain_passes() {
    let bundle = temp_bundle("visibility");
    write(&bundle, "index.md", "# Index\n\n- [Concept](concept.md)\n");
    write(
        &bundle,
        "concept.md",
        "---\ntype: Reference\ntitle: Concept\ndescription: \"\"\nvisibility: pubic\n---\n\n# Concept\n",
    );

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout.contains(
            "invalid visibility 'pubic' (allowed: private|public|showcase; absent means private)"
        ),
        "expected an invalid-visibility violation, got:\n{stdout}"
    );

    write(
        &bundle,
        "concept.md",
        "---\ntype: Reference\ntitle: Concept\ndescription: \"\"\nvisibility: public\n---\n\n# Concept\n",
    );

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected a clean check after correcting visibility, got:\n{stdout}"
    );
    assert!(stdout.contains("no invariant violations"));

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
        "---\ntype: ADR\ntitle: Old\ndescription: \"\"\nstatus: superseded\nsuperseded_by: 0002\n---\n\n# Old\n",
    );
    write(
        &bundle,
        "0002-new.md",
        "---\ntype: ADR\ntitle: New\ndescription: \"\"\n---\n\n# New\n",
    );

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

/// ADR 0019, AC ac-s3-1: a hand-written record (here, a YAML comment on
/// `type:` and a reordered `title:` key) fails `check` with a violation
/// naming `living-docs fmt`, and the same record passes after `fmt` rewrites
/// it — the deterministic remediation loop the check's message promises.
#[test]
fn hand_written_frontmatter_fails_check_and_passes_after_fmt() {
    let bundle = temp_bundle("hand-written-fmt-roundtrip");
    write(
        &bundle,
        "adr/0001-doc.md",
        "---\ntitle: Hand Written\ntype: ADR  # a comment\ndescription: Written by hand.\n---\n\n# Hand Written\n\nBody.\n",
    );

    let before = run_check(&bundle);
    let before_stdout = stdout_of(&before);
    assert_eq!(before.status.code(), Some(1), "got:\n{before_stdout}");
    assert!(
        before_stdout.contains("living-docs fmt"),
        "got:\n{before_stdout}"
    );

    let fmt_output = run_fmt(&bundle);
    assert!(
        fmt_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&fmt_output.stderr)
    );

    let after = run_check(&bundle);
    let after_stdout = stdout_of(&after);
    assert!(
        !after_stdout.contains("living-docs fmt"),
        "expected fmt to clear the canonical violation, got:\n{after_stdout}"
    );

    let _ = fs::remove_dir_all(&bundle);
}

/// ADR 0019, AC ac-s3-2: a fresh `new` scaffold, for every doc type carrying
/// a template, is a canonical round-trip fixed point — `check` reports no
/// canonical violation on it, with no `fmt` pass required.
#[test]
fn fresh_new_scaffold_is_a_canonical_round_trip_fixed_point_for_every_doc_type() {
    let numbered_types = doc_type::DOC_TYPES
        .iter()
        .filter(|spec| matches!(spec.identity, Identity::Numbered { .. }))
        .map(|spec| spec.token);
    for doc_type in numbered_types {
        let docs = temp_bundle(&format!("fresh-scaffold-{doc_type}"));

        let new_output = run_new(&docs, doc_type, "Fixed Point");
        assert!(
            new_output.status.success(),
            "{doc_type}: stderr: {}",
            String::from_utf8_lossy(&new_output.stderr)
        );

        let output = run_check(&docs);
        let stdout = stdout_of(&output);
        assert!(
            !stdout.contains("living-docs fmt"),
            "{doc_type} scaffold is not a canonical round-trip fixed point:\n{stdout}"
        );

        let _ = fs::remove_dir_all(&docs);
    }
}

/// ADR 0026 decision point 7: a briefed bundle-root singleton
/// (`constitution.md`) is CLI-owned for the canonical-frontmatter invariant
/// and exempt from directory-index membership — `check` reports neither a
/// dangling markdown link left over from an unfilled slot nor a
/// non-canonical frontmatter block.
#[test]
fn briefed_singleton_scaffold_passes_check_over_its_own_bundle() {
    let docs = temp_bundle("briefed-singleton");
    write(&docs, "index.md", "# Index\n");

    let brief_output = run_brief(&docs, singleton_token(), "Acme Constitution");
    assert!(
        brief_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&brief_output.stderr)
    );

    let singleton_file = doc_type::DOC_TYPES
        .iter()
        .find_map(|spec| match spec.identity {
            Identity::Singleton { file } => Some(file),
            Identity::Numbered { .. } | Identity::Named { .. } => None,
        })
        .expect("registry must carry at least one singleton row");
    assert!(
        docs.join(singleton_file).is_file(),
        "brief did not scaffold {singleton_file} at the bundle root"
    );

    let output = run_check(&docs);
    let stdout = stdout_of(&output);
    assert_eq!(output.status.code(), Some(0), "got:\n{stdout}");
    assert!(stdout.contains("no invariant violations"), "got:\n{stdout}");

    let _ = fs::remove_dir_all(&docs);
}

/// The other half of decision point 7's canonical-frontmatter ownership: a
/// hand-written bundle-root singleton with non-canonical frontmatter is
/// caught by `check`, not silently accepted because it sits outside every
/// CLI-owned type directory. `constitution.md` is hardcoded, matching
/// `constitution_md_is_exempt_from_the_directory_index_listing_requirement`
/// above, with the same registry premise guarded explicitly so a renamed
/// singleton row fails this assertion instead of silently testing the
/// wrong path.
#[test]
fn hand_written_bundle_root_singleton_fails_check_with_the_fmt_remediation() {
    let spec = doc_type::spec_for("constitution")
        .expect("fixture premise broken: `constitution` is no longer a registered token");
    assert_eq!(
        spec.identity,
        Identity::Singleton {
            file: "constitution.md"
        },
        "fixture premise broken: the constitution row no longer names constitution.md — update this fixture",
    );

    let bundle = temp_bundle("hand-written-singleton");
    write(&bundle, "index.md", "# Index\n");
    write(
        &bundle,
        "constitution.md",
        "---\ntitle: Acme  # a comment\ntype: Constitution\n---\n\n# Acme\n",
    );

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1), "got:\n{stdout}");
    assert!(stdout.contains("living-docs fmt"), "got:\n{stdout}");

    let _ = fs::remove_dir_all(&bundle);
}
