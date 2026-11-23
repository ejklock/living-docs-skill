use super::*;

fn doc_with_body_lines(doc_type: &str, body_lines: usize) -> String {
    doc_with_status_and_body_lines(doc_type, None, body_lines)
}

fn doc_with_status_and_body_lines(
    doc_type: &str,
    status: Option<&str>,
    body_lines: usize,
) -> String {
    let body = (0..body_lines)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let status_line = status.map_or(String::new(), |status| format!("status: {status}\n"));
    format!("---\ntype: {doc_type}\n{status_line}---\n{body}")
}

#[test]
fn a_deprecated_adr_over_target_is_not_flagged() {
    assert_eq!(
        over_target_body_lines(&doc_with_status_and_body_lines(
            "ADR",
            Some("Deprecated"),
            121
        )),
        None
    );
}

#[test]
fn a_superseded_adr_over_target_is_not_flagged() {
    assert_eq!(
        over_target_body_lines(&doc_with_status_and_body_lines(
            "ADR",
            Some("Superseded"),
            121
        )),
        None
    );
}

#[test]
fn an_accepted_adr_over_target_is_still_flagged() {
    assert_eq!(
        over_target_body_lines(&doc_with_status_and_body_lines(
            "ADR",
            Some("Accepted"),
            121
        )),
        Some(121)
    );
}

#[test]
fn a_case_insensitive_deprecated_status_is_not_flagged() {
    assert_eq!(
        over_target_body_lines(&doc_with_status_and_body_lines(
            "ADR",
            Some("deprecated"),
            121
        )),
        None
    );
}

#[test]
fn body_line_count_excludes_the_frontmatter_block() {
    assert_eq!(body_line_count("---\ntype: ADR\n---\none\ntwo\n"), 2);
}

#[test]
fn body_line_count_without_frontmatter_counts_every_line() {
    assert_eq!(body_line_count("one\ntwo\nthree\n"), 3);
}

#[test]
fn a_body_at_exactly_the_warn_threshold_is_not_flagged() {
    assert_eq!(
        over_target_body_lines(&doc_with_body_lines("ADR", 120)),
        None
    );
}

#[test]
fn a_body_one_line_over_the_warn_threshold_is_flagged_with_its_count() {
    assert_eq!(
        over_target_body_lines(&doc_with_body_lines("ADR", 121)),
        Some(121)
    );
}

#[test]
fn research_is_exempt_regardless_of_length() {
    assert_eq!(
        over_target_body_lines(&doc_with_body_lines("Research", 400)),
        None
    );
}

#[test]
fn a_type_absent_from_the_registry_is_exempt_regardless_of_length() {
    assert!(
        doc_type::spec_for_frontmatter("Glossary").is_none(),
        "fixture premise broken: `Glossary` is now a registered frontmatter value — pick another unregistered type",
    );
    assert_eq!(
        over_target_body_lines(&doc_with_body_lines("Glossary", 400)),
        None
    );
}

/// Proves `check::size` reads `doc_type::DOC_TYPES` rather than a hardcoded
/// list of which types get the size target — it does not, and cannot, prove
/// any single row's `body_size` verdict is *correct*. That is held by the
/// pinned tests beside it: `a_body_one_line_over_...` pins ADR = Targeted,
/// `research_is_exempt_regardless_of_length` pins Research = Exempt.
#[test]
fn every_registry_row_is_flagged_exactly_when_its_body_size_is_targeted() {
    let mut saw_targeted = false;
    let mut saw_exempt = false;
    for spec in doc_type::DOC_TYPES {
        let over_target = over_target_body_lines(&doc_with_body_lines(spec.frontmatter, 121));
        match spec.body_size {
            BodySize::Targeted => {
                saw_targeted = true;
                assert_eq!(
                    over_target,
                    Some(121),
                    "{} is Targeted so it should carry the size target",
                    spec.frontmatter
                );
            }
            BodySize::Exempt => {
                saw_exempt = true;
                assert_eq!(
                    over_target, None,
                    "{} is Exempt so it should not carry the size target",
                    spec.frontmatter
                );
            }
        }
    }
    assert!(saw_targeted, "no Targeted row was exercised");
    assert!(saw_exempt, "no Exempt row was exercised");
}
