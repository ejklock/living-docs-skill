use super::*;
use crate::doc_type;

const TEMPLATE: &str = "---\ntype: ADR\ntitle: T\n---\n\n# NNNN. Placeholder\n\n<!-- status guidance -->\n\n## Context\n\n<!-- context guidance -->\n\n{{CONTEXT}}\n\n## Decision\n\nWe will {{DECISION}}.\n\n# References\n\n[1] [{{SOURCE}}]({{URL}})\n";

#[test]
fn named_sections_are_filled_and_unnamed_ones_keep_their_guidance() {
    let filled = fill_sections(
        TEMPLATE,
        r#"{"Context": "The forces.", "Decision": "We will do X."}"#,
        "Choose X",
        Some(7),
    )
    .expect("valid payload must fill");

    assert!(filled.contains("## Context\n\nThe forces.\n"));
    assert!(filled.contains("## Decision\n\nWe will do X.\n"));
    assert!(!filled.contains("{{CONTEXT}}"));
    assert!(!filled.contains("{{DECISION}}"));
    assert!(filled.contains("[1] [{{SOURCE}}]({{URL}})"));
}

#[test]
fn the_title_heading_is_filled_with_the_number_when_the_template_carries_nnnn() {
    let filled = fill_sections(TEMPLATE, "{}", "Choose X", Some(7)).expect("empty payload fills");

    assert!(filled.contains("# 0007. Choose X\n"));
    assert!(!filled.contains("Placeholder"));
}

#[test]
fn a_titleless_number_or_unnumbered_template_fills_the_bare_title() {
    let template =
        "---\ntype: Issue\ntitle: T\n---\n\n## Old\n\n{{SUMMARY}}\n\n### Scope\n\n{{SCOPE}}\n";

    let filled =
        fill_sections(template, r#"{"Intro": "Summary."}"#, "Fix the gate", None).expect("fills");

    assert!(filled.contains("## Fix the gate\n\nSummary.\n"));
    assert!(filled.contains("### Scope\n"));
    assert!(filled.contains("{{SCOPE}}"));
}

#[test]
fn an_unknown_section_key_is_refused_listing_the_types_sections() {
    let err = fill_sections(TEMPLATE, r#"{"Mystery": "x"}"#, "T", Some(1))
        .expect_err("unknown key must refuse");

    assert!(err.contains("Mystery"));
    assert!(err.contains("Context"));
    assert!(err.contains("Decision"));
    assert!(err.contains("References"));
    assert!(err.contains("Intro"));
}

#[test]
fn a_non_object_payload_and_a_non_string_section_are_refused() {
    assert!(fill_sections(TEMPLATE, "[1,2]", "T", None)
        .expect_err("array must refuse")
        .contains("object"));
    assert!(fill_sections(TEMPLATE, r#"{"Context": 3}"#, "T", None)
        .expect_err("number must refuse")
        .contains("string"));
}

#[test]
fn every_registry_template_exposes_at_least_one_fillable_section() {
    for spec in doc_type::DOC_TYPES {
        let filled = fill_sections(spec.template, "{}", "Some Title", Some(1));
        assert!(
            filled.is_ok(),
            "{} template must survive an empty payload: {:?}",
            spec.token,
            filled.err()
        );
    }
}
