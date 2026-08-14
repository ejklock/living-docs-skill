use crate::commands::new::{
    fill_frontmatter, fill_frontmatter_description, fill_frontmatter_title,
};
use crate::record::format_scalar;

#[test]
fn fill_frontmatter_sets_type_status_and_timestamp() {
    let template = "---\ntype: ADR\nstatus: Proposed            # Proposed | Accepted\ntimestamp: <ISO 8601 datetime>\n---\n\n# Body\n<placeholder>\n";
    let filled = fill_frontmatter(template, "ADR", "2026-07-14T00:00:00Z");

    assert!(filled.contains("type: ADR"));
    assert!(filled.contains("status: Proposed"));
    assert!(filled.contains("timestamp: 2026-07-14T00:00:00Z"));
}

#[test]
fn fill_frontmatter_preserves_the_guidance_comment_verbatim() {
    let template = "---\ntype: ADR\nstatus: Proposed            # Proposed | Accepted | Superseded | Deprecated\ntimestamp: <ISO 8601 datetime>\n---\n\n# Body\n";
    let filled = fill_frontmatter(template, "ADR", "2026-07-14T00:00:00Z");

    assert!(filled.contains("# Proposed | Accepted | Superseded | Deprecated"));
}

#[test]
fn fill_frontmatter_leaves_the_body_untouched() {
    let template = "---\ntype: BDR\nstatus: Draft               # Draft | Accepted\ntimestamp: <ISO 8601 datetime>\n---\n\n<!-- Status lives in frontmatter (`status`), not a body line. -->\n<Replace the diagram above with a flowchart...>\n";
    let filled = fill_frontmatter(template, "BDR", "2026-07-14T00:00:00Z");

    assert!(filled.contains("<!-- Status lives in frontmatter (`status`), not a body line. -->"));
    assert!(filled.contains("<Replace the diagram above with a flowchart...>"));
}

#[test]
fn fill_frontmatter_without_a_closing_fence_returns_the_template_unchanged() {
    let template = "no frontmatter here\n";
    assert_eq!(
        fill_frontmatter(template, "ADR", "2026-07-14T00:00:00Z"),
        template
    );
}

/// ADR 0029 AC4: the seeded `status:` comes from each type's own
/// `status_vocabulary[0]`, not a hardcoded literal — issue seeds `open`,
/// bdr/prd/research seed `Draft`, adr still seeds `Proposed`.
#[test]
fn fill_frontmatter_seeds_each_types_own_first_vocabulary_value() {
    let template =
        "---\ntype: <TYPE>\nstatus: <STATUS>\ntimestamp: <ISO 8601 datetime>\n---\n\n# Body\n";

    let cases = [
        ("ADR", "Proposed"),
        ("BDR", "Draft"),
        ("PRD", "Draft"),
        ("Issue", "open"),
        ("Research", "Draft"),
    ];

    for (frontmatter_type, expected_status) in cases {
        let filled = fill_frontmatter(template, frontmatter_type, "2026-07-14T00:00:00Z");
        assert!(
            filled.contains(&format!("status: {expected_status}")),
            "{frontmatter_type} expected status: {expected_status}, got: {filled}"
        );
    }
}

#[test]
fn fill_frontmatter_falls_back_to_proposed_for_an_unresolvable_type() {
    let template = "---\ntype: Glossary\nstatus: <STATUS>\n---\n\n# Body\n";
    let filled = fill_frontmatter(template, "Glossary", "2026-07-14T00:00:00Z");

    assert!(filled.contains("status: Proposed"), "got: {filled}");
}

#[test]
fn fill_frontmatter_title_replaces_the_placeholder_with_the_argument() {
    let template =
        "---\ntype: ADR\ntitle: <Short decision title>\nstatus: Proposed\n---\n\n# Body\n";
    let filled = fill_frontmatter_title(template, "My Decision");

    assert!(filled.contains("title: My Decision\n"));
    assert!(!filled.contains("<Short decision title>"));
}

#[test]
fn fill_frontmatter_title_quotes_exactly_as_the_canonical_serializer_would() {
    let template =
        "---\ntype: ADR\ntitle: <Short decision title>\nstatus: Proposed\n---\n\n# Body\n";
    let filled = fill_frontmatter_title(template, "Caching: A Deep Dive");

    assert!(filled.contains(&format!(
        "title: {}\n",
        format_scalar("Caching: A Deep Dive")
    )));
}

#[test]
fn fill_frontmatter_title_leaves_the_body_untouched() {
    let template =
        "---\ntype: Issue\ntitle: <Issue title>\n---\n\n## <Issue title>\n\n<intro guidance>\n";
    let filled = fill_frontmatter_title(template, "Fix It");

    assert!(filled.contains("## <Issue title>"));
    assert!(filled.contains("<intro guidance>"));
}

#[test]
fn fill_frontmatter_title_without_a_closing_fence_returns_the_content_unchanged() {
    let content = "no frontmatter here\n";
    assert_eq!(fill_frontmatter_title(content, "My Decision"), content);
}

#[test]
fn fill_frontmatter_description_replaces_the_placeholder_with_the_argument() {
    let template = "---\ntype: ADR\ndescription: <One sentence — the decision and its scope.>\nstatus: Proposed\n---\n\n# Body\n";
    let filled = fill_frontmatter_description(template, Some("A concise decision description."));

    assert!(filled.contains("description: A concise decision description.\n"));
    assert!(!filled.contains("<One sentence"));
}

#[test]
fn fill_frontmatter_description_quotes_exactly_as_the_canonical_serializer_would() {
    let template = "---\ntype: ADR\ndescription: <One sentence — the decision and its scope.>\nstatus: Proposed\n---\n\n# Body\n";
    let filled = fill_frontmatter_description(template, Some("Caching: A Deep Dive"));

    assert!(filled.contains(&format!(
        "description: {}\n",
        format_scalar("Caching: A Deep Dive")
    )));
}

#[test]
fn fill_frontmatter_description_leaves_the_body_untouched() {
    let template = "---\ntype: Issue\ndescription: <One sentence>\n---\n\n## <Issue title>\n\n<intro guidance>\n";
    let filled = fill_frontmatter_description(template, Some("Fix it"));

    assert!(filled.contains("## <Issue title>"));
    assert!(filled.contains("<intro guidance>"));
}

#[test]
fn fill_frontmatter_description_without_a_closing_fence_returns_the_content_unchanged() {
    let content = "no frontmatter here\n";
    assert_eq!(
        fill_frontmatter_description(content, Some("A description")),
        content
    );
}

#[test]
fn fill_frontmatter_description_is_a_no_op_when_none_is_given() {
    let template =
        "---\ntype: ADR\ndescription: <One sentence — the decision and its scope.>\n---\n\n# Body\n";
    assert_eq!(fill_frontmatter_description(template, None), template);
}

#[test]
fn fill_frontmatter_description_inserts_the_line_when_the_frontmatter_lacks_it() {
    let template = "---\ntype: ADR\nstatus: Proposed\n---\n\n# Body\n";
    let filled = fill_frontmatter_description(template, Some("A concise sentence."));

    assert!(
        filled.contains("description: A concise sentence.\n"),
        "got: {filled}"
    );
    assert!(filled.contains("type: ADR\n"));
    assert!(filled.contains("status: Proposed\n"));
    assert!(filled.contains("# Body\n"));
}

mod kind {
    use crate::commands::new::fill_frontmatter_kind;
    use crate::doc_type;

    const VIEW_CONTENT: &str =
        "---\ntype: Architecture View\ntitle: X\nkind: component\ntimestamp: t\n---\n\nBody.\n";

    fn view_spec() -> &'static doc_type::DocTypeSpec {
        doc_type::spec_for("view").expect("view must be a registered token")
    }

    #[test]
    fn fills_a_valid_kind_over_the_template_seed() {
        let filled = fill_frontmatter_kind(VIEW_CONTENT, view_spec(), Some("context"))
            .expect("a registry kind must fill");
        assert!(filled.contains("kind: context\n"));
        assert!(!filled.contains("kind: component"));
    }

    #[test]
    fn none_keeps_the_template_seed_untouched() {
        let filled = fill_frontmatter_kind(VIEW_CONTENT, view_spec(), None)
            .expect("no kind must be a no-op");
        assert_eq!(filled, VIEW_CONTENT);
    }

    #[test]
    fn every_registry_kind_is_accepted() {
        for kind in doc_type::VIEW_KIND_ORDER {
            assert!(fill_frontmatter_kind(VIEW_CONTENT, view_spec(), Some(kind)).is_ok());
        }
    }

    #[test]
    fn an_unknown_kind_is_refused_naming_the_vocabulary() {
        let err = fill_frontmatter_kind(VIEW_CONTENT, view_spec(), Some("mystery"))
            .expect_err("an unlisted kind must be refused");
        assert!(err.contains("mystery"));
        assert!(err.contains("context"));
        assert!(err.contains("deployment"));
    }

    #[test]
    fn a_kind_on_a_numbered_type_is_refused() {
        let adr = doc_type::spec_for("adr").expect("adr must be a registered token");
        let err = fill_frontmatter_kind("---\ntype: ADR\n---\n\nBody.\n", adr, Some("context"))
            .expect_err("--kind on a numbered type must be refused");
        assert!(err.contains("adr"));
    }
}
