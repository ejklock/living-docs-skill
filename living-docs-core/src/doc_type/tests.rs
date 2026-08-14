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
        spec_for("bdr").unwrap().status_vocabulary,
        &["Draft", "Accepted", "Implemented"]
    );
    assert_eq!(
        spec_for("prd").unwrap().status_vocabulary,
        &["Draft", "Accepted", "Implemented"]
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
/// row `commands::new`/`commands::brief` branch on to write the bundle's
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
