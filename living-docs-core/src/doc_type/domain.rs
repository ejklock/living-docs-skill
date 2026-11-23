//! The domain row: living glossaries and rule sets kept in `docs/context/`.

use super::{BodySize, DocTypeSpec, Identity, IndexPartition};

/// Listed by filename. `status_vocabulary` is empty because a domain record
/// is updated in place, never numbered or superseded; the body is exempt from
/// the size target because a glossary is long by nature.
pub(super) const DOMAIN: DocTypeSpec = DocTypeSpec {
    token: "domain",
    identity: Identity::Nested { dir: "context" },
    frontmatter: "Context",
    template: include_str!("../../../skills/living-docs/templates/domain.md"),
    index_heading: "Domain",
    index_partition: IndexPartition::Flat,
    web_creatable: false,
    body_size: BodySize::Exempt,
    status_vocabulary: &[],
    requires_owner: false,
    terminal_statuses: &[],
    sections: &[],
};
