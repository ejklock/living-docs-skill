//! Re-export shim (ADR 0019 slice S1): the canonical markdown serializer
//! moved to `living_docs_core::record::to_canonical_markdown`. Re-exported
//! unchanged so `db_store::serialize::to_canonical_markdown` keeps
//! resolving exactly as before.

pub use living_docs_core::record::to_canonical_markdown;
