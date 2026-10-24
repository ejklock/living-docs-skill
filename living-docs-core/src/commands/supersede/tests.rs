use super::*;

#[test]
fn apply_frontmatter_field_fills_an_existing_empty_key_line() {
    let contents = "---\ntype: ADR\nsupersedes:\nsuperseded_by:\n---\n\n# Body\n";
    let updated = apply_frontmatter_field(contents, "superseded_by", "0002").unwrap();
    assert!(updated.contains("superseded_by: 0002"));
    assert!(updated.contains("supersedes:\n"));
}

#[test]
fn apply_frontmatter_field_preserves_a_trailing_guidance_comment() {
    let contents =
        "---\nsupersedes:                 # NNNN of the ADR this replaces, if any\n---\n\n# Body\n";
    let updated = apply_frontmatter_field(contents, "supersedes", "0001").unwrap();
    assert!(updated.contains("supersedes: 0001 # NNNN of the ADR this replaces, if any"));
}

#[test]
fn apply_frontmatter_field_inserts_an_absent_key_before_the_closing_fence() {
    let contents = "---\ntype: BDR\nsuperseded_by:\n---\n\n# Body\n";
    let updated = apply_frontmatter_field(contents, "supersedes", "0001").unwrap();
    assert!(updated.contains("supersedes: 0001"));
    assert!(updated.contains("---\ntype: BDR\nsuperseded_by:\nsupersedes: 0001\n---"));
}

#[test]
fn apply_frontmatter_field_leaves_the_body_untouched() {
    let contents = "---\ntype: ADR\nsupersedes:\n---\n\n## Context\n\nSome body text.\n";
    let updated = apply_frontmatter_field(contents, "supersedes", "0001").unwrap();
    assert!(updated.contains("## Context\n\nSome body text.\n"));
}

#[test]
fn apply_frontmatter_field_without_a_frontmatter_block_returns_none() {
    assert_eq!(
        apply_frontmatter_field("no frontmatter here\n", "supersedes", "0001"),
        None
    );
}

#[test]
fn parse_record_number_rejects_non_numeric_input() {
    assert!(parse_record_number("abcd").is_err());
}

#[test]
fn parse_record_reference_accepts_a_bare_number_with_no_qualifier() {
    let (dir, number) = parse_record_reference("0028").expect("bare number must parse");

    assert_eq!(dir, None);
    assert_eq!(number, 28);
}

#[test]
fn parse_record_reference_resolves_a_type_qualifier_through_the_registry() {
    let (dir, number) = parse_record_reference("issue/0028").expect("qualifier must resolve");

    assert_eq!(dir, Some("issues"));
    assert_eq!(number, 28);
}

#[test]
fn parse_record_reference_rejects_an_unknown_type_qualifier() {
    let err = parse_record_reference("glossary/0028")
        .expect_err("an unregistered token must be rejected");

    assert!(err.contains("glossary"), "got: {err}");
    assert!(err.contains("issue"), "got: {err}");
}
mod read_back {
    use super::*;
    use crate::test_support::WritableMapStore as MapStore;

    const PATH: &str = "/bundle/adr/0001-a.md";
    const RECORD: &str = "---\ntype: ADR\ntitle: A\ndescription: d\nstatus: Proposed\n---\n\n# A\n";

    fn write(
        store: &MapStore,
        fields: &[(&str, String)],
        expected: &[(&str, TailValue)],
    ) -> Result<(), String> {
        set_frontmatter_fields_expecting(store, Path::new(PATH), fields, expected)
    }

    #[test]
    fn a_folded_description_is_refused_and_the_record_stays_as_it_was() {
        let folded = RECORD.replace("description: d", "description: >\n  one\n  two");
        let store = MapStore::seeded(&[(PATH, folded.as_str())]);
        let err = write(&store, &[("description", "x".to_string())], &[]).expect_err("folded");
        assert!(
            err.contains("description") && err.contains(PATH),
            "got: {err}"
        );
        assert_eq!(store.contents(PATH), folded);
    }

    #[test]
    fn a_value_the_parser_stores_differently_than_given_is_refused() {
        let store = MapStore::seeded(&[(PATH, RECORD)]);
        let given = TailValue::Scalar("0x10".to_string());
        let err = write(
            &store,
            &[("tracker", "0x10".to_string())],
            &[("tracker", given)],
        )
        .expect_err("0x10 reads back as 16");
        assert!(err.contains("tracker"), "got: {err}");
        assert_eq!(store.contents(PATH), RECORD);
    }

    #[test]
    fn a_draft_that_changes_a_key_it_did_not_write_is_refused() {
        let store = MapStore::seeded(&[(PATH, RECORD)]);
        let err = write(&store, &[("owner", "[unclosed".to_string())], &[])
            .expect_err("an open flow list swallows the keys after it");
        assert!(err.contains("owner"), "got: {err}");
        assert_eq!(store.contents(PATH), RECORD);
    }

    #[test]
    fn a_draft_that_ends_the_frontmatter_early_changes_the_body_and_is_refused() {
        let store = MapStore::seeded(&[(PATH, RECORD)]);
        let err = write(&store, &[("tracker", "x\n---\ninjected".to_string())], &[])
            .expect_err("the closing fence moved into the value");
        assert!(err.contains("body"), "got: {err}");
        assert_eq!(store.contents(PATH), RECORD);
    }

    #[test]
    fn a_record_without_a_final_newline_gains_only_that_newline() {
        let bare = RECORD.trim_end_matches('\n');
        let store = MapStore::seeded(&[(PATH, bare)]);
        write(&store, &[("owner", "bob".to_string())], &[]).expect("newline appended");
        let written = store.contents(PATH);
        let body = bare.split_once("\n---\n").unwrap().1;
        assert!(written.contains("owner: bob\n"), "got: {written}");
        assert!(
            written.ends_with(&format!("\n---\n{body}\n")),
            "got: {written}"
        );
    }

    #[test]
    fn only_a_first_trailing_newline_is_tolerated_as_a_body_change() {
        assert!(!super::super::body_changed("text", "text\n"));
        assert!(super::super::body_changed("text\n", "text\n\n"));
        assert!(super::super::body_changed("text", "text\n\n"));
        assert!(!super::super::body_changed("text\n", "text\n"));
    }

    #[test]
    fn a_draft_that_adds_a_key_nobody_wrote_is_refused() {
        let store = MapStore::seeded(&[(PATH, RECORD)]);
        let err = write(&store, &[("owner", "x\nsneaky: y".to_string())], &[])
            .expect_err("a second key rides in on the value");
        assert!(err.contains("sneaky"), "got: {err}");
        assert_eq!(store.contents(PATH), RECORD);
    }

    #[test]
    fn a_value_that_reads_back_as_given_is_written() {
        let store = MapStore::seeded(&[(PATH, RECORD)]);
        let given = TailValue::Scalar("12".to_string());
        write(
            &store,
            &[("tracker", "12".to_string())],
            &[("tracker", given)],
        )
        .expect("12");
        assert!(store.contents(PATH).contains("tracker: 12\n"));
    }
}
