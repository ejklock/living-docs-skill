//! Referential-integrity fitness tests for the multi-project schema (ADR
//! 0005, issue 0005 slice 0005-A): a `relations`/`record_tags` row that
//! references a record or tag which does not exist is refused by SQLite's
//! foreign-key enforcement (no server required), and every record `sync`
//! inserts carries the default project's id.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use db_store::entity::{projects, record_tags, records, relations, tags};
use db_store::{connect_in_memory, migrate, sync};
use living_docs_core::store::DocStore;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait};

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

const QUOKKA_DOC: &str = "---\ntype: ADR\ntitle: Quokka Caching Strategy\ndescription: Adopt quokka-based caching for the read model.\nstatus: Accepted\n---\n# 0001. Quokka Caching Strategy\n\nWe adopt an aggressive quokka caching strategy for search results.\n";

fn single_record_corpus() -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from("/bundle");
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("adr").join("0001-quokka-caching.md"),
        QUOKKA_DOC.to_owned(),
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

/// Syncs a single record and returns `(project_id, record_id)` for the
/// record that landed in the default project.
async fn seed_project_and_record(conn: &DatabaseConnection) -> (i32, i32) {
    let (store, bundle) = single_record_corpus();
    sync(conn, &store, &bundle)
        .await
        .expect("sync seeds the default project and one record");

    let record = records::Entity::find()
        .one(conn)
        .await
        .expect("query the synced record")
        .expect("sync inserted exactly one record");

    (record.project_id, record.id)
}

fn assert_foreign_key_violation(error: sea_orm::DbErr) {
    let message = error.to_string().to_lowercase();
    assert!(
        message.contains("foreign key"),
        "expected a foreign key violation, got: {error}"
    );
}

#[tokio::test]
async fn relations_insert_is_refused_when_the_referenced_record_is_missing() {
    let conn = connected_and_migrated().await;
    let (project_id, existing_record_id) = seed_project_and_record(&conn).await;
    let missing_record_id = existing_record_id + 1_000;

    let result = relations::ActiveModel {
        project_id: ActiveValue::Set(project_id),
        from_record_id: ActiveValue::Set(existing_record_id),
        to_record_id: ActiveValue::Set(missing_record_id),
        kind: ActiveValue::Set("supersede".to_owned()),
        ..Default::default()
    }
    .insert(&conn)
    .await;

    assert_foreign_key_violation(
        result.expect_err("inserting a relation to a nonexistent record must fail"),
    );
}

#[tokio::test]
async fn record_tags_insert_is_refused_when_the_referenced_tag_is_missing() {
    let conn = connected_and_migrated().await;
    let (_project_id, existing_record_id) = seed_project_and_record(&conn).await;

    let result = record_tags::ActiveModel {
        record_id: ActiveValue::Set(existing_record_id),
        tag_id: ActiveValue::Set(999_999),
    }
    .insert(&conn)
    .await;

    assert_foreign_key_violation(
        result.expect_err("inserting a record_tags row for a nonexistent tag must fail"),
    );
}

#[tokio::test]
async fn record_tags_insert_is_refused_when_the_referenced_record_is_missing() {
    let conn = connected_and_migrated().await;
    let (project_id, _existing_record_id) = seed_project_and_record(&conn).await;

    let tag = tags::ActiveModel {
        project_id: ActiveValue::Set(project_id),
        name: ActiveValue::Set("adr".to_owned()),
        ..Default::default()
    }
    .insert(&conn)
    .await
    .expect("a tag under an existing project is accepted");

    let result = record_tags::ActiveModel {
        record_id: ActiveValue::Set(999_999),
        tag_id: ActiveValue::Set(tag.id),
    }
    .insert(&conn)
    .await;

    assert_foreign_key_violation(
        result.expect_err("inserting a record_tags row for a nonexistent record must fail"),
    );
}

#[tokio::test]
async fn relations_and_record_tags_insert_succeed_when_every_referenced_row_exists() {
    let conn = connected_and_migrated().await;
    let (project_id, record_id) = seed_project_and_record(&conn).await;

    relations::ActiveModel {
        project_id: ActiveValue::Set(project_id),
        from_record_id: ActiveValue::Set(record_id),
        to_record_id: ActiveValue::Set(record_id),
        kind: ActiveValue::Set("self-link".to_owned()),
        ..Default::default()
    }
    .insert(&conn)
    .await
    .expect("a relation between two existing records is accepted");

    let tag = tags::ActiveModel {
        project_id: ActiveValue::Set(project_id),
        name: ActiveValue::Set("adr".to_owned()),
        ..Default::default()
    }
    .insert(&conn)
    .await
    .expect("a tag under an existing project is accepted");

    record_tags::ActiveModel {
        record_id: ActiveValue::Set(record_id),
        tag_id: ActiveValue::Set(tag.id),
    }
    .insert(&conn)
    .await
    .expect("a record_tags row linking two existing rows is accepted");
}

#[tokio::test]
async fn sync_assigns_every_record_a_non_null_project_id_pointing_at_the_default_project() {
    let conn = connected_and_migrated().await;
    let (store, bundle) = single_record_corpus();
    sync(&conn, &store, &bundle).await.expect("sync");

    let project = projects::Entity::find()
        .one(&conn)
        .await
        .expect("query the default project")
        .expect("sync creates exactly one default project");

    let synced_records = records::Entity::find()
        .all(&conn)
        .await
        .expect("query records");

    assert!(
        !synced_records.is_empty(),
        "sync must insert at least one record"
    );
    assert!(
        synced_records
            .iter()
            .all(|record| record.project_id == project.id),
        "every synced record must point at the default project"
    );
}

#[tokio::test]
async fn sync_reuses_the_same_default_project_across_repeated_runs() {
    let conn = connected_and_migrated().await;
    let (store, bundle) = single_record_corpus();

    sync(&conn, &store, &bundle).await.expect("first sync");
    sync(&conn, &store, &bundle).await.expect("second sync");

    let project_count = projects::Entity::find()
        .all(&conn)
        .await
        .expect("query projects")
        .len();

    assert_eq!(
        project_count, 1,
        "re-syncing must not create a second default project"
    );
}
