use super::*;

fn names(contents: &str) -> Vec<String> {
    heading_names(contents)
}

#[test]
fn the_first_heading_is_the_title_and_is_not_a_section() {
    assert_eq!(
        names("---\ntype: Issue\ntitle: Title\n---\n\n## Title\n\n### Scope\n\n### Acceptance\n"),
        ["Scope", "Acceptance"]
    );
}

#[test]
fn a_record_without_frontmatter_is_read_from_its_first_line() {
    assert_eq!(names("# Title\n\n## Scope\n"), ["Title", "Scope"]);
}

#[test]
fn headings_inside_backtick_and_tilde_fences_are_ignored() {
    let body = "---\ntitle: T\n---\n# T\n\n```\n## In backticks\n```\n\n~~~\n## In tildes\n~~~\n\n## Real\n";
    assert_eq!(names(body), ["Real"]);
}

#[test]
fn a_different_fence_marker_does_not_close_an_open_fence() {
    assert_eq!(
        names("---\ntitle: T\n---\n# T\n\n```\n~~~\n## Hidden\n```\n## Real\n"),
        ["Real"]
    );
}

#[test]
fn text_that_is_not_a_heading_is_ignored() {
    assert_eq!(
        names("---\ntitle: T\n---\n# T\n\n#hashtag\n####### seven\n## Real  \n"),
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

#[test]
fn an_untitled_body_keeps_its_first_heading_as_a_section() {
    assert_eq!(
        names("---\ntype: Issue\n---\n\n### Scope\n\n### Acceptance\n"),
        ["Scope", "Acceptance"]
    );
}

#[test]
fn a_first_heading_that_matches_the_title_is_dropped_with_its_number_prefix() {
    let record = "---\ntitle: My title\n---\n\n## 0001. My title\n\n### Scope\n";
    assert_eq!(names(record), ["Scope"]);
}

#[test]
fn a_first_heading_that_disagrees_with_the_title_is_a_section() {
    let record = "---\ntitle: My title\n---\n\n## Other\n\n### Scope\n";
    assert_eq!(names(record), ["Other", "Scope"]);
}

#[test]
fn a_first_heading_that_differs_from_the_title_only_by_case_is_a_section() {
    let record = "---\ntitle: my title\n---\n\n## My Title\n\n### Scope\n";
    assert_eq!(names(record), ["My Title", "Scope"]);
}

#[test]
fn four_spaces_or_a_tab_never_make_a_heading_and_three_spaces_do() {
    let body = "---\ntitle: T\n---\n# T\n\n    ## Four\n\t## Tab\n   ## Three\n";
    assert_eq!(names(body), ["Three"]);
}

#[test]
fn an_indented_detail_heading_does_not_cut_a_contract_section() {
    let body = "### Scope\n\nkept\n\n    ### Plan\n\nstill scope\n";
    assert_eq!(contract_body(body, ISSUE_SCHEMA), body);
}

const RECORD: &str = "---\ntitle: T\n---\n\n## 0001. T\n\n### Scope\n\ns\n\n### Outcome\n\nold\n\n#### Sub\n\nsub\n\n### Plan\n\np\n";

#[test]
fn replacing_a_section_keeps_its_heading_and_drops_its_subsections() {
    let updated = replace_or_append_section(RECORD, "outcome", "new\n");
    assert_eq!(
        updated,
        "---\ntitle: T\n---\n\n## 0001. T\n\n### Scope\n\ns\n\n### Outcome\n\nnew\n\n### Plan\n\np\n"
    );
}

#[test]
fn replacing_the_last_section_runs_to_the_end_of_the_record() {
    let updated = replace_or_append_section(RECORD, "Plan", "q");
    assert!(updated.ends_with("### Plan\n\nq\n"), "{updated}");
}

#[test]
fn an_absent_section_is_appended_at_the_level_most_sections_use() {
    let updated = replace_or_append_section(RECORD, "Notes", "n\n");
    assert!(updated.ends_with("p\n\n### Notes\n\nn\n"), "{updated}");
}

#[test]
fn a_record_with_no_sections_appends_at_level_two() {
    let updated = replace_or_append_section("---\ntitle: T\n---\n\n# T\n", "Notes", "n\n");
    assert!(updated.ends_with("# T\n\n## Notes\n\nn\n"), "{updated}");
}

#[test]
fn the_title_heading_is_never_the_section_replaced() {
    let updated = replace_or_append_section(RECORD, "T", "n\n");
    assert!(updated.contains("## 0001. T\n\n### Scope"), "{updated}");
    assert!(updated.ends_with("### T\n\nn\n"), "{updated}");
}

fn record_with(section: &str) -> String {
    format!("---\ntitle: T\n---\n\n## 0001. T\n\n### Scope\n\n{section}\n### Plan\n\np\n")
}

fn missing(section: &str, text: &str) -> Vec<String> {
    missing_children(&record_with(section), "Scope", text)
}

const NONE: [&str; 0] = [];

#[test]
fn a_child_absent_from_the_new_text_is_missing() {
    assert_eq!(
        missing("s\n\n#### Child A\n\nx\n", "new\n"),
        ["#### Child A"]
    );
}

#[test]
fn a_child_carried_at_the_same_level_and_text_in_any_case_is_not_missing() {
    assert_eq!(
        missing("s\n\n#### Child A\n\nx\n", "new\n\n#### child a\n\ny\n"),
        NONE
    );
}

#[test]
fn a_child_carried_at_another_level_is_missing() {
    assert_eq!(
        missing("s\n\n#### Child A\n\nx\n", "new\n\n##### Child A\n"),
        ["#### Child A"]
    );
}

#[test]
fn a_child_carried_with_other_text_is_missing() {
    assert_eq!(
        missing("s\n\n#### Child A\n\nx\n", "new\n\n#### Child B\n"),
        ["#### Child A"]
    );
}

#[test]
fn a_section_without_children_misses_none() {
    assert_eq!(missing("s\n\n", "new\n"), NONE);
}

#[test]
fn a_heading_like_line_in_a_fenced_block_of_the_section_is_not_a_child() {
    assert_eq!(missing("s\n\n```\n#### Fake\n```\n\n", "new\n"), NONE);
}

#[test]
fn a_heading_like_line_in_an_indented_block_of_the_section_is_not_a_child() {
    assert_eq!(missing("s\n\n    #### Fake\n\n", "new\n"), NONE);
}

#[test]
fn a_child_like_line_in_a_code_block_of_the_new_text_does_not_carry_it() {
    assert_eq!(
        missing(
            "s\n\n#### Child A\n\nx\n",
            "new\n\n```\n#### Child A\n```\n"
        ),
        ["#### Child A"]
    );
}

#[test]
fn only_the_child_the_new_text_does_not_carry_is_missing() {
    assert_eq!(
        missing(
            "s\n\n#### Child A\n\nx\n\n#### Child B\n\ny\n",
            "new\n\n#### Child B\n"
        ),
        ["#### Child A"]
    );
}

#[test]
fn a_grandchild_counts_as_a_child() {
    assert_eq!(
        missing(
            "s\n\n#### Child A\n\nx\n\n##### Grand\n\ny\n",
            "new\n\n#### Child A\n"
        ),
        ["##### Grand"]
    );
}

#[test]
fn a_child_of_another_section_is_not_a_child_of_this_one() {
    let contents = "---\ntitle: T\n---\n\n## 0001. T\n\n### Scope\n\ns\n\n### Plan\n\n#### Step\n";
    assert_eq!(missing_children(contents, "Scope", "n\n"), NONE);
}

#[test]
fn an_absent_section_misses_no_children() {
    assert_eq!(missing_children(RECORD, "Notes", "n\n"), NONE);
}

const TWINS: &str = "s\n\n#### A\n\n##### Example\n\nex-a\n\n#### B\n\n##### Example\n\nex-b\n";

#[test]
fn two_same_named_children_with_one_carried_copy_miss_one() {
    let text = "n\n\n#### A\n\n##### Example\n\nz\n\n#### B\n";
    assert_eq!(missing(TWINS, text), ["##### Example"]);
}

#[test]
fn two_same_named_children_with_two_carried_copies_miss_none() {
    let text = "n\n\n#### A\n\n##### Example\n\n#### B\n\n##### Example\n";
    assert_eq!(missing(TWINS, text), NONE);
}

#[test]
fn two_same_named_children_with_three_carried_copies_miss_none() {
    let text = "n\n\n##### Example\n\n#### A\n\n##### Example\n\n#### B\n\n##### Example\n";
    assert_eq!(missing(TWINS, text), NONE);
}

#[test]
fn a_carried_copy_in_another_case_consumes_one_of_two_same_named_children() {
    let text = "n\n\n#### A\n\n##### example\n\n#### B\n";
    assert_eq!(missing(TWINS, text), ["##### Example"]);
}

#[test]
fn a_carried_copy_at_one_level_does_not_consume_the_same_name_at_another_level() {
    let section = "s\n\n#### Example\n\nx\n\n##### Example\n\ny\n";
    assert_eq!(missing(section, "n\n\n#### Example\n"), ["##### Example"]);
}
