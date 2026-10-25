use super::super::*;
use super::{assert_round_trips, issue_record_with_list_valued_tail, numbered_record};

#[test]
fn to_canonical_markdown_emits_the_fixed_field_order() {
    let record = numbered_record();

    let markdown = to_canonical_markdown(&record);

    assert_eq!(
        markdown,
        "---\n\
         type: ADR\n\
         title: Tailed Decision\n\
         description: d.\n\
         owner: carol\n\
         status: Accepted\n\
         supersedes: 0001\n\
         tags: [caching, performance]\n\
         labels: important\n\
         blocked_by: 0002\n\
         tracker: JIRA-42\n\
         timestamp: 2026-07-17T00:00:00Z\n\
         ---\n\
         \n\
         # 0001. Tailed Decision\n\n\
         Body.\n"
    );
    assert!(!markdown.contains("number:"));
}

#[test]
fn to_canonical_markdown_round_trips_a_numbered_record_through_extract_record() {
    let record = numbered_record();

    let markdown = to_canonical_markdown(&record);
    let reparsed = extract_record(Path::new("adr/0001-tailed.md"), &markdown);

    assert_round_trips(&reparsed, &record);
}

#[test]
fn to_canonical_markdown_emits_a_sequence_tail_value_in_flow_style() {
    let record = issue_record_with_list_valued_tail();

    let markdown = to_canonical_markdown(&record);

    assert!(markdown.contains("labels: [slice, skeleton, refactor]"));
    assert!(markdown.contains("blocked_by: []"));
}

#[test]
fn to_canonical_markdown_round_trips_an_issue_record_with_list_valued_tail_keys() {
    let record = issue_record_with_list_valued_tail();

    let markdown = to_canonical_markdown(&record);
    let reparsed = extract_record(Path::new("issues/0006-findability-search.md"), &markdown);

    assert_round_trips(&reparsed, &record);
    assert_eq!(
        reparsed.frontmatter_tail, record.frontmatter_tail,
        "labels/blocked_by must round-trip as the same ordered list of scalars"
    );
}

#[test]
fn extract_record_reads_a_list_valued_tail_key_as_a_sequence() {
    let contents = "---\ntype: Issue\ntitle: T\ndescription: d.\nlabels: [slice, skeleton]\nblocked_by: []\n---\nBody.\n";
    let extracted = extract_record(Path::new("issues/0001-t.md"), contents);

    assert_eq!(
        extracted.frontmatter_tail,
        vec![
            (
                "labels".to_owned(),
                TailValue::Sequence(vec!["slice".to_owned(), "skeleton".to_owned()])
            ),
            ("blocked_by".to_owned(), TailValue::Sequence(Vec::new())),
        ]
    );
}

#[test]
fn to_canonical_markdown_round_trips_a_concept_record_through_extract_record() {
    let record = ExtractedRecord {
        doc_type: "Glossary".to_owned(),
        number: None,
        concept_id: Some("glossary/findability".to_owned()),
        identity_kind: CONCEPT_IDENTITY_KIND.to_owned(),
        title: "Findability".to_owned(),
        description: "The ease of locating a doc.".to_owned(),
        body: "# Findability\n\nBody.\n".to_owned(),
        supersedes: None,
        superseded_by: None,
        tags: vec!["glossary".to_owned()],
        status: Some("Active".to_owned()),
        owner: None,
        frontmatter_tail: Vec::new(),
    };

    let markdown = to_canonical_markdown(&record);
    let reparsed = extract_record(Path::new("glossary/findability.md"), &markdown);

    assert_round_trips(&reparsed, &record);
}

#[test]
fn to_canonical_markdown_omits_absent_optional_fields() {
    let record = ExtractedRecord {
        doc_type: "ADR".to_owned(),
        number: None,
        concept_id: None,
        identity_kind: NUMBER_IDENTITY_KIND.to_owned(),
        title: "No Extras".to_owned(),
        description: String::new(),
        body: "Body.\n".to_owned(),
        supersedes: None,
        superseded_by: None,
        tags: Vec::new(),
        status: None,
        owner: None,
        frontmatter_tail: Vec::new(),
    };

    let markdown = to_canonical_markdown(&record);

    assert_eq!(
        markdown,
        "---\ntype: ADR\ntitle: No Extras\ndescription: \"\"\n---\n\nBody.\n"
    );
    assert!(!markdown.contains("number:"));
    assert!(!markdown.contains("supersedes:"));
    assert!(!markdown.contains("tags:"));
}

#[test]
fn format_scalar_quotes_a_value_containing_a_colon_space_and_round_trips_it() {
    let record = ExtractedRecord {
        doc_type: "ADR".to_owned(),
        number: Some(2),
        concept_id: None,
        identity_kind: NUMBER_IDENTITY_KIND.to_owned(),
        title: "Caching: A Deep Dive".to_owned(),
        description: "d.".to_owned(),
        body: "Body.\n".to_owned(),
        supersedes: None,
        superseded_by: None,
        tags: Vec::new(),
        status: None,
        owner: None,
        frontmatter_tail: Vec::new(),
    };

    let markdown = to_canonical_markdown(&record);

    assert!(markdown.contains("title: \"Caching: A Deep Dive\""));
    let reparsed = extract_record(Path::new("adr/0002-caching.md"), &markdown);
    assert_eq!(reparsed.title, "Caching: A Deep Dive");
}

#[test]
fn format_scalar_quotes_a_comment_opener_and_leaves_a_bare_hash_alone() {
    for value in ["a #b", "a\t#b", "x ##"] {
        assert_eq!(format_scalar(value), format!("\"{value}\""));
    }
    assert_eq!(format_scalar("a#b"), "a#b");
}

#[test]
fn format_scalar_quotes_a_value_whose_plain_form_reads_back_as_another_string() {
    for value in ["1.50", "~", "null", "0x10", "1e3", "Null"] {
        assert_eq!(format_scalar(value), format!("\"{value}\""), "{value}");
    }
}

#[test]
fn format_scalar_keeps_a_value_plain_when_its_plain_form_reads_back_the_same() {
    for value in ["3", "true", "1.5", "a, b", "a#b", "Accepted"] {
        assert_eq!(format_scalar(value), value, "{value}");
    }
}

#[test]
fn format_scalar_quotes_an_inner_line_break_and_escapes_it() {
    assert_eq!(format_scalar("first\nsecond"), "\"first\\nsecond\"");
}

#[test]
fn format_flow_list_keeps_its_items_plain_when_no_character_rule_applies() {
    assert_eq!(format_flow_list(&["1.50", "a b"]), "[1.50, a b]");
}

#[test]
fn a_value_with_a_comment_opener_round_trips_whole_as_title_and_list_item() {
    let mut record = issue_record_with_list_valued_tail();
    record.title = "Fix a #b".to_owned();
    record.frontmatter_tail = vec![(
        "labels".to_owned(),
        TailValue::Sequence(vec!["a #b".to_owned(), "c".to_owned()]),
    )];

    let markdown = to_canonical_markdown(&record);
    let reparsed = extract_record(Path::new("issues/0006-findability-search.md"), &markdown);

    assert_eq!(reparsed.title, "Fix a #b");
    assert_eq!(reparsed.frontmatter_tail, record.frontmatter_tail);
}

/// Asserts `status` is read from [`ExtractedRecord::status`] rather than
/// the frontmatter tail: the typed field must still round-trip to a
/// `status:` frontmatter line even though `frontmatter_tail` never
/// carries a `status` entry.
#[test]
fn to_canonical_markdown_emits_status_from_the_typed_field() {
    let record = ExtractedRecord {
        doc_type: "ADR".to_owned(),
        number: Some(3),
        concept_id: None,
        identity_kind: NUMBER_IDENTITY_KIND.to_owned(),
        title: "Typed Status".to_owned(),
        description: "d.".to_owned(),
        body: "Body.\n".to_owned(),
        supersedes: None,
        superseded_by: None,
        tags: Vec::new(),
        status: Some("Accepted".to_owned()),
        owner: None,
        frontmatter_tail: Vec::new(),
    };

    let markdown = to_canonical_markdown(&record);

    assert!(markdown.contains("status: Accepted"));
    let reparsed = extract_record(Path::new("adr/0003-typed-status.md"), &markdown);
    assert_eq!(reparsed.status, Some("Accepted".to_owned()));
}
