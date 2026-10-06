//! The section schema of every doc type, in template order. One place lists
//! them all so a type's required headings and its disclosure tiers are read
//! side by side.

use super::{Requirement, SectionSpec, Tier};

const fn section(name: &'static str, requirement: Requirement, tier: Tier) -> SectionSpec {
    SectionSpec {
        name,
        requirement,
        tier,
    }
}

pub(super) const ADR_SECTIONS: &[SectionSpec] = &[
    section("Context", Requirement::Required, Tier::Contract),
    section("Decision", Requirement::Required, Tier::Contract),
    section("Consequences", Requirement::Required, Tier::Contract),
    section("Verification", Requirement::Optional, Tier::Contract),
    section("References", Requirement::Optional, Tier::Detail),
];

pub(super) const PRD_SECTIONS: &[SectionSpec] = &[
    section(
        "Problem / Motivation",
        Requirement::Required,
        Tier::Contract,
    ),
    section("Goals", Requirement::Optional, Tier::Contract),
    section("Non-goals", Requirement::Required, Tier::Contract),
    section("Requirements", Requirement::Optional, Tier::Contract),
    section("Acceptance criteria", Requirement::Optional, Tier::Contract),
    section("Success metrics", Requirement::Optional, Tier::Contract),
    section("Open questions", Requirement::Optional, Tier::Detail),
    section("Related", Requirement::Optional, Tier::Detail),
];

pub(super) const ISSUE_SECTIONS: &[SectionSpec] = &[
    section("Scope", Requirement::Required, Tier::Contract),
    section("Decision", Requirement::Optional, Tier::Contract),
    section("Acceptance", Requirement::Required, Tier::Contract),
    section("Plan", Requirement::Optional, Tier::Detail),
    section("Outcome", Requirement::Optional, Tier::Detail),
];

pub(super) const RESEARCH_SECTIONS: &[SectionSpec] = &[
    section("Question", Requirement::Optional, Tier::Contract),
    section("Method", Requirement::Optional, Tier::Detail),
    section("Findings", Requirement::Optional, Tier::Contract),
    section("Implications", Requirement::Optional, Tier::Contract),
    section("Open Questions", Requirement::Optional, Tier::Detail),
    section("References", Requirement::Optional, Tier::Detail),
];

pub(super) const CONSTITUTION_SECTIONS: &[SectionSpec] = &[
    section("Product", Requirement::Required, Tier::Contract),
    section("Scope Boundaries", Requirement::Required, Tier::Contract),
    section(
        "Data Model / Schema Foundation",
        Requirement::Optional,
        Tier::Contract,
    ),
    section("Non-negotiables", Requirement::Required, Tier::Contract),
    section("Amendment Log", Requirement::Optional, Tier::Detail),
];
