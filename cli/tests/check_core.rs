use living_docs_core::doc_type::{self, Identity};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

mod common;
use common::{fixture, living_docs, run_check, stdout_of, write};

fn temp_bundle(label: &str) -> PathBuf {
    common::temp_bundle("check", label)
}

fn run_fmt(bundle: &Path) -> Output {
    living_docs()
        .args(["fmt", bundle.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs fmt")
}

fn run_authoring_verb(verb: &str, docs_dir: &Path, doc_type: &str, title: &str) -> Output {
    living_docs()
        .args([
            "--docs-dir",
            docs_dir.to_str().unwrap(),
            verb,
            doc_type,
            title,
        ])
        .output()
        .unwrap_or_else(|_| panic!("failed to run living-docs {verb}"))
}

fn run_new(docs_dir: &Path, doc_type: &str, title: &str) -> Output {
    run_authoring_verb("new", docs_dir, doc_type, title)
}

/// The exact retired-record callout line a Superseded fixture must open
/// with, sourced from the same module `living-docs fmt` writes through.
fn superseded_callout(successor: &str) -> String {
    living_docs_core::callout::expected(Some("superseded"), Some(successor))
        .expect("a superseded status always yields a callout")
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
        "---\ntype: Constitution\ntitle: Constitution\ndescription: \"\"\n---\n\n# Constitution\n\n## Product\n\np.\n\n## Scope Boundaries\n\ns.\n\n## Non-negotiables\n\nn.\n",
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
        &format!(
            "---\ntype: ADR\ntitle: Old\ndescription: \"\"\nstatus: superseded\nsuperseded_by: 0002\n---\n\n{}\n\n# Old\n",
            superseded_callout("0002-new.md")
        ),
    );
    write(
        &bundle,
        "0002-new.md",
        "---\ntype: ADR\ntitle: New\ndescription: \"\"\n---\n\n# New\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n",
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
        "---\ntitle: Hand Written\ntype: ADR  # a comment\ndescription: Written by hand.\n---\n\n# Hand Written\n\n## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n",
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

/// Decision point 7's canonical-frontmatter ownership: a
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
        "---\ntitle: Acme  # a comment\ntype: Constitution\n---\n\n# Acme\n\n## Product\n\np.\n\n## Scope Boundaries\n\ns.\n\n## Non-negotiables\n\nn.\n",
    );

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1), "got:\n{stdout}");
    assert!(stdout.contains("living-docs fmt"), "got:\n{stdout}");

    let _ = fs::remove_dir_all(&bundle);
}
