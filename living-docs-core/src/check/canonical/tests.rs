use super::*;
use crate::test_support::MapStore;
use std::collections::BTreeMap;
use std::path::Path;

fn store_with(path: &str, contents: &str) -> (MapStore, Vec<PathBuf>) {
    let mut files = BTreeMap::new();
    files.insert(PathBuf::from(path), contents.to_owned());
    let all_md = vec![PathBuf::from(path)];
    (MapStore { files }, all_md)
}

#[test]
fn frontmatter_block_slices_between_the_fences() {
    assert_eq!(
        frontmatter_block("---\ntype: ADR\n---\n\nBody.\n"),
        Some("type: ADR")
    );
}

#[test]
fn frontmatter_block_is_none_without_a_leading_fence() {
    assert_eq!(frontmatter_block("# No frontmatter\n"), None);
}

#[test]
fn check_canonical_frontmatter_accepts_an_already_canonical_record() {
    let canonical = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nBody.\n";
    let (store, all_md) = store_with("/bundle/adr/0001-doc.md", canonical);
    let mut reporter = Reporter::new();

    check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

    assert!(reporter.into_violations().is_empty());
}

#[test]
fn check_canonical_frontmatter_flags_a_trailing_yaml_comment() {
    let commented = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.  # a comment\n---\n\n# Quokka Caching\n\nBody.\n";
    let (store, all_md) = store_with("/bundle/adr/0001-doc.md", commented);
    let mut reporter = Reporter::new();

    check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

    let violations = reporter.into_violations();
    assert_eq!(violations.len(), 1);
    assert!(violations[0].1.contains("living-docs manage fmt"));
}

#[test]
fn check_canonical_frontmatter_flags_reordered_keys() {
    let reordered = "---\ntitle: Quokka Caching\ntype: ADR\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nBody.\n";
    let (store, all_md) = store_with("/bundle/adr/0001-doc.md", reordered);
    let mut reporter = Reporter::new();

    check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

    assert_eq!(reporter.into_violations().len(), 1);
}

#[test]
fn check_canonical_frontmatter_flags_extra_spacing_around_a_value() {
    let spaced = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.   \n---\n\n# Quokka Caching\n\nBody.\n";
    let (store, all_md) = store_with("/bundle/adr/0001-doc.md", spaced);
    let mut reporter = Reporter::new();

    check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

    assert_eq!(reporter.into_violations().len(), 1);
}

#[test]
fn check_canonical_frontmatter_skips_records_outside_cli_owned_directories() {
    let outside_dir = "reference";
    assert!(
        crate::doc_type::spec_for_dir(outside_dir).is_none(),
        "fixture premise broken: `{outside_dir}` is now a registry-owned directory — pick another",
    );

    let commented = "---\ntype: Research\ntitle: Field Notes  # a comment\n---\n\nBody.\n";
    for path in [
        format!("/bundle/{outside_dir}/0001-notes.md"),
        "/bundle/0001-notes.md".to_string(),
    ] {
        let (store, all_md) = store_with(&path, commented);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

        assert!(reporter.into_violations().is_empty());
    }
}

#[test]
fn check_canonical_frontmatter_flags_every_cli_owned_directory() {
    let commented = "---\ntype: ADR\ntitle: X  # comment\n---\n\nBody.\n";
    for spec in crate::doc_type::DOC_TYPES {
        let crate::doc_type::Identity::Numbered { dir } = spec.identity else {
            continue;
        };
        let path = format!("/bundle/{dir}/0001-doc.md");
        let (store, all_md) = store_with(&path, commented);
        let mut reporter = Reporter::new();

        check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

        assert_eq!(reporter.into_violations().len(), 1);
    }
}

/// Guards the fixture premise the same way
/// `check_canonical_frontmatter_skips_records_outside_cli_owned_directories`
/// guards its own: the singleton filename these tests hardcode is driven
/// off `doc_type::DOC_TYPES`, so a renamed row fails this assertion
/// loudly instead of the fixture silently exercising the wrong path.
fn assert_constitution_md_is_still_the_registered_singleton() {
    let spec = crate::doc_type::spec_for("constitution")
        .expect("fixture premise broken: `constitution` is no longer a registered token");
    assert_eq!(
        spec.identity,
        crate::doc_type::Identity::Singleton {
            file: "constitution.md"
        },
        "fixture premise broken: the constitution row no longer names constitution.md — update this fixture",
    );
}

#[test]
fn check_canonical_frontmatter_flags_a_hand_written_bundle_root_singleton() {
    assert_constitution_md_is_still_the_registered_singleton();
    let commented = "---\ntype: Constitution\ntitle: X  # comment\n---\n\nBody.\n".to_string();
    let (store, all_md) = store_with("/bundle/constitution.md", &commented);
    let mut reporter = Reporter::new();

    check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

    let violations = reporter.into_violations();
    assert_eq!(violations.len(), 1);
    assert!(violations[0].1.contains("living-docs manage fmt"));
}

#[test]
fn check_canonical_frontmatter_does_not_flag_the_singleton_filename_nested_in_a_subdirectory() {
    assert_constitution_md_is_still_the_registered_singleton();
    let commented = "---\ntype: Constitution\ntitle: X  # comment\n---\n\nBody.\n".to_string();
    let (store, all_md) = store_with("/bundle/reference/constitution.md", &commented);
    let mut reporter = Reporter::new();

    check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

    assert!(reporter.into_violations().is_empty());
}

#[test]
fn check_canonical_frontmatter_accepts_the_canonical_bundle_root_singleton() {
    assert_constitution_md_is_still_the_registered_singleton();
    let path = Path::new("/bundle/constitution.md");
    let hand_written = "---\ntype: Constitution\ntitle: X\n---\n\nBody.\n";
    let canonical = to_canonical_markdown(&extract_record(path, hand_written));
    let (store, all_md) = store_with("/bundle/constitution.md", &canonical);
    let mut reporter = Reporter::new();

    check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

    assert!(reporter.into_violations().is_empty());
}

#[test]
fn check_canonical_frontmatter_skips_a_record_with_no_frontmatter_block() {
    let (store, all_md) = store_with("/bundle/adr/notes.md", "# Just a heading\n\nBody.\n");
    let mut reporter = Reporter::new();

    check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

    assert!(reporter.into_violations().is_empty());
}

#[test]
fn check_canonical_frontmatter_skips_reserved_files() {
    let commented = "---\ntype: ADR\ntitle: X  # comment\n---\n\nBody.\n";
    let (store, all_md) = store_with("/bundle/adr/index.md", commented);
    let mut reporter = Reporter::new();

    check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

    assert!(reporter.into_violations().is_empty());
}

#[test]
fn check_canonical_frontmatter_accepts_the_same_record_after_a_fmt_pass() {
    let commented = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.  # a comment\n---\n\n# Quokka Caching\n\nBody.\n";
    let path = Path::new("/bundle/adr/0001-doc.md");
    let fmt_pass = to_canonical_markdown(&extract_record(path, commented));
    let (store, all_md) = store_with("/bundle/adr/0001-doc.md", &fmt_pass);
    let mut reporter = Reporter::new();

    check_canonical_frontmatter(&store, Path::new("/bundle"), &all_md, &mut reporter);

    assert!(reporter.into_violations().is_empty());
}
