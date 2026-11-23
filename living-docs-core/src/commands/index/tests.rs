use super::*;

/// Fitness function: the token set `index` regenerates with no explicit type
/// equals the registry's Identity::Numbered, Identity::Named and
/// Identity::Nested token set, in the registry's own order.
#[test]
fn all_type_tokens_matches_every_numbered_named_and_nested_registry_token_in_order() {
    assert_eq!(
        all_type_tokens(),
        vec!["adr", "prd", "issue", "research", "view", "domain"]
    );
}

#[test]
fn a_nested_domain_type_is_listed_like_a_named_one() {
    assert!(is_named("domain"));
    assert!(is_named("view"));
    assert!(!is_named("adr"));
}

/// The `constitution` row is a deliberate exclusion, not an oversight:
/// a singleton has no directory index for the bare sweep to regenerate.
#[test]
fn all_type_tokens_excludes_the_constitution_singleton() {
    assert!(!all_type_tokens().contains(&"constitution".to_string()));
}

#[test]
fn numbered_prefix_accepts_four_digit_dash_form() {
    assert_eq!(numbered_prefix("0007-old.md"), Some(7));
}

#[test]
fn numbered_prefix_rejects_index_and_malformed_names() {
    assert_eq!(numbered_prefix("index.md"), None);
    assert_eq!(numbered_prefix("12-old.md"), None);
    assert_eq!(numbered_prefix("abcd-old.md"), None);
}

#[test]
fn fallback_preamble_is_minimal_for_a_fresh_file() {
    assert_eq!(fallback_preamble("", "adr"), "# ADRs\n\n");
}

#[test]
fn fallback_preamble_wraps_unmarked_existing_content() {
    assert_eq!(
        fallback_preamble("Custom intro.\n", "prd"),
        "Custom intro.\n\n"
    );
}

#[test]
fn find_boundary_offset_locates_the_adr_active_heading() {
    let existing = "# ADRs\n\nIntro.\n\n## Active\n\n* [0001 — X](0001-x.md) - Proposed\n";
    let offset = find_boundary_offset(existing).unwrap();
    assert_eq!(
        &existing[offset..],
        "## Active\n\n* [0001 — X](0001-x.md) - Proposed\n"
    );
}

#[test]
fn find_boundary_offset_locates_the_first_row_for_non_adr_types() {
    let existing = "# PRDs\n\nIntro.\n\n* [0001 — X](0001-x.md) - Draft\n";
    let offset = find_boundary_offset(existing).unwrap();
    assert_eq!(&existing[offset..], "* [0001 — X](0001-x.md) - Draft\n");
}

#[test]
fn find_boundary_offset_locates_a_legacy_heading_regardless_of_its_text() {
    let existing = "# Issues\n\nIntro.\n\n## Done\n\n* [0001 — X](0001-x.md) - closed\n";
    let offset = find_boundary_offset(existing).unwrap();
    assert_eq!(
        &existing[offset..],
        "## Done\n\n* [0001 — X](0001-x.md) - closed\n"
    );
}

#[test]
fn find_boundary_offset_locates_a_hand_maintained_table_header_row() {
    let existing = "# ADRs\n\nIntro.\n\n| # | Decision | Status |\n|---|---|---|\n| [0001](0001-x.md) | X | Accepted |\n";
    let offset = find_boundary_offset(existing).unwrap();
    assert_eq!(
        &existing[offset..],
        "| # | Decision | Status |\n|---|---|---|\n| [0001](0001-x.md) | X | Accepted |\n"
    );
}

#[test]
fn is_boundary_line_detects_a_numbered_listing_table_header() {
    assert!(is_boundary_line("| # | Decision | Status |"));
}

#[test]
fn is_boundary_line_detects_a_table_row_whose_first_cell_is_a_record_link() {
    assert!(is_boundary_line("| [0001](0001-x.md) | X | Accepted |"));
    assert!(is_boundary_line("| [0007-legacy-row | X | Accepted |"));
}

#[test]
fn is_boundary_line_ignores_an_unrelated_table_row() {
    assert!(!is_boundary_line("| Some | Other | Row |"));
    assert!(!is_boundary_line("Just prose, not a table at all."));
}

struct DiskStore {
    files: std::collections::BTreeMap<std::path::PathBuf, String>,
}

impl DocStore for DiskStore {
    fn list(&self, root: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
        Ok(self
            .files
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect())
    }

    /// Records come from `files`; an `index.md` is served from disk, where the
    /// scratch bundle keeps the hand-written one the run migrates.
    fn read(&self, path: &Path) -> std::io::Result<String> {
        if path.file_name().is_some_and(|name| name == "index.md") {
            return std::fs::read_to_string(path);
        }
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "not found"))
    }

    fn write(&self, _path: &Path, _contents: &str) -> std::io::Result<()> {
        Ok(())
    }

    fn rename(&self, _from: &Path, _to: &Path) -> std::io::Result<()> {
        Ok(())
    }
}

/// Runs `index` twice over a one-record bundle whose index starts as
/// `hand_index`, returning the first and second regenerated contents.
fn migrate_twice(name: &str, doc_type: &str, hand_index: &str) -> (String, String) {
    let root = std::env::temp_dir().join(format!("ld-index-{name}-{}", std::process::id()));
    let type_dir = root.join(doc_type);
    std::fs::create_dir_all(&type_dir).expect("create scratch type dir");
    let index_path = type_dir.join("index.md");
    std::fs::write(&index_path, hand_index).expect("write hand index");
    let mut files = std::collections::BTreeMap::new();
    files.insert(
        type_dir.join("0001-foo.md"),
        "---\ntitle: Foo\nstatus: Draft\n---\n# Foo\n".to_string(),
    );
    let store = DiskStore { files };
    let (_, first) = compute(&store, &root, doc_type).expect("first index run");
    std::fs::write(&index_path, &first).expect("write first run");
    let (_, second) = compute(&store, &root, doc_type).expect("second index run");
    let _ = std::fs::remove_dir_all(&root);
    (first, second)
}

#[test]
fn table_header_with_any_first_cell_migrates_with_its_rows() {
    let hand = "# ADRs\n\n| ADR | Decision |\n|---|---|\n| [0001](x.md) | y |\n";
    let (first, second) = migrate_twice("table-header", "adr", hand);
    assert!(!first.contains("| ADR |"), "{first}");
    assert!(first.starts_with("# ADRs\n\n## Active"), "{first}");
    assert_eq!(first, second);
}

#[test]
fn table_header_legend_above_a_listing_survives_byte_for_byte() {
    let legend = "# PRDs\n\n| Status | Meaning |\n|---|---|\n| Open | live |\n\n";
    let hand = format!("{legend}* [0001 — Foo](0001-foo.md) - Draft\n");
    let (first, second) = migrate_twice("table-legend", "prd", &hand);
    assert!(first.starts_with(legend), "{first}");
    assert_eq!(first.matches("0001-foo.md").count(), 1);
    assert_eq!(first, second);
}

#[test]
fn table_header_without_a_separator_stays_preamble() {
    let existing = "| ADR | Decision |\n| [0001](x.md) | y |\n| [0002](z.md) | w |\n";
    assert_eq!(find_boundary_offset(existing), existing.find("| [0001]"));
}

#[test]
fn prose_above_a_separator_row_is_never_the_table_header() {
    let existing = "Intro prose\n|---|---|\n| [0001](x.md) | y |\n";
    assert_eq!(find_boundary_offset(existing), existing.find("| [0001]"));
}

#[test]
fn a_row_of_empty_cells_is_not_a_separator_row() {
    let existing = "| A | B |\n| | |\n| [0001](x.md) | y |\n";
    assert_eq!(find_boundary_offset(existing), existing.find("| [0001]"));
}

#[test]
fn table_header_with_a_separator_and_no_row_stays_preamble() {
    let existing = "| ADR | Decision |\n|---|---|\n\n## Active\n";
    let heading = existing.find("## ").unwrap();
    assert_eq!(find_boundary_offset(existing), Some(heading));
    assert_eq!(
        find_boundary_offset("| ADR | Decision |\n|---|---|\n"),
        None
    );
}

#[test]
fn dash_bullet_hand_list_is_replaced_not_duplicated() {
    let hand = "# Research\n\nIntro text.\n\n- [Foo](0001-foo.md) - hand summary\n";
    let (first, second) = migrate_twice("dash-bullet", "research", hand);
    assert!(
        first.starts_with("# Research\n\nIntro text.\n\n"),
        "{first}"
    );
    assert!(!first.contains("hand summary"), "{first}");
    assert_eq!(first.matches("0001-foo.md").count(), 1);
    assert_eq!(first, second);
}

#[test]
fn dash_bullet_with_a_dot_slash_destination_is_a_boundary() {
    let existing = "Intro.\n- [Foo](./0001-foo.md) - s\n";
    assert_eq!(find_boundary_offset(existing), Some(7));
    assert!(is_boundary_line("- [Foo](0001-foo.md) - s"));
}

#[test]
fn dash_bullet_to_a_non_record_destination_stays_preamble() {
    assert!(!is_boundary_line("- [Guide](../guide.md)"));
    assert!(!is_boundary_line("- [Guide](guide.md)"));
    assert!(!is_boundary_line("- [Notes](2024notes.md)"));
    assert!(!is_boundary_line("- [Adr](../adr/0001-x.md)"));
    let existing = "# R\n\n- [Guide](../guide.md)\n\n* [0001 — Foo](0001-foo.md) - Draft\n";
    let offset = find_boundary_offset(existing).unwrap();
    assert!(existing[..offset].contains("../guide.md"));
    assert!(is_boundary_line("* [anything](x.md)"));
}
