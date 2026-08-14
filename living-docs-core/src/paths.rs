use crate::doc_type::{self, Identity};

/// Maps a doc-type token from the CLI to its docs-tree subdirectory (relative
/// to `--docs-dir`). `issue` is deliberately plural (`issues`) — everything
/// else matches the token. `None` for both an unknown token and a singleton
/// type, which has no directory (ADR 0026).
pub fn dir_for(token: &str) -> Option<&'static str> {
    match doc_type::spec_for(token)?.identity {
        Identity::Numbered { dir } => Some(dir),
        Identity::Singleton { .. } | Identity::Named { .. } => None,
    }
}

/// Maps a docs-tree subdirectory name back to its doc-type token — the
/// exact reverse of [`dir_for`].
pub fn doc_type_for_dir(dir_name: &str) -> Option<&'static str> {
    Some(doc_type::spec_for_dir(dir_name)?.token)
}

/// Maps a doc-type token to the canonical value written to the frontmatter
/// `type` field.
pub fn frontmatter_type_for(token: &str) -> Option<&'static str> {
    Some(doc_type::spec_for(token)?.frontmatter)
}

/// Lowercase kebab-case slug: keeps ASCII alphanumerics, collapses any run of
/// other characters into a single `-`, and drops leading/trailing separators.
pub fn slugify(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    let mut pending_hyphen = false;

    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_hyphen && !slug.is_empty() {
                slug.push('-');
            }
            pending_hyphen = false;
            slug.push(ch.to_ascii_lowercase());
        } else {
            pending_hyphen = true;
        }
    }

    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_for_maps_issue_to_the_plural_directory() {
        assert_eq!(dir_for("issue"), Some("issues"));
        assert_eq!(dir_for("adr"), Some("adr"));
        assert_eq!(dir_for("bdr"), Some("bdr"));
        assert_eq!(dir_for("prd"), Some("prd"));
    }

    #[test]
    fn dir_for_returns_none_for_an_unknown_token() {
        assert_eq!(dir_for("glossary"), None);
        assert_eq!(dir_for(""), None);
    }

    /// A singleton token is known to the registry but has no directory — a
    /// different reason for `None` than an unknown token (ADR 0026
    /// Consequences: "`dir_for` becoming fallible for a reason other than
    /// unknown type"). The token is read off the registry's first
    /// [`Identity::Singleton`] row rather than hardcoded, so a renamed row
    /// fails this test instead of silently passing.
    #[test]
    fn dir_for_returns_none_for_a_known_singleton_type() {
        let singleton_token = doc_type::DOC_TYPES
            .iter()
            .find(|spec| matches!(spec.identity, Identity::Singleton { .. }))
            .expect("registry must carry at least one singleton row")
            .token;

        assert!(doc_type::spec_for(singleton_token).is_some());
        assert_eq!(dir_for(singleton_token), None);
    }

    #[test]
    fn doc_type_for_dir_maps_the_plural_issues_directory_to_issue() {
        assert_eq!(doc_type_for_dir("issues"), Some("issue"));
        assert_eq!(doc_type_for_dir("adr"), Some("adr"));
        assert_eq!(doc_type_for_dir("bdr"), Some("bdr"));
        assert_eq!(doc_type_for_dir("prd"), Some("prd"));
    }

    #[test]
    fn doc_type_for_dir_rejects_unknown_directories() {
        assert_eq!(doc_type_for_dir("constitution"), None);
        assert_eq!(doc_type_for_dir("issue"), None);
        assert_eq!(doc_type_for_dir(""), None);
    }

    #[test]
    fn doc_type_for_dir_is_the_exact_reverse_of_dir_for() {
        let numbered_tokens = doc_type::DOC_TYPES
            .iter()
            .filter(|spec| matches!(spec.identity, Identity::Numbered { .. }))
            .map(|spec| spec.token);
        for token in numbered_tokens {
            let dir = dir_for(token).expect("a Numbered doc type has a directory");
            assert_eq!(doc_type_for_dir(dir), Some(token));
        }
    }

    #[test]
    fn frontmatter_type_for_uses_canonical_casing() {
        assert_eq!(frontmatter_type_for("adr"), Some("ADR"));
        assert_eq!(frontmatter_type_for("bdr"), Some("BDR"));
        assert_eq!(frontmatter_type_for("prd"), Some("PRD"));
        assert_eq!(frontmatter_type_for("issue"), Some("Issue"));
    }

    #[test]
    fn slugify_lowercases_and_kebab_cases() {
        assert_eq!(slugify("My Title"), "my-title");
    }

    #[test]
    fn slugify_collapses_punctuation_runs_into_one_hyphen() {
        assert_eq!(slugify("Some Complex, Title!!"), "some-complex-title");
    }

    #[test]
    fn slugify_trims_leading_and_trailing_separators() {
        assert_eq!(slugify("  --Weird Title--  "), "weird-title");
    }

    #[test]
    fn slugify_of_only_punctuation_is_empty() {
        assert_eq!(slugify("!!!"), "");
    }
}
