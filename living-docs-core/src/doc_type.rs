//! The single compile-time enumeration of the doc-type taxonomy (ADR 0026).
//! Every site that once hand-wrote the doc-type tokens looks them up here
//! instead, so a token's directory, frontmatter value and template can never
//! disagree — the invariant `commands::new::plan_at` used to assert at
//! runtime becomes unrepresentable.

/// Where a doc type's records live, carried as an enum variant field rather
/// than a struct field so a singleton type cannot have a stale directory
/// (ADR 0026 decision point 1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Identity {
    /// `<dir>/NNNN-<slug>.md`; the number is allocated by `next`.
    Numbered { dir: &'static str },
    /// A single `<file>` relative to the bundle root; a second one is
    /// refused.
    Singleton { file: &'static str },
    /// `<dir>/<slug>.md`, slug from the title — a living record keyed by
    /// its concern, updated in place, never numbered or superseded
    /// (ADR 0036).
    Named { dir: &'static str },
}

/// Whether a doc type's body is measured against the advisory 100/120-line
/// target in `check::size`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BodySize {
    Targeted,
    Exempt,
}

/// The axis `index` partitions a type's records along.
#[derive(PartialEq, Eq, Debug)]
pub enum IndexPartition {
    OpenClosed,
    ActiveSuperseded,
    Flat,
}

/// Everything a doc type needs to be created, indexed and offered by every
/// consumer: its token, path shape, frontmatter value, embedded template,
/// index rendering, web-creatability, and whether its body carries the
/// advisory size target.
#[derive(PartialEq, Debug)]
pub struct DocTypeSpec {
    pub token: &'static str,
    pub identity: Identity,
    pub frontmatter: &'static str,
    pub template: &'static str,
    pub index_heading: &'static str,
    pub index_partition: IndexPartition,
    pub web_creatable: bool,
    pub body_size: BodySize,
    /// The values `living-docs status` is willing to set on this type's
    /// records, in seed order — `status_vocabulary[0]` is what `new`
    /// seed a fresh record with (ADR 0029). `Superseded` is deliberately
    /// never a member of any row: it is reachable only through
    /// `living-docs supersede`, which also wires the
    /// `supersedes`/`superseded_by` links.
    pub status_vocabulary: &'static [&'static str],
    /// Whether `check` treats a missing `owner:` frontmatter value on this
    /// type's records as a finding. A finding is a warning by default and an
    /// invariant violation under `check --require-owner`; a type with
    /// `requires_owner: false` never produces the finding either way.
    pub requires_owner: bool,
    /// The values, case-insensitive, that mark a record of this type as
    /// closed for good — consulted by the moved-source clearing rule to
    /// decide whether a dependent record still needs to react to a linked
    /// record's status change. `Superseded` reaches every type through
    /// `supersede` and never needs to be listed here.
    pub terminal_statuses: &'static [&'static str],
    /// The body sections this type declares, in template order; empty for a
    /// type that declares none. `check` requires the `Required` ones on live
    /// records, and `read` discloses by `Tier`.
    pub sections: &'static [SectionSpec],
}

/// Whether a declared section must be present on a live record.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Requirement {
    Required,
    Optional,
}

/// `Contract` sections are what a reader needs to know what was agreed;
/// `Detail` sections are the working material behind it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tier {
    Contract,
    Detail,
}

/// One body section a doc type declares, matched by heading name.
#[derive(PartialEq, Eq, Debug)]
pub struct SectionSpec {
    pub name: &'static str,
    pub requirement: Requirement,
    pub tier: Tier,
}

mod sections;
use sections::{
    ADR_SECTIONS, CONSTITUTION_SECTIONS, ISSUE_SECTIONS, PRD_SECTIONS, RESEARCH_SECTIONS,
};

impl DocTypeSpec {
    /// Whether `status` (case-insensitive) retires a record of this type:
    /// `Superseded` for every type, or a status this row's
    /// `terminal_statuses` lists. `set title` refuses a retired record and
    /// `check`'s `HEADING`/`SIZE` advisories skip one, both through this one
    /// definition (ADR 0063), so the CLI's writes and `check`'s reads can
    /// never disagree about which records are history.
    pub fn is_retired(&self, status: &str) -> bool {
        status.eq_ignore_ascii_case("superseded")
            || self
                .terminal_statuses
                .iter()
                .any(|terminal| terminal.eq_ignore_ascii_case(status))
    }
}

const ADR: DocTypeSpec = DocTypeSpec {
    token: "adr",
    identity: Identity::Numbered { dir: "adr" },
    frontmatter: "ADR",
    template: include_str!("../../skills/living-docs/templates/adr.md"),
    index_heading: "ADRs",
    index_partition: IndexPartition::ActiveSuperseded,
    web_creatable: true,
    body_size: BodySize::Targeted,
    status_vocabulary: &["Proposed", "Accepted", "Deprecated"],
    requires_owner: true,
    terminal_statuses: &["Deprecated"],
    sections: ADR_SECTIONS,
};

const PRD: DocTypeSpec = DocTypeSpec {
    token: "prd",
    identity: Identity::Numbered { dir: "prd" },
    frontmatter: "PRD",
    template: include_str!("../../skills/living-docs/templates/prd.md"),
    index_heading: "PRDs",
    index_partition: IndexPartition::ActiveSuperseded,
    web_creatable: true,
    body_size: BodySize::Targeted,
    status_vocabulary: &["Draft", "Accepted", "Implemented", "Deprecated"],
    requires_owner: false,
    terminal_statuses: &["Deprecated"],
    sections: PRD_SECTIONS,
};

const ISSUE: DocTypeSpec = DocTypeSpec {
    token: "issue",
    identity: Identity::Numbered { dir: "issues" },
    frontmatter: "Issue",
    template: include_str!("../../skills/living-docs/templates/issue.md"),
    index_heading: "Issues",
    index_partition: IndexPartition::OpenClosed,
    web_creatable: true,
    body_size: BodySize::Targeted,
    status_vocabulary: &["open", "in-progress", "closed"],
    requires_owner: false,
    terminal_statuses: &["closed", "done"],
    sections: ISSUE_SECTIONS,
};

const RESEARCH: DocTypeSpec = DocTypeSpec {
    token: "research",
    identity: Identity::Numbered { dir: "research" },
    frontmatter: "Research",
    template: include_str!("../../skills/living-docs/templates/research.md"),
    index_heading: "Research",
    index_partition: IndexPartition::Flat,
    web_creatable: true,
    body_size: BodySize::Exempt,
    status_vocabulary: &["Draft", "Accepted"],
    requires_owner: false,
    terminal_statuses: &[],
    sections: RESEARCH_SECTIONS,
};

/// The closed `kind` vocabulary for architecture views, in the C4/arc42
/// zoom order the generated index sorts by (ADR 0036): structure from the
/// outside in, then runtime behavior, then data, then deployment. `new
/// view --kind` validates against this list; an absent or unknown kind
/// sorts after every listed one.
pub const VIEW_KIND_ORDER: &[&str] = &[
    "context",
    "container",
    "component",
    "flow",
    "sequence",
    "state",
    "data-model",
    "deployment",
];

/// Architecture views (ADR 0036): living documents keyed by concern in
/// `docs/architecture/`, sequenced in the generated index by their `kind`
/// frontmatter (C4/arc42 zoom order). `status_vocabulary` is empty for the
/// same reason as Constitution's: a view carries no `NNNN`, is updated in
/// place, and is never superseded — git history is its trail.
const VIEW: DocTypeSpec = DocTypeSpec {
    token: "view",
    identity: Identity::Named {
        dir: "architecture",
    },
    frontmatter: "Architecture View",
    template: include_str!("../../skills/living-docs/templates/architecture-view.md"),
    index_heading: "Architecture",
    index_partition: IndexPartition::Flat,
    web_creatable: false,
    body_size: BodySize::Targeted,
    status_vocabulary: &[],
    requires_owner: false,
    terminal_statuses: &[],
    sections: &[],
};

/// `index_heading`/`index_partition` are inert for a singleton — it has no
/// directory index to render — and are set to placeholder values rather than
/// wrapped in an `Option`, since no directory-index code path ever reads them
/// for this row (ADR 0026 decision point 6). `status_vocabulary` is empty for
/// the same reason: a singleton carries no `NNNN`, so `living-docs status
/// <NNNN>` can never resolve one — Constitution's own `Draft | Ratified |
/// Amended` vocabulary is out of this row's scope (ADR 0029).
const CONSTITUTION: DocTypeSpec = DocTypeSpec {
    token: "constitution",
    identity: Identity::Singleton {
        file: "constitution.md",
    },
    frontmatter: "Constitution",
    template: include_str!("../../skills/living-docs/templates/constitution.md"),
    index_heading: "Constitution",
    index_partition: IndexPartition::Flat,
    web_creatable: true,
    body_size: BodySize::Exempt,
    status_vocabulary: &[],
    requires_owner: false,
    terminal_statuses: &[],
    sections: CONSTITUTION_SECTIONS,
};

/// The sole enumeration of the doc-type taxonomy. Every consumer derives
/// from this table instead of hand-syncing its own copy.
pub const DOC_TYPES: &[DocTypeSpec] = &[ADR, PRD, ISSUE, RESEARCH, VIEW, CONSTITUTION];

/// Looks up a doc type by its CLI token.
pub fn spec_for(token: &str) -> Option<&'static DocTypeSpec> {
    DOC_TYPES.iter().find(|spec| spec.token == token)
}

/// Looks up a doc type by its `type:` frontmatter value. Returns the first
/// match, which is well-defined only because `frontmatter` values are unique
/// across `DOC_TYPES` — an invariant guarded by
/// `frontmatter_values_are_unique_so_spec_for_frontmatter_is_well_defined`.
pub fn spec_for_frontmatter(frontmatter: &str) -> Option<&'static DocTypeSpec> {
    DOC_TYPES
        .iter()
        .find(|spec| spec.frontmatter == frontmatter)
}

/// Looks up a doc type by its numbered-series directory name — the reverse
/// of a [`Identity::Numbered`] spec's `dir`. A singleton type has no
/// directory, so it never matches.
pub fn spec_for_dir(dir: &str) -> Option<&'static DocTypeSpec> {
    DOC_TYPES.iter().find(|spec| matches_dir(spec, dir))
}

fn matches_dir(spec: &DocTypeSpec, dir: &str) -> bool {
    match spec.identity {
        Identity::Numbered { dir: spec_dir } | Identity::Named { dir: spec_dir } => spec_dir == dir,
        Identity::Singleton { .. } => false,
    }
}

#[cfg(test)]
mod template_tests;
#[cfg(test)]
mod tests;
