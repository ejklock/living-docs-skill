use super::*;

/// ADR 0026 fitness function A: every row's template must actually carry
/// that row's frontmatter type, and `spec_for` must resolve each token
/// back to the exact same static row — so a row added with a mismatched
/// template, or one that fails to round-trip, fails to compile-time
/// agreement here rather than surfacing as a runtime panic.
#[test]
fn fitness_function_a_every_spec_matches_its_template_and_round_trips() {
    for spec in DOC_TYPES {
        assert!(
            !spec.template.is_empty(),
            "{} has an empty template",
            spec.token
        );

        let type_line = spec
            .template
            .lines()
            .find(|line| line.starts_with("type:"))
            .unwrap_or_else(|| panic!("{} template has no 'type:' frontmatter line", spec.token));
        assert_eq!(
            type_line,
            format!("type: {}", spec.frontmatter),
            "{} template's frontmatter type disagrees with its spec",
            spec.token
        );

        let resolved = spec_for(spec.token)
            .unwrap_or_else(|| panic!("{} did not round-trip through spec_for", spec.token));
        assert_eq!(
            resolved, spec,
            "{} did not round-trip to an identical spec",
            spec.token
        );
    }
}

#[test]
fn spec_for_returns_none_for_an_unknown_token() {
    assert!(spec_for("glossary").is_none());
    assert!(spec_for("").is_none());
}

/// ADR 0029: every numbered type carries its own settable status values,
/// in seed order; Constitution carries none — it is a singleton with no
/// `NNNN`, so `living-docs status <NNNN>` can never reach it.
#[test]
fn status_vocabulary_matches_adr_0029_per_type() {
    assert_eq!(
        spec_for("adr").unwrap().status_vocabulary,
        &["Proposed", "Accepted", "Deprecated"]
    );
    assert_eq!(
        spec_for("prd").unwrap().status_vocabulary,
        &["Draft", "Accepted", "Implemented", "Deprecated"]
    );
    assert_eq!(
        spec_for("issue").unwrap().status_vocabulary,
        &["open", "in-progress", "closed"]
    );
    assert_eq!(
        spec_for("research").unwrap().status_vocabulary,
        &["Draft", "Accepted"]
    );
    assert!(spec_for("constitution")
        .unwrap()
        .status_vocabulary
        .is_empty());
    assert!(spec_for("view").unwrap().status_vocabulary.is_empty());
}

#[test]
fn status_vocabulary_never_carries_superseded_for_any_type() {
    for spec in DOC_TYPES {
        assert!(
            !spec
                .status_vocabulary
                .iter()
                .any(|value| value.eq_ignore_ascii_case("superseded")),
            "{} must never list Superseded in status_vocabulary",
            spec.token
        );
    }
}

/// The row this slice adds: `constitution` now resolves, and it resolves
/// as a [`Identity::Singleton`] naming exactly `constitution.md` — the
/// row `commands::new` branches on to write the bundle's
/// single unnumbered record.
#[test]
fn spec_for_resolves_constitution_as_a_singleton_named_constitution_md() {
    let spec = spec_for("constitution").expect("constitution must be a registered token");
    assert_eq!(
        spec.identity,
        Identity::Singleton {
            file: "constitution.md"
        }
    );
}

#[test]
fn spec_for_dir_matches_the_plural_issues_directory() {
    assert_eq!(spec_for_dir("issues").map(|spec| spec.token), Some("issue"));
    assert_eq!(spec_for_dir("adr").map(|spec| spec.token), Some("adr"));
}

#[test]
fn spec_for_dir_returns_none_for_an_unknown_directory() {
    assert!(spec_for_dir("constitution").is_none());
    assert!(spec_for_dir("issue").is_none());
    assert!(spec_for_dir("").is_none());
}

#[test]
fn is_retired_matches_the_types_own_terminal_status_case_insensitively() {
    let adr = spec_for("adr").unwrap();
    assert!(adr.is_retired("Deprecated"));
    assert!(adr.is_retired("deprecated"));
    assert!(!adr.is_retired("Accepted"));

    let issue = spec_for("issue").unwrap();
    assert!(issue.is_retired("closed"));
    assert!(issue.is_retired("done"));
    assert!(issue.is_retired("CLOSED"));
    assert!(!issue.is_retired("open"));
}

#[test]
fn is_retired_treats_superseded_as_retired_for_every_type() {
    for spec in DOC_TYPES {
        assert!(spec.is_retired("Superseded"));
        assert!(spec.is_retired("superseded"));
    }
}

/// ADR 0027: `spec_for_frontmatter` resolves the first row whose
/// `frontmatter` matches, so a duplicate would make that resolution
/// non-deterministic. This guards the invariant, not a literal list.
#[test]
fn frontmatter_values_are_unique_so_spec_for_frontmatter_is_well_defined() {
    let unique: std::collections::HashSet<&str> =
        DOC_TYPES.iter().map(|spec| spec.frontmatter).collect();
    assert_eq!(
        unique.len(),
        DOC_TYPES.len(),
        "DOC_TYPES has duplicate frontmatter values"
    );
}

fn required_sections_absent_from<'a>(spec: &'a DocTypeSpec, template: &str) -> Vec<&'a str> {
    let headings = crate::sections::heading_names(template);
    spec.sections
        .iter()
        .filter(|section| section.requirement == Requirement::Required)
        .filter(|section| !crate::sections::has_section(&headings, section.name))
        .map(|section| section.name)
        .collect()
}

#[test]
fn the_issue_row_declares_its_sections_in_order_with_requirement_and_tier() {
    let declared: Vec<_> = spec_for("issue")
        .unwrap()
        .sections
        .iter()
        .map(|section| (section.name, section.requirement, section.tier))
        .collect();
    assert_eq!(
        declared,
        [
            ("Scope", Requirement::Required, Tier::Contract),
            ("Decision", Requirement::Optional, Tier::Contract),
            ("Acceptance", Requirement::Required, Tier::Contract),
            ("Plan", Requirement::Optional, Tier::Detail),
            ("Outcome", Requirement::Optional, Tier::Detail),
        ]
    );
}

#[test]
fn a_type_without_a_schema_declares_no_sections() {
    assert!(spec_for("view").unwrap().sections.is_empty());
}

fn declared_by(token: &str) -> Vec<(&'static str, Requirement, Tier)> {
    spec_for(token)
        .unwrap()
        .sections
        .iter()
        .map(|section| (section.name, section.requirement, section.tier))
        .collect()
}

#[test]
fn the_adr_row_declares_its_sections() {
    assert_eq!(
        declared_by("adr"),
        [
            ("Context", Requirement::Required, Tier::Contract),
            ("Decision", Requirement::Required, Tier::Contract),
            ("Consequences", Requirement::Required, Tier::Contract),
            ("Verification", Requirement::Optional, Tier::Contract),
            ("References", Requirement::Optional, Tier::Detail),
        ]
    );
}

#[test]
fn the_prd_row_declares_its_sections() {
    assert_eq!(
        declared_by("prd"),
        [
            (
                "Problem / Motivation",
                Requirement::Required,
                Tier::Contract
            ),
            ("Goals", Requirement::Optional, Tier::Contract),
            ("Non-goals", Requirement::Required, Tier::Contract),
            ("Requirements", Requirement::Optional, Tier::Contract),
            ("Acceptance criteria", Requirement::Optional, Tier::Contract),
            ("Success metrics", Requirement::Optional, Tier::Contract),
            ("Open questions", Requirement::Optional, Tier::Detail),
            ("Related", Requirement::Optional, Tier::Detail),
        ]
    );
}

#[test]
fn the_constitution_row_declares_its_sections() {
    assert_eq!(
        declared_by("constitution"),
        [
            ("Product", Requirement::Required, Tier::Contract),
            ("Scope Boundaries", Requirement::Required, Tier::Contract),
            (
                "Data Model / Schema Foundation",
                Requirement::Optional,
                Tier::Contract
            ),
            ("Non-negotiables", Requirement::Required, Tier::Contract),
            ("Amendment Log", Requirement::Optional, Tier::Detail),
        ]
    );
}

#[test]
fn the_research_row_requires_nothing() {
    assert_eq!(
        declared_by("research"),
        [
            ("Question", Requirement::Optional, Tier::Contract),
            ("Method", Requirement::Optional, Tier::Detail),
            ("Findings", Requirement::Optional, Tier::Contract),
            ("Implications", Requirement::Optional, Tier::Contract),
            ("Open Questions", Requirement::Optional, Tier::Detail),
            ("References", Requirement::Optional, Tier::Detail),
        ]
    );
}

#[test]
fn every_required_section_is_a_heading_in_its_rows_template() {
    for spec in DOC_TYPES {
        assert_eq!(
            required_sections_absent_from(spec, spec.template),
            Vec::<&str>::new(),
            "{} template lacks a required section",
            spec.token
        );
    }
}

#[test]
fn the_template_fitness_check_reports_a_renamed_required_heading() {
    let spec = spec_for("issue").unwrap();
    let renamed = spec
        .template
        .replace("### Acceptance", "### Acceptance criteria");
    assert_eq!(
        required_sections_absent_from(spec, &renamed),
        ["Acceptance"]
    );
}
