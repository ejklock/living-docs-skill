//! `db-store`: the SQLite/FTS5 derived read-model adapter (ADR 0004, issue
//! 0002). Slice S2a landed the schema foundation (connect + migrate). Slice
//! S2b adds the idempotent full-rebuild `sync`, ranked FTS5 `search`, and the
//! `SearchIndex` port implementation. CLI wiring lands in S2c. Slice 0006-B
//! adds the canonical [`serialize`] module and [`DbDocStore`], the `DocStore`
//! port's read-side adapter (ADR 0007).

pub mod entity;
pub mod migration;
pub mod record;
pub mod search;
pub mod serialize;
pub mod sync;

use std::io;
use std::path::{Path, PathBuf};

use living_docs_core::store::{DocStore, SearchIndex};
use sea_orm::{
    ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, EntityTrait,
    QueryFilter, QueryOrder,
};
use sea_orm_migration::MigratorTrait;

use entity::projects::{Column as ProjectColumn, Entity as Projects};
use entity::{frontmatter_fields, record_tags, relations, tags};
use entity::{Column, Entity as Records};
use migration::Migrator;

pub use record::{ExtractedRecord, SearchHit};
pub use search::{search, search_in_project};
pub use sync::{sync, sync_project};

/// A single record's title and markdown source body, looked up by its
/// bundle-relative path (ADR 0006, issue 0003 slice S3b).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordView {
    pub title: String,
    pub body: String,
}

/// Looks up the record at `path`, spanning every project. Returns `None`
/// when no record exists at that path. `path` is now unique only within a
/// project (`UNIQUE(project_id, path)`, ADR 0005 issue 0005 slice 0005-A),
/// so once a second project exists this can match an arbitrary one of
/// several same-path records; callers that know their project should use
/// [`record_by_path_in_project`] instead. Kept unscoped for the web front,
/// which is not project-aware until issue 0005 slice 0005-C.
pub async fn record_by_path(conn: &DatabaseConnection, path: &str) -> Result<Option<RecordView>> {
    let record = Records::find()
        .filter(Column::Path.eq(path))
        .one(conn)
        .await?;
    Ok(record.map(record_to_view))
}

/// Looks up the record at `path` within `project_id` only. Returns `None`
/// when no record exists at that path in that project (ADR 0005, issue
/// 0005 slice 0005-B).
pub async fn record_by_path_in_project(
    conn: &DatabaseConnection,
    project_id: i32,
    path: &str,
) -> Result<Option<RecordView>> {
    let record = Records::find()
        .filter(Column::ProjectId.eq(project_id))
        .filter(Column::Path.eq(path))
        .one(conn)
        .await?;
    Ok(record.map(record_to_view))
}

fn record_to_view(model: entity::Model) -> RecordView {
    RecordView {
        title: model.title,
        body: model.body,
    }
}

/// A single project's slug and display name, as listed for the web front's
/// project filter (ADR 0005, issue 0005 slice 0005-C2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectView {
    pub slug: String,
    pub name: String,
}

/// Lists every project, ordered by name, for the web front's project filter
/// (ADR 0005, issue 0005 slice 0005-C2). Built entirely from the SeaORM
/// query builder so it runs unchanged on both the sqlite and postgres
/// backends (lesson 3696: no raw per-engine SQL here).
pub async fn list_projects(conn: &DatabaseConnection) -> Result<Vec<ProjectView>> {
    let projects = Projects::find()
        .order_by_asc(ProjectColumn::Name)
        .all(conn)
        .await?;
    Ok(projects.into_iter().map(project_to_view).collect())
}

fn project_to_view(model: entity::projects::Model) -> ProjectView {
    ProjectView {
        slug: model.slug,
        name: model.name,
    }
}

/// Result alias for this crate's fallible operations, using SeaORM's own
/// error type since every failure here originates from the database layer.
pub type Result<T> = std::result::Result<T, DbErr>;

/// Opens a database connection at `url`, inferring the backend from its
/// scheme (`sqlite://…` or `postgres://…`, ADR 0004 issue 0004). For a
/// SQLite file URL, creates any missing parent directories before handing
/// the URL to SeaORM unchanged; other schemes are passed straight through.
/// A SQLite connection has `PRAGMA foreign_keys` enabled so the multi-project
/// FK constraints (ADR 0005 issue 0005 slice 0005-A) are enforced; Postgres
/// enforces foreign keys natively and needs no such step.
pub async fn connect(url: &str) -> Result<DatabaseConnection> {
    if let Some(path) = sqlite_file_path(url) {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(|err| DbErr::Custom(err.to_string()))?;
        }
    }
    let conn = Database::connect(url).await?;
    enable_sqlite_foreign_keys(&conn).await?;
    Ok(conn)
}

/// Turns on SQLite's foreign-key enforcement for `conn`, a no-op on every
/// other backend. SeaORM defaults a SQLite connection pool to a single
/// connection unless told otherwise, so one `PRAGMA` here covers every query
/// this connection ever runs.
async fn enable_sqlite_foreign_keys(conn: &DatabaseConnection) -> Result<()> {
    if conn.get_database_backend() != DbBackend::Sqlite {
        return Ok(());
    }
    conn.execute_unprepared("PRAGMA foreign_keys = ON")
        .await
        .map(|_| ())
}

/// Extracts the filesystem path from a SQLite file URL (`sqlite://<path>`,
/// optionally followed by a `?query`), or `None` for the in-memory form
/// (`sqlite::memory:`) and every non-SQLite scheme.
fn sqlite_file_path(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("sqlite://")?;
    let path = rest.split('?').next().unwrap_or(rest);
    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// Opens an in-memory SQLite connection for tests, with
/// `PRAGMA foreign_keys` enabled (see [`connect`]). State is discarded once
/// the returned connection is dropped.
pub async fn connect_in_memory() -> Result<DatabaseConnection> {
    let conn = Database::connect("sqlite::memory:").await?;
    enable_sqlite_foreign_keys(&conn).await?;
    Ok(conn)
}

/// Applies every pending migration to `conn`, creating the `records` table
/// and the `records_fts` FTS5 virtual table on a fresh database.
pub async fn migrate(conn: &DatabaseConnection) -> Result<()> {
    Migrator::up(conn, None).await
}

/// Bridges the synchronous `living_docs_core::store::SearchIndex` port onto
/// this crate's async [`search`]. Holds its own dedicated current-thread
/// Tokio runtime *and* connects through it: `SearchIndex::search` is a sync
/// trait method that may be invoked from a caller with no async context at
/// all, so it cannot assume one exists — and a sea-orm connection is not
/// safe to drive from a Tokio runtime other than the one that created it, so
/// the connection and the runtime that opened it are kept together and
/// never split.
pub struct DbSearchIndex {
    conn: DatabaseConnection,
    runtime: tokio::runtime::Runtime,
}

impl DbSearchIndex {
    /// Opens `url` on a dedicated current-thread runtime and wraps the
    /// resulting connection for synchronous search.
    pub fn new(url: &str) -> io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let conn = runtime.block_on(connect(url)).map_err(io::Error::other)?;
        Ok(Self { conn, runtime })
    }
}

impl SearchIndex for DbSearchIndex {
    fn search(&self, query: &str) -> io::Result<Vec<PathBuf>> {
        let hits = self
            .runtime
            .block_on(search::search(&self.conn, query))
            .map_err(io::Error::other)?;
        Ok(hits
            .into_iter()
            .map(|hit| PathBuf::from(hit.path))
            .collect())
    }
}

/// The project [`DbDocStore::new`] scopes itself to.
const DEFAULT_PROJECT_SLUG: &str = "default";

/// Synchronous `DocStore` adapter over this crate's read-model (ADR 0007,
/// issue 0006 slice 0006-B). Bridges the async SeaORM connection onto the
/// sync `DocStore` port exactly like [`DbSearchIndex`]: a dedicated
/// current-thread runtime owns the connection and every trait method
/// `block_on`s through it, so `DbDocStore` never assumes an ambient async
/// context. Scoped to one project and one `root`, both fixed at
/// construction — `list`/`read` join/strip against `root` rather than the
/// `DocStore` trait's per-call argument, because every record's stored
/// `path` is relative to the one bundle its project was synced from, and
/// `write` is implemented in slice 0006-C; until then it returns an
/// `io::Error` naming the deferral rather than panicking, so a `DbDocStore`
/// built for read-only use (`check`, the future `export`) keeps compiling
/// and working.
pub struct DbDocStore {
    conn: DatabaseConnection,
    runtime: tokio::runtime::Runtime,
    root: PathBuf,
    project_id: i32,
}

impl DbDocStore {
    /// Opens `url` scoped to the `"default"` project rooted at `root`.
    pub fn new(url: &str, root: PathBuf) -> io::Result<Self> {
        Self::for_project(url, root, DEFAULT_PROJECT_SLUG)
    }

    /// Opens `url` scoped to `project_slug`, rooted at `root`. `project_slug`
    /// must already have been synced — this adapter reads an existing
    /// read-model, it does not create a project.
    pub fn for_project(url: &str, root: PathBuf, project_slug: &str) -> io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let conn = runtime.block_on(connect(url)).map_err(io::Error::other)?;
        let project_id = runtime
            .block_on(find_project_id(&conn, project_slug))
            .map_err(io::Error::other)?;
        Ok(Self {
            conn,
            runtime,
            root,
            project_id,
        })
    }
}

async fn find_project_id(conn: &DatabaseConnection, slug: &str) -> Result<i32> {
    Projects::find()
        .filter(ProjectColumn::Slug.eq(slug))
        .one(conn)
        .await?
        .map(|project| project.id)
        .ok_or_else(|| DbErr::Custom(format!("project '{slug}' has not been synced")))
}

impl DocStore for DbDocStore {
    fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
        let paths = self
            .runtime
            .block_on(list_record_paths(&self.conn, self.project_id))
            .map_err(io::Error::other)?;
        Ok(paths.into_iter().map(|path| self.root.join(path)).collect())
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        let relative = path
            .strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        let record = self
            .runtime
            .block_on(load_record(&self.conn, self.project_id, &relative))
            .map_err(io::Error::other)?
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, format!("no record at {relative}"))
            })?;
        Ok(serialize::to_canonical_markdown(&record))
    }

    fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
        Err(io::Error::other(
            "DbDocStore::write is implemented in slice 0006-C",
        ))
    }
}

async fn list_record_paths(conn: &DatabaseConnection, project_id: i32) -> Result<Vec<String>> {
    let records = Records::find()
        .filter(Column::ProjectId.eq(project_id))
        .order_by_asc(Column::Path)
        .all(conn)
        .await?;
    Ok(records.into_iter().map(|record| record.path).collect())
}

/// Assembles the [`ExtractedRecord`] the record at `path` (within
/// `project_id`) would have produced had it been extracted straight from
/// its source markdown: the typed columns, its resolved
/// `supersedes`/`superseded_by` (the related record's zero-padded `number`,
/// ADR 0007 decision 3), its tags sorted for deterministic serialization,
/// and its ordered frontmatter tail.
async fn load_record(
    conn: &DatabaseConnection,
    project_id: i32,
    path: &str,
) -> Result<Option<ExtractedRecord>> {
    let Some(model) = Records::find()
        .filter(Column::ProjectId.eq(project_id))
        .filter(Column::Path.eq(path))
        .one(conn)
        .await?
    else {
        return Ok(None);
    };

    let frontmatter_tail = load_frontmatter_tail(conn, model.id).await?;
    let supersedes = resolve_supersedes(conn, model.id).await?;
    let superseded_by = resolve_superseded_by(conn, model.id).await?;
    let record_tags = load_sorted_tags(conn, model.id).await?;

    Ok(Some(ExtractedRecord {
        doc_type: model.doc_type,
        number: model.number,
        concept_id: model.concept_id,
        identity_kind: model.identity_kind,
        title: model.title,
        description: model.description,
        body: model.body,
        supersedes,
        superseded_by,
        tags: record_tags,
        frontmatter_tail,
    }))
}

async fn load_frontmatter_tail(
    conn: &DatabaseConnection,
    record_id: i32,
) -> Result<Vec<(String, String)>> {
    let rows = frontmatter_fields::Entity::find()
        .filter(frontmatter_fields::Column::RecordId.eq(record_id))
        .order_by_asc(frontmatter_fields::Column::Ordinal)
        .all(conn)
        .await?;
    Ok(rows.into_iter().map(|row| (row.key, row.value)).collect())
}

const SUPERSEDE_RELATION_KIND: &str = "supersede";

/// `record_id`'s `supersedes` edge (this record is the relation's source),
/// resolved to the target record's zero-padded `NNNN` number — the same raw
/// form [`crate::record::extract_record`] parses from frontmatter (ADR 0007
/// decision 3). `None` when no such edge exists, or the target carries no
/// `number`.
async fn resolve_supersedes(conn: &DatabaseConnection, record_id: i32) -> Result<Option<String>> {
    let Some(relation) = relations::Entity::find()
        .filter(relations::Column::Kind.eq(SUPERSEDE_RELATION_KIND))
        .filter(relations::Column::FromRecordId.eq(record_id))
        .one(conn)
        .await?
    else {
        return Ok(None);
    };
    resolve_number(conn, relation.to_record_id).await
}

/// `record_id`'s `superseded_by` edge (this record is the relation's
/// target), resolved the same way as [`resolve_supersedes`] but following
/// the edge in reverse.
async fn resolve_superseded_by(
    conn: &DatabaseConnection,
    record_id: i32,
) -> Result<Option<String>> {
    let Some(relation) = relations::Entity::find()
        .filter(relations::Column::Kind.eq(SUPERSEDE_RELATION_KIND))
        .filter(relations::Column::ToRecordId.eq(record_id))
        .one(conn)
        .await?
    else {
        return Ok(None);
    };
    resolve_number(conn, relation.from_record_id).await
}

async fn resolve_number(conn: &DatabaseConnection, record_id: i32) -> Result<Option<String>> {
    let record = Records::find_by_id(record_id).one(conn).await?;
    Ok(record
        .and_then(|record| record.number)
        .map(|number| format!("{number:04}")))
}

async fn load_sorted_tags(conn: &DatabaseConnection, record_id: i32) -> Result<Vec<String>> {
    let tag_ids: Vec<i32> = record_tags::Entity::find()
        .filter(record_tags::Column::RecordId.eq(record_id))
        .all(conn)
        .await?
        .into_iter()
        .map(|row| row.tag_id)
        .collect();
    let mut names: Vec<String> = tags::Entity::find()
        .filter(tags::Column::Id.is_in(tag_ids))
        .all(conn)
        .await?
        .into_iter()
        .map(|tag| tag.name)
        .collect();
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Statement};

    async fn table_names(conn: &DatabaseConnection) -> Vec<String> {
        let statement = Statement::from_string(
            conn.get_database_backend(),
            "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name".to_owned(),
        );
        conn.query_all(statement)
            .await
            .expect("query sqlite_master for table names")
            .into_iter()
            .map(|row| row.try_get::<String>("", "name").expect("name column"))
            .collect()
    }

    async fn row_count(conn: &DatabaseConnection, table: &str) -> i64 {
        let statement = Statement::from_string(
            conn.get_database_backend(),
            format!("SELECT COUNT(*) AS n FROM {table}"),
        );
        conn.query_one(statement)
            .await
            .expect("query row count")
            .expect("row count query returns exactly one row")
            .try_get::<i64>("", "n")
            .expect("n column")
    }

    #[tokio::test]
    async fn migrate_creates_records_and_records_fts_tables_empty() {
        let conn = connect_in_memory()
            .await
            .expect("connect to in-memory sqlite");
        migrate(&conn).await.expect("apply migration");

        let tables = table_names(&conn).await;
        assert!(
            tables.iter().any(|name| name == "records"),
            "records table missing from sqlite_master: {tables:?}"
        );
        assert!(
            tables.iter().any(|name| name == "records_fts"),
            "records_fts table missing from sqlite_master: {tables:?}"
        );

        assert_eq!(row_count(&conn, "records").await, 0);
        assert_eq!(row_count(&conn, "records_fts").await, 0);
    }

    fn temp_sqlite_url(label: &str) -> (PathBuf, String) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir()
            .join(format!("living-docs-db-store-test-{label}-{nanos}"))
            .join("index.db");
        let url = format!("sqlite://{}?mode=rwc", path.display());
        (path, url)
    }

    #[tokio::test]
    async fn record_by_path_returns_some_for_a_synced_path_and_none_for_an_unknown_path() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");
        let (store, bundle) = sync::test_support::seeded_corpus();
        sync::sync(&conn, &store, &bundle)
            .await
            .expect("sync seeded corpus");

        let found = record_by_path(&conn, "adr/0001-quokka-caching.md")
            .await
            .expect("query record_by_path")
            .expect("record exists for the seeded path");
        assert_eq!(found.title, "Quokka Caching Strategy");
        assert!(found.body.contains("quokka caching strategy"));

        let missing = record_by_path(&conn, "adr/9999-missing.md")
            .await
            .expect("query record_by_path");
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn record_by_path_in_project_scopes_to_one_project_when_two_share_a_path() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");

        let (store_a, bundle_a) =
            sync::test_support::single_record_corpus_at("/bundle-a", "Team A Title");
        sync::sync_project(&conn, &store_a, &bundle_a, "team-a")
            .await
            .expect("sync team-a");
        let (store_b, bundle_b) =
            sync::test_support::single_record_corpus_at("/bundle-b", "Team B Title");
        sync::sync_project(&conn, &store_b, &bundle_b, "team-b")
            .await
            .expect("sync team-b");

        let project_a = entity::projects::Entity::find()
            .filter(entity::projects::Column::Slug.eq("team-a"))
            .one(&conn)
            .await
            .expect("query team-a project")
            .expect("team-a project exists");
        let project_b = entity::projects::Entity::find()
            .filter(entity::projects::Column::Slug.eq("team-b"))
            .one(&conn)
            .await
            .expect("query team-b project")
            .expect("team-b project exists");

        let found_a = record_by_path_in_project(&conn, project_a.id, "adr/0001-quokka-caching.md")
            .await
            .expect("query record_by_path_in_project for team-a")
            .expect("team-a has a record at this path");
        assert_eq!(found_a.title, "Team A Title");

        let found_b = record_by_path_in_project(&conn, project_b.id, "adr/0001-quokka-caching.md")
            .await
            .expect("query record_by_path_in_project for team-b")
            .expect("team-b has a record at this path");
        assert_eq!(found_b.title, "Team B Title");
    }

    #[tokio::test]
    async fn list_projects_returns_every_project_ordered_by_name() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");

        let (store_b, bundle_b) =
            sync::test_support::single_record_corpus_at("/bundle-b", "Team B Title");
        sync::sync_project(&conn, &store_b, &bundle_b, "team-b")
            .await
            .expect("sync team-b");
        let (store_a, bundle_a) =
            sync::test_support::single_record_corpus_at("/bundle-a", "Team A Title");
        sync::sync_project(&conn, &store_a, &bundle_a, "team-a")
            .await
            .expect("sync team-a");

        let projects = list_projects(&conn).await.expect("list projects");

        assert_eq!(
            projects,
            vec![
                ProjectView {
                    slug: "team-a".to_owned(),
                    name: "team-a".to_owned(),
                },
                ProjectView {
                    slug: "team-b".to_owned(),
                    name: "team-b".to_owned(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn list_projects_returns_an_empty_vector_when_no_project_has_synced() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");

        let projects = list_projects(&conn).await.expect("list projects");

        assert!(projects.is_empty());
    }

    #[test]
    fn db_search_index_bridges_the_sync_search_index_port_without_an_ambient_runtime() {
        let (db_path, db_url) = temp_sqlite_url("search-index-bridge");

        let setup_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build setup runtime");
        setup_runtime.block_on(async {
            let conn = connect(&db_url).await.expect("connect");
            migrate(&conn).await.expect("migrate");
            let (store, bundle) = sync::test_support::seeded_corpus();
            sync::sync(&conn, &store, &bundle).await.expect("sync");
        });
        drop(setup_runtime);

        let index = DbSearchIndex::new(&db_url).expect("build search index");
        let hits = index.search("quokka").expect("search should succeed");

        assert_eq!(hits, vec![PathBuf::from("adr/0001-quokka-caching.md")]);

        let no_hits = index
            .search("zzzznomatch")
            .expect("no-match search should succeed");
        assert!(no_hits.is_empty());

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir(db_path.parent().expect("path has a parent"));
    }

    #[tokio::test]
    async fn connect_creates_missing_parent_dirs_for_a_sqlite_file_url() {
        let (db_path, db_url) = temp_sqlite_url("parent-dir-creation");
        assert!(!db_path.parent().expect("path has a parent").exists());

        let conn = connect(&db_url).await.expect("connect creates parent dirs");
        assert_eq!(conn.get_database_backend(), sea_orm::DbBackend::Sqlite);

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir(db_path.parent().expect("path has a parent"));
    }

    #[tokio::test]
    async fn connect_infers_postgres_backend_from_scheme_without_a_live_server() {
        let mut options = sea_orm::ConnectOptions::new("postgres://user:pass@localhost/db");
        options.connect_lazy(true);

        let conn = Database::connect(options)
            .await
            .expect("lazy postgres connect never touches the network");

        assert_eq!(conn.get_database_backend(), sea_orm::DbBackend::Postgres);
    }

    #[tokio::test]
    async fn connect_infers_sqlite_backend_from_a_file_url() {
        let (db_path, db_url) = temp_sqlite_url("backend-inference");

        let conn = connect(&db_url).await.expect("connect");
        assert_eq!(conn.get_database_backend(), sea_orm::DbBackend::Sqlite);

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir(db_path.parent().expect("path has a parent"));
    }
}
