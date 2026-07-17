//! Cross-project + project-scoped search fitness tests (ADR 0005, issue
//! 0005 slice 0005-C1): an unscoped search spans every project and labels
//! each hit with its project slug; a scoped search narrows to one
//! project's hits (sqlite in-memory, no server required).

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use db_store::{connect_in_memory, migrate, search, search_in_project, sync_project};
use living_docs_core::store::DocStore;
use sea_orm::DatabaseConnection;

struct MemoryStore {
    files: BTreeMap<PathBuf, String>,
}

impl DocStore for MemoryStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(self
            .files
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect())
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
    }

    fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
        Ok(())
    }
}

fn quokka_corpus(bundle_root: &str, title: &str) -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from(bundle_root);
    let doc = format!(
        "---\ntype: ADR\ntitle: {title}\ndescription: d.\nstatus: Accepted\n---\n# {title}\n\nAn aggressive quokka caching strategy.\n"
    );
    let mut files = BTreeMap::new();
    files.insert(bundle.join("adr").join("0001-quokka.md"), doc);
    (MemoryStore { files }, bundle)
}

async fn connected_and_migrated() -> DatabaseConnection {
    let conn = connect_in_memory()
        .await
        .expect("connect to in-memory sqlite");
    migrate(&conn).await.expect("migrate");
    conn
}

async fn seed_two_projects_sharing_a_term(conn: &DatabaseConnection) {
    let (store_a, bundle_a) = quokka_corpus("/bundle-a", "Team A Quokka Caching");
    sync_project(conn, &store_a, &bundle_a, "team-a")
        .await
        .expect("sync team-a");

    let (store_b, bundle_b) = quokka_corpus("/bundle-b", "Team B Quokka Caching");
    sync_project(conn, &store_b, &bundle_b, "team-b")
        .await
        .expect("sync team-b");
}

#[tokio::test]
async fn unscoped_search_returns_hits_from_every_project_labeled_by_project() {
    let conn = connected_and_migrated().await;
    seed_two_projects_sharing_a_term(&conn).await;

    let hits = search(&conn, "quokka").await.expect("search");

    assert_eq!(hits.len(), 2, "both projects share the term, got: {hits:?}");
    let mut projects: Vec<&str> = hits.iter().map(|hit| hit.project.as_str()).collect();
    projects.sort_unstable();
    assert_eq!(projects, vec!["team-a", "team-b"]);
}

#[tokio::test]
async fn scoped_search_returns_only_that_projects_hits() {
    let conn = connected_and_migrated().await;
    seed_two_projects_sharing_a_term(&conn).await;

    let hits = search_in_project(&conn, "quokka", "team-a")
        .await
        .expect("scoped search");

    assert_eq!(hits.len(), 1, "only team-a should match, got: {hits:?}");
    assert_eq!(hits[0].project, "team-a");
    assert_eq!(hits[0].title, "Team A Quokka Caching");
}

#[tokio::test]
async fn scoped_search_against_an_unknown_project_returns_empty() {
    let conn = connected_and_migrated().await;
    seed_two_projects_sharing_a_term(&conn).await;

    let hits = search_in_project(&conn, "quokka", "team-nonexistent")
        .await
        .expect("scoped search against an unknown project");

    assert!(hits.is_empty());
}

#[tokio::test]
async fn unscoped_search_with_no_match_returns_empty() {
    let conn = connected_and_migrated().await;
    seed_two_projects_sharing_a_term(&conn).await;

    let hits = search(&conn, "zzzznomatch")
        .await
        .expect("search with no match");

    assert!(hits.is_empty());
}
