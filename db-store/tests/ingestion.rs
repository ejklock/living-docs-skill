//! Per-project ingestion fitness tests (ADR 0005, issue 0005 slice
//! 0005-B): supersede frontmatter yields a `relations` row linking the two
//! records, `tags` frontmatter yields `tags`/`record_tags` rows, every row
//! carries the right `project_id`, and re-syncing one project never touches
//! another project's records, relations, or tags (no server required).

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use db_store::entity::{projects, record_tags, records, relations, tags};
use db_store::{connect_in_memory, migrate, sync_project};
use living_docs_core::store::DocStore;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

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

const OLD_DOC: &str = "---\ntype: ADR\ntitle: Quokka Caching Strategy\ndescription: Adopt quokka-based caching.\nstatus: Superseded\nsuperseded_by: 0002\ntags: [caching]\n---\n# 0001. Quokka Caching Strategy\n\nBody.\n";
const NEW_DOC: &str = "---\ntype: ADR\ntitle: Improved Caching Strategy\ndescription: Supersedes quokka caching.\nstatus: Accepted\nsupersedes: 0001\ntags: [caching, performance]\n---\n# 0002. Improved Caching Strategy\n\nBody.\n";

fn supersede_and_tags_corpus(bundle_root: &str) -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from(bundle_root);
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("adr").join("0001-quokka-caching.md"),
        OLD_DOC.to_owned(),
    );
    files.insert(
        bundle.join("adr").join("0002-improved-caching.md"),
        NEW_DOC.to_owned(),
    );
    (MemoryStore { files }, bundle)
}

fn single_tagged_record_corpus(bundle_root: &str) -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from(bundle_root);
    let doc = "---\ntype: ADR\ntitle: Team B Decision\ndescription: d.\nstatus: Accepted\ntags: [team-b]\n---\n# 0001. Team B Decision\n\nBody.\n";
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("adr").join("0001-team-b-decision.md"),
        doc.to_owned(),
    );
    (MemoryStore { files }, bundle)
}

async fn connected_and_migrated() -> DatabaseConnection {
    let conn = connect_in_memory()
        .await
        .expect("connect to in-memory sqlite");
    migrate(&conn).await.expect("migrate");
    conn
}

async fn project_by_slug(conn: &DatabaseConnection, slug: &str) -> projects::Model {
    projects::Entity::find()
        .filter(projects::Column::Slug.eq(slug))
        .one(conn)
        .await
        .expect("query project")
        .unwrap_or_else(|| panic!("project '{slug}' was not synced"))
}

async fn record_id(conn: &DatabaseConnection, project_id: i32, path: &str) -> i32 {
    records::Entity::find()
        .filter(records::Column::ProjectId.eq(project_id))
        .filter(records::Column::Path.eq(path))
        .one(conn)
        .await
        .expect("query record")
        .unwrap_or_else(|| panic!("record at {path} was not synced into project {project_id}"))
        .id
}

async fn record_ids_for_project(conn: &DatabaseConnection, project_id: i32) -> Vec<i32> {
    records::Entity::find()
        .filter(records::Column::ProjectId.eq(project_id))
        .all(conn)
        .await
        .expect("query records")
        .into_iter()
        .map(|record| record.id)
        .collect()
}

async fn relations_for_project(
    conn: &DatabaseConnection,
    project_id: i32,
) -> Vec<relations::Model> {
    relations::Entity::find()
        .filter(relations::Column::ProjectId.eq(project_id))
        .all(conn)
        .await
        .expect("query relations")
}

async fn tags_for_project(conn: &DatabaseConnection, project_id: i32) -> Vec<tags::Model> {
    tags::Entity::find()
        .filter(tags::Column::ProjectId.eq(project_id))
        .all(conn)
        .await
        .expect("query tags")
}

async fn record_tags_for_records(
    conn: &DatabaseConnection,
    record_ids: &[i32],
) -> Vec<record_tags::Model> {
    record_tags::Entity::find()
        .all(conn)
        .await
        .expect("query record_tags")
        .into_iter()
        .filter(|row| record_ids.contains(&row.record_id))
        .collect()
}

#[tokio::test]
async fn supersede_frontmatter_yields_one_relations_row_linking_the_two_records() {
    let conn = connected_and_migrated().await;
    let (store, bundle) = supersede_and_tags_corpus("/bundle-supersede");

    sync_project(&conn, &store, &bundle, "team-a")
        .await
        .expect("sync team-a");

    let project = project_by_slug(&conn, "team-a").await;
    let old_record_id = record_id(&conn, project.id, "adr/0001-quokka-caching.md").await;
    let new_record_id = record_id(&conn, project.id, "adr/0002-improved-caching.md").await;

    let relations = relations_for_project(&conn, project.id).await;

    assert_eq!(
        relations.len(),
        1,
        "supersedes + reciprocal superseded_by must resolve to exactly one relation, got: \
         {relations:?}"
    );
    let relation = &relations[0];
    assert_eq!(relation.kind, "supersede");
    assert_eq!(relation.project_id, project.id);
    assert_eq!(
        relation.from_record_id, new_record_id,
        "the newer record (declares supersedes) is the relation's source"
    );
    assert_eq!(
        relation.to_record_id, old_record_id,
        "the older record (declares superseded_by) is the relation's target"
    );
}

#[tokio::test]
async fn tags_frontmatter_yields_tags_and_record_tags_rows_with_the_right_project_id() {
    let conn = connected_and_migrated().await;
    let (store, bundle) = supersede_and_tags_corpus("/bundle-tags");

    sync_project(&conn, &store, &bundle, "team-a")
        .await
        .expect("sync team-a");

    let project = project_by_slug(&conn, "team-a").await;
    let project_tags = tags_for_project(&conn, project.id).await;
    let mut tag_names: Vec<&str> = project_tags.iter().map(|tag| tag.name.as_str()).collect();
    tag_names.sort_unstable();
    assert_eq!(tag_names, vec!["caching", "performance"]);
    assert!(project_tags.iter().all(|tag| tag.project_id == project.id));

    let record_ids = record_ids_for_project(&conn, project.id).await;
    let record_tags = record_tags_for_records(&conn, &record_ids).await;

    assert_eq!(
        record_tags.len(),
        3,
        "one caching link per record (2) plus one performance link on the new record, got: \
         {record_tags:?}"
    );
}

#[tokio::test]
async fn resyncing_one_project_does_not_touch_another_projects_rows() {
    let conn = connected_and_migrated().await;
    let (store_a, bundle_a) = supersede_and_tags_corpus("/bundle-a");
    let (store_b, bundle_b) = single_tagged_record_corpus("/bundle-b");

    sync_project(&conn, &store_a, &bundle_a, "team-a")
        .await
        .expect("sync team-a");
    sync_project(&conn, &store_b, &bundle_b, "team-b")
        .await
        .expect("sync team-b");

    let project_b = project_by_slug(&conn, "team-b").await;
    let record_ids_before = record_ids_for_project(&conn, project_b.id).await;
    let tags_before = tags_for_project(&conn, project_b.id).await;
    let record_tags_before = record_tags_for_records(&conn, &record_ids_before).await;

    sync_project(&conn, &store_a, &bundle_a, "team-a")
        .await
        .expect("re-sync team-a");

    let record_ids_after = record_ids_for_project(&conn, project_b.id).await;
    let tags_after = tags_for_project(&conn, project_b.id).await;
    let record_tags_after = record_tags_for_records(&conn, &record_ids_after).await;

    assert_eq!(
        record_ids_before, record_ids_after,
        "re-syncing team-a must not touch team-b's records"
    );
    assert_eq!(
        tags_before, tags_after,
        "re-syncing team-a must not touch team-b's tags"
    );
    assert_eq!(
        record_tags_before, record_tags_after,
        "re-syncing team-a must not touch team-b's record_tags"
    );

    let relations_a = relations_for_project(&conn, project_by_slug(&conn, "team-a").await.id).await;
    assert_eq!(
        relations_a.len(),
        1,
        "team-a's supersede relation must survive its own re-sync"
    );
}
