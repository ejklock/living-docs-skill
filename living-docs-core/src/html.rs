//! Single-file HTML render of a bundle for local reading: read-only, never
//! served or published, and byte-identical for the same bundle and the same
//! mermaid.js source.

use crate::store::DocStore;
use std::path::Path;

/// Where the rendered page takes mermaid.js from.
pub enum MermaidSource {
    /// The text of mermaid.js, inlined into the page.
    Inline(String),
    /// A script tag for the pinned CDN URL with its subresource-integrity hash.
    Pinned,
}

/// Renders every record of the bundle at `bundle` into one self-contained,
/// deterministic HTML document.
pub fn render(_store: &dyn DocStore, _bundle: &Path, _mermaid: &MermaidSource) -> String {
    String::new()
}
