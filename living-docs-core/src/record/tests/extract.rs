use super::super::*;

#[test]
fn extract_record_derives_number_from_the_filenames_nnnn_prefix() {
    let contents = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\n---\n# 0001. Quokka Caching\n\nBody text.\n";
    let extracted = extract_record(Path::new("adr/0001-quokka-caching.md"), contents);

    assert_eq!(extracted.doc_type, "ADR");
    assert_eq!(extracted.number, Some(1));
    assert_eq!(extracted.concept_id, None);
    assert_eq!(extracted.identity_kind, NUMBER_IDENTITY_KIND);
    assert_eq!(extracted.title, "Quokka Caching");
    assert_eq!(extracted.description, "Adopt quokka caching.");
    assert_eq!(extracted.body, "# 0001. Quokka Caching\n\nBody text.\n");
}

#[test]
fn extract_record_ignores_a_number_frontmatter_key_and_derives_from_the_path_instead() {
    let contents = "---\ntype: ADR\nnumber: 99\n---\nBody.\n";
    let extracted = extract_record(Path::new("adr/0007-x.md"), contents);

    assert_eq!(extracted.number, Some(7));
}

#[test]
fn extract_record_derives_concept_id_from_the_project_relative_path() {
    let contents = "---\ntype: Glossary\ntitle: Findability\n---\n# Findability\n\nBody.\n";
    let extracted = extract_record(Path::new("context/user-auth.md"), contents);

    assert_eq!(extracted.identity_kind, CONCEPT_IDENTITY_KIND);
    assert_eq!(extracted.concept_id, Some("context/user-auth".to_owned()));
    assert_eq!(extracted.number, None);
}

#[test]
fn extract_record_ignores_a_concept_id_frontmatter_key_and_derives_from_the_path_instead() {
    let contents = "---\ntype: Glossary\nconcept_id: wrong-slug\n---\nBody.\n";
    let extracted = extract_record(Path::new("glossary/findability.md"), contents);

    assert_eq!(
        extracted.concept_id,
        Some("glossary/findability".to_owned())
    );
}

#[test]
fn extract_record_assigns_the_number_identity_kind_to_every_numbered_doc_type() {
    for doc_type in ["ADR", "PRD", "Issue"] {
        let contents = format!("---\ntype: {doc_type}\n---\nBody.\n");
        let extracted = extract_record(Path::new("adr/0007-numbered.md"), &contents);

        assert_eq!(
            extracted.identity_kind, NUMBER_IDENTITY_KIND,
            "{doc_type} must classify as the number identity kind"
        );
        assert_eq!(extracted.number, Some(7));
        assert_eq!(extracted.concept_id, None);
    }
}

#[test]
fn extract_record_yields_no_number_when_the_filename_lacks_a_valid_nnnn_prefix() {
    let contents = "---\ntype: ADR\n---\nBody.\n";
    let extracted = extract_record(Path::new("adr/no-number-here.md"), contents);

    assert_eq!(extracted.identity_kind, NUMBER_IDENTITY_KIND);
    assert_eq!(extracted.number, None);
}

#[test]
fn extract_record_concept_doc_always_yields_a_concept_id_even_without_frontmatter() {
    let contents = "Body with no frontmatter block at all.\n";
    let extracted = extract_record(Path::new("glossary/findability.md"), contents);

    assert_eq!(extracted.identity_kind, CONCEPT_IDENTITY_KIND);
    assert_eq!(
        extracted.concept_id,
        Some("glossary/findability".to_owned())
    );
}

#[test]
fn extract_record_falls_back_to_first_heading_when_title_is_absent() {
    let contents = "---\ntype: ADR\n---\n# Fallback Heading\n\nBody.\n";
    let extracted = extract_record(Path::new("/bundle/adr/0002-fallback.md"), contents);

    assert_eq!(extracted.title, "Fallback Heading");
}

#[test]
fn extract_record_falls_back_to_filename_stem_when_no_title_or_heading() {
    let contents = "---\ntype: ADR\n---\nBody with no heading.\n";
    let extracted = extract_record(Path::new("/bundle/adr/0003-stemmed.md"), contents);

    assert_eq!(extracted.title, "0003-stemmed");
}

#[test]
fn extract_record_defaults_missing_description_to_empty() {
    let contents = "---\ntype: ADR\ntitle: No Extras\n---\nBody.\n";
    let extracted = extract_record(Path::new("adr/0004-no-extras.md"), contents);

    assert_eq!(extracted.description, "");
    assert_eq!(extracted.concept_id, None);
}

#[test]
fn extract_record_strips_the_frontmatter_block_from_the_body() {
    let contents = "---\ntype: ADR\ntitle: Stripped\n---\n# Stripped\n\nRemaining body.\n";
    let extracted = extract_record(Path::new("/bundle/adr/0005-stripped.md"), contents);

    assert!(!extracted.body.contains("---"));
    assert_eq!(extracted.body, "# Stripped\n\nRemaining body.\n");
}

#[test]
fn extract_record_without_frontmatter_treats_the_whole_file_as_body() {
    let contents = "# No Frontmatter\n\nJust a body.\n";
    let extracted = extract_record(Path::new("/bundle/log.md"), contents);

    assert_eq!(extracted.doc_type, "");
    assert_eq!(extracted.body, contents);
    assert_eq!(extracted.title, "No Frontmatter");
}

#[test]
fn extract_record_ignores_a_concept_id_stray_on_a_numbered_doc_type() {
    let contents = "---\ntype: Issue\nconcept_id: findability\n---\nBody.\n";
    let extracted = extract_record(Path::new("issues/0006-findability.md"), contents);

    assert_eq!(extracted.identity_kind, NUMBER_IDENTITY_KIND);
    assert_eq!(extracted.number, Some(6));
    assert_eq!(
        extracted.concept_id, None,
        "concept_id is not the identity field for a numbered doc type"
    );
}

#[test]
fn extract_record_tail_excludes_typed_and_relation_keys_in_source_order() {
    let contents = "---\ntype: ADR\ntitle: Tailed\ndescription: d.\nnumber: 1\nstatus: Accepted\nsupersedes:\nsuperseded_by:\ntags: [a]\ntracker: JIRA-1\ntimestamp: 2026-07-17T00:00:00Z\n---\nBody.\n";
    let extracted = extract_record(Path::new("/bundle/adr/0001-tailed.md"), contents);

    assert_eq!(
        extracted.frontmatter_tail,
        vec![
            ("tracker".to_owned(), TailValue::Scalar("JIRA-1".to_owned())),
            (
                "timestamp".to_owned(),
                TailValue::Scalar("2026-07-17T00:00:00Z".to_owned())
            ),
        ]
    );
    assert_eq!(extracted.status, Some("Accepted".to_owned()));
}

#[test]
fn extract_record_reads_status_when_present() {
    let contents = "---\ntype: ADR\nstatus: Accepted\n---\nBody.\n";
    let extracted = extract_record(Path::new("/bundle/adr/0001-status.md"), contents);

    assert_eq!(extracted.status, Some("Accepted".to_owned()));
}

#[test]
fn extract_record_defaults_missing_status_to_none() {
    let contents = "---\ntype: ADR\ntitle: No Status\n---\nBody.\n";
    let extracted = extract_record(Path::new("/bundle/adr/0002-no-status.md"), contents);

    assert_eq!(extracted.status, None);
}

#[test]
fn extract_record_tail_is_empty_when_only_typed_keys_are_present() {
    let contents = "---\ntype: ADR\ntitle: No Tail\ndescription: d.\nnumber: 1\n---\nBody.\n";
    let extracted = extract_record(Path::new("/bundle/adr/0002-no-tail.md"), contents);

    assert!(extracted.frontmatter_tail.is_empty());
}

#[test]
fn extract_record_reads_supersedes_superseded_by_and_tags() {
    let contents = "---\ntype: ADR\nsupersedes: 0001\nsuperseded_by: 0003\ntags: [caching, performance]\n---\nBody.\n";
    let extracted = extract_record(Path::new("/bundle/adr/0002-improved.md"), contents);

    assert_eq!(extracted.supersedes, Some(vec!["0001".to_owned()]));
    assert_eq!(extracted.superseded_by, Some("0003".to_owned()));
    assert_eq!(
        extracted.tags,
        vec!["caching".to_owned(), "performance".to_owned()]
    );
}

#[test]
fn extract_record_defaults_missing_supersede_fields_and_tags_to_empty() {
    let contents = "---\ntype: ADR\nsupersedes:\nsuperseded_by:\ntags: []\n---\nBody.\n";
    let extracted = extract_record(Path::new("/bundle/adr/0001-first.md"), contents);

    assert_eq!(extracted.supersedes, None);
    assert_eq!(extracted.superseded_by, None);
    assert!(extracted.tags.is_empty());
}

#[test]
fn extract_record_defaults_tags_to_empty_when_the_key_is_absent() {
    let contents = "---\ntype: ADR\n---\nBody.\n";
    let extracted = extract_record(Path::new("/bundle/adr/0004-no-tags.md"), contents);

    assert!(extracted.tags.is_empty());
}

#[test]
fn extract_record_keeps_the_typed_items_of_a_supersedes_list_that_is_not_all_record_numbers() {
    let null_item = vec![Value::String("0001".to_owned()), Value::Null];
    let empty_item = vec![
        Value::String("0001".to_owned()),
        Value::String(String::new()),
    ];
    for (source, items) in [
        ("supersedes: [0001, ~]", null_item.clone()),
        ("supersedes:\n  - 0001\n  - null", null_item),
        ("supersedes: [0001, \"\"] # note", empty_item),
    ] {
        let contents = format!("---\ntype: ADR\n{source}\ntitle: T\n---\nBody.\n");
        let extracted = extract_record(Path::new("/bundle/adr/0004-x.md"), &contents);

        assert_eq!(extracted.supersedes, None, "{source}");
        assert_eq!(
            extracted.frontmatter_tail,
            vec![("supersedes".to_owned(), TailValue::Typed(items))],
            "{source}"
        );
    }
}

#[test]
fn extract_record_keeps_a_supersedes_list_of_strings_that_is_not_all_record_numbers_as_a_sequence()
{
    let contents = "---\ntype: ADR\nsupersedes: [0001, \"x]\"]\n---\nBody.\n";
    let extracted = extract_record(Path::new("/bundle/adr/0004-x.md"), contents);

    assert_eq!(
        extracted.frontmatter_tail,
        vec![(
            "supersedes".to_owned(),
            TailValue::Sequence(vec!["0001".to_owned(), "x]".to_owned()])
        )]
    );
}
