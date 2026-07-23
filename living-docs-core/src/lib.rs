//! Deterministic domain layer of Living Docs authoring (ADR 0001): doc-type
//! mapping, embedded templates, frontmatter reading, the `new`/`index`/`next`/
//! `supersede` commands, and the `check` invariant suite. Depends on no
//! adapter or front — `cli` is a thin arg-parsing shell over this crate.

pub mod check;
pub mod commands;
pub mod frontmatter;
pub mod paths;
pub mod pii;
pub mod record;
pub mod store;
pub mod templates;
