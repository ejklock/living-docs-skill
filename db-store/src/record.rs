//! Re-export shim (ADR 0019 slice S1): the canonical record model —
//! [`ExtractedRecord`], [`extract_record`], and the identity-kind constants
//! — moved to `living_docs_core::record`. This module re-exports its public
//! items unchanged so `db_store::record::extract_record`,
//! `db_store::ExtractedRecord`, and every existing caller keep resolving
//! exactly as before. [`is_reserved`] and [`SearchHit`] are db-store-specific
//! (the FTS5 read-model projection) and stay defined here.

use std::path::Path;

pub use living_docs_core::record::{
    extract_record, ExtractedRecord, TailValue, CONCEPT_IDENTITY_KIND, NUMBER_IDENTITY_KIND,
};

/// A single ranked full-text search hit: the record's bundle-relative path,
/// its title, an FTS5 snippet highlighting the matched term, and the slug of
/// the project it belongs to (ADR 0005, issue 0005 slice 0005-C1). Every
/// hit carries `project` regardless of whether the search that produced it
/// was scoped to one project or spanned all of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub project: String,
}

/// True for the two reserved filenames that carry no OKF frontmatter and are
/// excluded from the read-model, mirroring
/// `living_docs_core::check::records::is_reserved`.
pub fn is_reserved(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("index.md") | Some("log.md")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn is_reserved_matches_index_and_log_only() {
        assert!(is_reserved(Path::new("/bundle/index.md")));
        assert!(is_reserved(Path::new("/bundle/log.md")));
        assert!(!is_reserved(Path::new("/bundle/adr/0001-title.md")));
    }

    #[test]
    fn search_hit_carries_path_title_snippet_and_project() {
        let hit = SearchHit {
            path: "adr/0001-quokka-caching.md".to_owned(),
            title: "Quokka Caching".to_owned(),
            snippet: "an aggressive [quokka] caching strategy".to_owned(),
            project: "team-a".to_owned(),
        };

        assert_eq!(
            hit.path,
            PathBuf::from("adr/0001-quokka-caching.md").to_string_lossy()
        );
        assert!(hit.snippet.contains("[quokka]"));
        assert_eq!(hit.project, "team-a");
    }
}
