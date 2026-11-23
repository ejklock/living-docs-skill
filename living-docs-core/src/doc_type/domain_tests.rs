use super::*;

#[test]
fn the_domain_row_resolves_by_token_directory_and_frontmatter() {
    let by_token = spec_for("domain").expect("domain token resolves");
    assert_eq!(spec_for("Domain"), Some(by_token));
    assert_eq!(spec_for_dir("context"), Some(by_token));
    assert_eq!(spec_for_frontmatter("Context"), Some(by_token));
}

#[test]
fn a_nested_domain_token_has_no_numbered_directory() {
    assert_eq!(crate::paths::dir_for("domain"), None);
}

#[test]
fn the_domain_row_is_a_nested_status_less_exempt_record() {
    let spec = spec_for("domain").expect("domain token resolves");
    assert_eq!(
        format!("{:?}", spec.identity),
        r#"Nested { dir: "context" }"#
    );
    assert_eq!(spec.body_size, BodySize::Exempt);
    assert!(spec.status_vocabulary.is_empty());
    assert!(spec.terminal_statuses.is_empty());
    assert!(spec.sections.is_empty());
    assert!(!spec.requires_owner);
}
