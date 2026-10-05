use super::*;

fn names(contents: &str) -> Vec<String> {
    heading_names(contents)
}

#[test]
fn the_first_heading_is_the_title_and_is_not_a_section() {
    assert_eq!(
        names("---\ntype: Issue\n---\n\n## Title\n\n### Scope\n\n### Acceptance\n"),
        ["Scope", "Acceptance"]
    );
}

#[test]
fn a_record_without_frontmatter_is_read_from_its_first_line() {
    assert_eq!(names("# Title\n\n## Scope\n"), ["Scope"]);
}

#[test]
fn headings_inside_backtick_and_tilde_fences_are_ignored() {
    let body = "# T\n\n```\n## In backticks\n```\n\n~~~\n## In tildes\n~~~\n\n## Real\n";
    assert_eq!(names(body), ["Real"]);
}

#[test]
fn a_different_fence_marker_does_not_close_an_open_fence() {
    assert_eq!(
        names("# T\n\n```\n~~~\n## Hidden\n```\n## Real\n"),
        ["Real"]
    );
}

#[test]
fn text_that_is_not_a_heading_is_ignored() {
    assert_eq!(
        names("# T\n\n#hashtag\n####### seven\n## Real  \n"),
        ["Real"]
    );
}

#[test]
fn has_section_matches_the_whole_name_ignoring_case() {
    let found = vec!["acceptance".to_owned(), "Acceptance criteria".to_owned()];
    assert!(has_section(&found, "Acceptance"));
    assert!(!has_section(&found[1..], "Acceptance"));
}

const ISSUE_SCHEMA: &[SectionSpec] = &[
    SectionSpec {
        name: "Scope",
        requirement: crate::doc_type::Requirement::Required,
        tier: Tier::Contract,
    },
    SectionSpec {
        name: "Plan",
        requirement: crate::doc_type::Requirement::Optional,
        tier: Tier::Detail,
    },
];

#[test]
fn contract_body_drops_a_detail_section_with_its_subheadings_only() {
    let body = "## T\n\npre\n\n### Plan\n\np\n\n#### Sub\n\ns\n\n### Scope\n\nkept\n";
    assert_eq!(
        contract_body(body, ISSUE_SCHEMA),
        "## T\n\npre\n\n### Scope\n\nkept\n"
    );
}

#[test]
fn contract_body_keeps_the_first_section_of_an_untitled_body() {
    assert_eq!(
        contract_body("### Scope\n\nfirst\n", ISSUE_SCHEMA),
        "### Scope\n\nfirst\n"
    );
}

#[test]
fn contract_body_with_an_empty_schema_is_unchanged() {
    let body = "# T\n\n## Plan\n\nx";
    assert_eq!(contract_body(body, &[]), body);
}
