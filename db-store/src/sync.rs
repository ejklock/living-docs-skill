//! Full-rebuild sync from a [`living_docs_core::store::DocStore`] into the
//! `records` read-model, plus its backend-native search index (ADR 0004,
//! issue 0002 slice S2b; ParadeDB branch issue 0004 slice 0004-B;
//! default-project assignment issue 0005 slice 0005-A).

use std::path::Path;

use living_docs_core::store::DocStore;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend,
    DbErr, EntityTrait, QueryFilter, Statement, TransactionTrait,
};

use crate::entity::projects;
use crate::entity::{ActiveModel, Entity as Records};
use crate::record::{extract_record, is_reserved};
use crate::Result;

/// The slug every synced record is assigned to until per-project ingestion
/// (`db sync --project <slug>`) lands in issue 0005 slice 0005-B.
const DEFAULT_PROJECT_SLUG: &str = "default";

/// Rebuilds the `records` table and its backend-native search index from
/// every non-reserved `.md` doc `store` lists under `bundle`, in one
/// transaction. Idempotent: running twice over an unchanged corpus yields
/// identical rows, since the table is fully cleared before repopulating.
/// Every inserted record is assigned to the single default project (upserted
/// by slug on first use). Returns the number of records inserted.
pub async fn sync(conn: &DatabaseConnection, store: &dyn DocStore, bundle: &Path) -> Result<usize> {
    let paths = store.list(bundle).map_err(io_err_to_db_err)?;
    let txn = conn.begin().await?;

    let project_id = ensure_default_project(&txn, bundle).await?;
    Records::delete_many().exec(&txn).await?;

    let mut count = 0usize;
    for path in paths {
        if is_reserved(&path) {
            continue;
        }
        insert_record(&txn, store, bundle, &path, project_id).await?;
        count += 1;
    }

    rebuild_search_index(&txn).await?;
    txn.commit().await?;
    Ok(count)
}

/// Finds the default project by its stable slug, inserting it (rooted at
/// `bundle`) the first time `sync` runs against a fresh database. Returns
/// the project's id either way.
async fn ensure_default_project<C: ConnectionTrait>(conn: &C, bundle: &Path) -> Result<i32> {
    if let Some(existing) = projects::Entity::find()
        .filter(projects::Column::Slug.eq(DEFAULT_PROJECT_SLUG))
        .one(conn)
        .await?
    {
        return Ok(existing.id);
    }

    let inserted = projects::ActiveModel {
        slug: ActiveValue::Set(DEFAULT_PROJECT_SLUG.to_owned()),
        name: ActiveValue::Set(DEFAULT_PROJECT_SLUG.to_owned()),
        root_path: ActiveValue::Set(Some(bundle.to_string_lossy().into_owned())),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    Ok(inserted.id)
}

async fn insert_record<C: ConnectionTrait>(
    conn: &C,
    store: &dyn DocStore,
    bundle: &Path,
    path: &Path,
    project_id: i32,
) -> Result<()> {
    let relative = path
        .strip_prefix(bundle)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();
    let contents = store.read(path).map_err(io_err_to_db_err)?;
    let extracted = extract_record(path, &contents);

    ActiveModel {
        project_id: ActiveValue::Set(project_id),
        path: ActiveValue::Set(relative),
        doc_type: ActiveValue::Set(extracted.doc_type),
        identity: ActiveValue::Set(extracted.identity),
        title: ActiveValue::Set(extracted.title),
        description: ActiveValue::Set(extracted.description),
        body: ActiveValue::Set(extracted.body),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    Ok(())
}

/// Rebuilds the backend-native search index over `records`. SQLite's FTS5
/// external-content index is stale after a bulk write and must be told to
/// rebuild; Postgres's `pg_search` BM25 index updates automatically on
/// insert, so this is a no-op there.
async fn rebuild_search_index<C: ConnectionTrait>(conn: &C) -> Result<()> {
    match conn.get_database_backend() {
        DbBackend::Sqlite => conn
            .execute(Statement::from_string(
                DbBackend::Sqlite,
                "INSERT INTO records_fts(records_fts) VALUES('rebuild')".to_owned(),
            ))
            .await
            .map(|_| ()),
        DbBackend::Postgres | DbBackend::MySql => Ok(()),
    }
}

fn io_err_to_db_err(err: std::io::Error) -> DbErr {
    DbErr::Custom(err.to_string())
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::collections::BTreeMap;
    use std::io;
    use std::path::{Path, PathBuf};

    use living_docs_core::store::DocStore;

    pub(crate) struct MemoryStore {
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
    const UNRELATED_DOC: &str = "---\ntype: ADR\ntitle: Unrelated Decision\ndescription: Something else entirely.\nstatus: Accepted\n---\n# 0002. Unrelated Decision\n\nThis document discusses logging conventions.\n";

    pub(crate) fn seeded_corpus() -> (MemoryStore, PathBuf) {
        let bundle = PathBuf::from("/bundle");
        let mut files = BTreeMap::new();
        files.insert(
            bundle.join("adr").join("0001-quokka-caching.md"),
            QUOKKA_DOC.to_owned(),
        );
        files.insert(
            bundle.join("adr").join("0002-unrelated.md"),
            UNRELATED_DOC.to_owned(),
        );
        files.insert(bundle.join("index.md"), "# Index\n".to_owned());
        (MemoryStore { files }, bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::seeded_corpus;
    use super::*;
    use crate::{connect_in_memory, migrate};
    use sea_orm::{FromQueryResult, QueryOrder};

    #[derive(Debug, FromQueryResult, PartialEq, Eq)]
    struct RecordRow {
        path: String,
        title: String,
    }

    async fn all_records(conn: &DatabaseConnection) -> Vec<RecordRow> {
        Records::find()
            .order_by_asc(crate::entity::Column::Path)
            .into_model::<RecordRow>()
            .all(conn)
            .await
            .expect("query records")
    }

    async fn row_count(conn: &DatabaseConnection, table: &str) -> i64 {
        conn.query_one(Statement::from_string(
            conn.get_database_backend(),
            format!("SELECT COUNT(*) AS n FROM {table}"),
        ))
        .await
        .expect("run count query")
        .expect("count query returns one row")
        .try_get::<i64>("", "n")
        .expect("n column")
    }

    #[tokio::test]
    async fn sync_skips_reserved_files_and_inserts_the_rest() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");
        let (store, bundle) = seeded_corpus();

        let count = sync(&conn, &store, &bundle).await.expect("sync");

        assert_eq!(count, 2);
        let rows = all_records(&conn).await;
        assert_eq!(
            rows,
            vec![
                RecordRow {
                    path: "adr/0001-quokka-caching.md".to_owned(),
                    title: "Quokka Caching Strategy".to_owned(),
                },
                RecordRow {
                    path: "adr/0002-unrelated.md".to_owned(),
                    title: "Unrelated Decision".to_owned(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn sync_is_idempotent_across_repeated_runs() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");
        let (store, bundle) = seeded_corpus();

        sync(&conn, &store, &bundle).await.expect("first sync");
        let first_rows = all_records(&conn).await;

        let second_count = sync(&conn, &store, &bundle).await.expect("second sync");
        let second_rows = all_records(&conn).await;

        assert_eq!(second_count, 2);
        assert_eq!(first_rows, second_rows);
        assert_eq!(row_count(&conn, "records").await, 2);
    }

    #[tokio::test]
    async fn sync_populates_the_fts_index() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");
        let (store, bundle) = seeded_corpus();

        sync(&conn, &store, &bundle).await.expect("sync");

        assert_eq!(row_count(&conn, "records_fts").await, 2);
    }

    #[tokio::test]
    async fn rebuild_search_index_is_a_no_op_on_postgres() {
        let mut options = sea_orm::ConnectOptions::new("postgres://user:pass@localhost/db");
        options.connect_lazy(true);
        let conn = sea_orm::Database::connect(options)
            .await
            .expect("lazy postgres connect never touches the network");

        rebuild_search_index(&conn)
            .await
            .expect("postgres rebuild is a no-op that never issues SQL");
    }
}
