//! The one list of advisory kinds `check` reports. The check modules tag
//! each advisory with a constant from here and the CLI help renders [`ALL`],
//! so the report and the documentation share the names.

/// One advisory kind and what a finding of that kind means.
pub struct Kind {
    pub name: &'static str,
    pub meaning: &'static str,
}

pub const SIZE: &str = "size";
pub const OWNER: &str = "owner";
pub const HEADING: &str = "heading";
pub const LIVENESS: &str = "liveness";
pub const MOVED_SOURCE: &str = "moved-source";

/// Every kind an advisory can carry, in a stable order.
pub const ALL: &[Kind] = &[
    Kind {
        name: SIZE,
        meaning: "a record body is longer than the advisory length target",
    },
    Kind {
        name: OWNER,
        meaning: "a record of a type that requires an owner has none",
    },
    Kind {
        name: HEADING,
        meaning: "a record heading disagrees with its frontmatter title",
    },
    Kind {
        name: LIVENESS,
        meaning: "a proposed record is stale because its linked issue is already terminal",
    },
    Kind {
        name: MOVED_SOURCE,
        meaning: "a record depends on a record whose status moved since it was written",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn no_two_kinds_share_a_name() {
        let names: BTreeSet<&str> = ALL.iter().map(|kind| kind.name).collect();
        assert_eq!(names.len(), ALL.len());
    }

    #[test]
    fn every_kind_has_a_meaning() {
        assert!(ALL.iter().all(|kind| !kind.meaning.is_empty()));
    }
}
