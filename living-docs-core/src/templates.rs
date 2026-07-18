/// Compile-time embedded doc templates, keyed by the CLI's doc-type token.
/// Embedding (rather than reading from disk at runtime) keeps the binary
/// self-contained per ADR 0001.
pub fn template_for(doc_type: &str) -> Option<&'static str> {
    match doc_type {
        "adr" => Some(include_str!("../../skills/living-docs/templates/adr.md")),
        "bdr" => Some(include_str!("../../skills/living-docs/templates/bdr.md")),
        "prd" => Some(include_str!("../../skills/living-docs/templates/prd.md")),
        "issue" => Some(include_str!("../../skills/living-docs/templates/issue.md")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_for_returns_the_matching_embedded_template() {
        assert!(template_for("adr").unwrap().starts_with("---\ntype: ADR"));
        assert!(template_for("bdr").unwrap().starts_with("---\ntype: BDR"));
        assert!(template_for("prd").unwrap().starts_with("---\ntype: PRD"));
        assert!(template_for("issue")
            .unwrap()
            .starts_with("---\ntype: Issue"));
    }

    #[test]
    fn template_for_rejects_unknown_types() {
        assert_eq!(template_for("bogus"), None);
        assert_eq!(template_for("constitution"), None);
    }
}
