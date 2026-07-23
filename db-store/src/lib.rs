//! `db-store`: the SQLite/FTS5 derived read-model adapter (ADR 0004, issue
//! 0002). Slice S2a landed the schema foundation (connect + migrate). Slice
//! S2b adds the idempotent full-rebuild `sync`, ranked FTS5 `search`, and the
//! `SearchIndex` port implementation. CLI wiring lands in S2c. Slice 0006-B
//! adds the canonical [`serialize`] module and [`DbDocStore`]'s read side.
//! Slice 0006-C2 adds [`DbDocStore::write`] (ADR 0007), completing the
//! `DocStore` port.

pub mod entity;
pub mod migration;
pub mod record;
pub mod search;
pub mod serialize;
pub mod sync;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use living_docs_core::store::{DocStore, SearchIndex};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, Database, DatabaseConnection,
    DatabaseTransaction, DbBackend, DbErr, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};
use sea_orm_migration::MigratorTrait;

use entity::projects::{Column as ProjectColumn, Entity as Projects};
use entity::{record_tags, relations, tags};
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

/// One record's nav-tree entry: enough to group and order the web three-pane
/// shell's sidebar by doc type without a second lookup (issue 0008, ADR
/// 0015, S1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavEntry {
    pub doc_type: String,
    pub number: Option<i32>,
    pub title: String,
    pub path: String,
}

/// A [`NavEntry`]'s raw column projection, used to select `DISTINCT` rows
/// over just the nav-relevant columns rather than the full [`entity::Model`]
/// (whose `project_id` would defeat cross-project dedup).
#[derive(Debug, FromQueryResult)]
struct NavRow {
    doc_type: String,
    number: Option<i32>,
    title: String,
    path: String,
}

/// Lists every distinct record path, spanning every project, ordered by
/// `doc_type` then `number` then `path` — the order the web three-pane
/// shell's nav tree groups and renders by (issue 0008, ADR 0015, S1). The
/// web front's nav links are path-addressed with no project dimension
/// (`/record/<path>`, issue 0005), so two projects synced from identical
/// bundles must collapse to one [`NavEntry`] per distinct `path` here —
/// project-scoped nav is an explicitly deferred follow-up (issue 0008, ADR
/// 0015, S2 browser-gate finding).
pub async fn records_by_type(conn: &DatabaseConnection) -> Result<Vec<NavEntry>> {
    let rows = Records::find()
        .filter(Column::DeletedAt.is_null())
        .select_only()
        .column(Column::DocType)
        .column(Column::Number)
        .column(Column::Title)
        .column(Column::Path)
        .distinct()
        .order_by_asc(Column::DocType)
        .order_by_asc(Column::Number)
        .order_by_asc(Column::Path)
        .into_model::<NavRow>()
        .all(conn)
        .await?;
    Ok(rows.into_iter().map(nav_row_to_nav_entry).collect())
}

fn nav_row_to_nav_entry(row: NavRow) -> NavEntry {
    NavEntry {
        doc_type: row.doc_type,
        number: row.number,
        title: row.title,
        path: row.path,
    }
}

/// A record related to another by a supersede edge, carrying the target's
/// path and title rather than its raw `NNNN` number — the shape the web
/// front's supersede-chain link needs to render a clickable reference
/// (issue 0008, ADR 0015, S1), unlike [`resolve_supersedes`]/
/// [`resolve_superseded_by`] which resolve to the raw frontmatter number for
/// [`load_record`]'s canonical round trip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelatedRef {
    pub path: String,
    pub title: String,
}

/// A record's metadata for the web three-pane shell's detail pane: its doc
/// type, status, both supersede directions (as path+title link targets),
/// its tags (issue 0008, ADR 0015, S1), its optimistic-concurrency
/// `revision` (ADR 0016, issue 0011), and its soft-delete marker
/// (ADR 0018, issue 0013 slice A) — `None` for a live record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordMeta {
    pub doc_type: String,
    pub status: Option<String>,
    pub supersedes: Vec<RelatedRef>,
    pub superseded_by: Vec<RelatedRef>,
    pub tags: Vec<String>,
    pub revision: i64,
    pub deleted_at: Option<i64>,
}

/// Looks up `path`'s metadata, spanning every project like [`record_by_path`]
/// (kept unscoped for the web front, which is not project-aware until issue
/// 0005 slice 0005-C). Returns `None` when no record exists at that path,
/// never an error.
pub async fn record_meta(conn: &DatabaseConnection, path: &str) -> Result<Option<RecordMeta>> {
    let Some(model) = Records::find()
        .filter(Column::Path.eq(path))
        .one(conn)
        .await?
    else {
        return Ok(None);
    };

    let supersedes = related_refs_from(conn, model.id).await?;
    let superseded_by = related_refs_to(conn, model.id).await?;
    let tags = load_sorted_tags(conn, model.id).await?;

    Ok(Some(RecordMeta {
        doc_type: model.doc_type,
        status: model.status,
        supersedes,
        superseded_by,
        tags,
        revision: model.revision,
        deleted_at: model.deleted_at,
    }))
}

/// `record_id`'s outgoing supersede edges (this record supersedes the
/// targets), resolved to each target's path+title.
async fn related_refs_from(conn: &DatabaseConnection, record_id: i32) -> Result<Vec<RelatedRef>> {
    let target_ids: Vec<i32> = relations::Entity::find()
        .filter(relations::Column::Kind.eq(SUPERSEDE_RELATION_KIND))
        .filter(relations::Column::FromRecordId.eq(record_id))
        .all(conn)
        .await?
        .into_iter()
        .map(|relation| relation.to_record_id)
        .collect();
    related_refs_for_ids(conn, &target_ids).await
}

/// `record_id`'s incoming supersede edges (this record is superseded by the
/// sources), resolved to each source's path+title.
async fn related_refs_to(conn: &DatabaseConnection, record_id: i32) -> Result<Vec<RelatedRef>> {
    let source_ids: Vec<i32> = relations::Entity::find()
        .filter(relations::Column::Kind.eq(SUPERSEDE_RELATION_KIND))
        .filter(relations::Column::ToRecordId.eq(record_id))
        .all(conn)
        .await?
        .into_iter()
        .map(|relation| relation.from_record_id)
        .collect();
    related_refs_for_ids(conn, &source_ids).await
}

async fn related_refs_for_ids(conn: &DatabaseConnection, ids: &[i32]) -> Result<Vec<RelatedRef>> {
    let records = Records::find()
        .filter(Column::Id.is_in(ids.to_owned()))
        .all(conn)
        .await?;
    Ok(records
        .into_iter()
        .map(|record| RelatedRef {
            path: record.path,
            title: record.title,
        })
        .collect())
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
/// issue 0006 slices 0006-B/0006-C2). Bridges the async SeaORM connection
/// onto the sync `DocStore` port exactly like [`DbSearchIndex`]: a dedicated
/// current-thread runtime owns the connection and every trait method
/// `block_on`s through it, so `DbDocStore` never assumes an ambient async
/// context. Scoped to one project and one `root`, both fixed at
/// construction — `list`/`read`/`write` join/strip against `root` rather
/// than the `DocStore` trait's per-call argument, because every record's
/// stored `path` is relative to the one bundle its project was synced from.
/// `write` parses the canonical markdown it is given, upserts the record by
/// `(project_id, path)`, and best-effort resolves its relations/tags
/// against the project's already-persisted records.
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

    /// `path` relative to this store's `root`, the identity `read`/`write`
    /// key every record by (project-relative, never bundle-joined).
    fn relative_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }

    /// Inserts a brand-new record at `path` and commits only if
    /// `living_docs_core::check::check_violations` finds nothing wrong with
    /// the project's resulting state — the transactional write+check verb
    /// ADR 0016/issue 0010 slice 2 gives both `new --backend db` and, in a
    /// later slice, Atlas's create route. The insert, the in-transaction
    /// snapshot, and the check all run inside one transaction: a failing
    /// check rolls the insert back, so no invalid record is ever visible.
    /// Before `check` runs, the affected type's on-disk `index.md` is
    /// regenerated from that same in-transaction snapshot (issue 0010 slice
    /// 2b) — `check_directory_membership` always reads `index.md` straight
    /// off disk (ADR 0007: it is a reserved fs-only presentation artifact),
    /// so without this the new record is always reported as an orphan. The
    /// DB transaction and this filesystem write are not cross-engine atomic;
    /// [`commit_or_restore`] restores the pre-call `index.md` content
    /// whenever the transaction itself does not end up committing. Every
    /// freshly inserted record's `revision` defaults to `1` (the migration's
    /// column default), so a successful commit always returns `1` — this
    /// verb is create-only, never a revision bump.
    pub fn write_checked(
        &self,
        path: &Path,
        contents: &str,
    ) -> std::result::Result<i64, WriteCheckedError> {
        let relative = self.relative_path(path);
        let extracted = record::extract_record(Path::new(&relative), contents);
        validate_identity(&relative, &extracted).map_err(WriteCheckedError::InvalidInput)?;
        let doc_type = doc_type_from_path(path)?;

        self.runtime.block_on(async {
            let txn = self.conn.begin().await.map_err(WriteCheckedError::Db)?;
            ensure_absent(&txn, self.project_id, &relative).await?;
            sync::insert_new_record(&txn, self.project_id, &relative, &extracted)
                .await
                .map_err(WriteCheckedError::Db)?;
            let snapshot = materialize_snapshot(&txn, self.project_id, &self.root)
                .await
                .map_err(WriteCheckedError::Db)?;
            let store = SnapshotDocStore { records: snapshot };

            let index_write = match write_regenerated_index(&store, &self.root, doc_type) {
                Ok(index_write) => index_write,
                Err(err) => {
                    txn.rollback().await.map_err(WriteCheckedError::Db)?;
                    return Err(err);
                }
            };

            let violations = living_docs_core::check::check_violations(&store, &self.root);
            commit_or_restore(txn, violations, index_write).await
        })
    }

    /// Replaces the content of the record already at `path`, gated on the
    /// same in-transaction `check_violations` [`write_checked`]
    /// (DbDocStore::write_checked) uses — the optimistic-concurrency edit
    /// verb ADR 0016 decision 3 and issue 0011 add on top of issue 0010's
    /// create-only path. Fails with [`WriteCheckedError::NotFound`] when no
    /// record exists at `path`. When `base_revision` is `Some`, the stored
    /// `revision` must still equal it or the call is rejected with
    /// [`WriteCheckedError::StaleRevision`] and nothing is written — ADR
    /// 0016's "changed underneath you — reload", never a merge. On success
    /// the stored `revision` is bumped by one and returned.
    pub fn update_checked(
        &self,
        path: &Path,
        contents: &str,
        base_revision: Option<i64>,
    ) -> std::result::Result<i64, WriteCheckedError> {
        let relative = self.relative_path(path);
        let extracted = record::extract_record(Path::new(&relative), contents);
        validate_identity(&relative, &extracted).map_err(WriteCheckedError::InvalidInput)?;
        let doc_type = doc_type_from_path(path)?;

        self.runtime.block_on(async {
            let txn = self.conn.begin().await.map_err(WriteCheckedError::Db)?;
            let existing =
                ensure_present_and_current(&txn, self.project_id, &relative, base_revision).await?;
            let new_revision = existing.revision + 1;
            sync::update_existing_record(
                &txn,
                self.project_id,
                &relative,
                existing.id,
                &extracted,
                new_revision,
            )
            .await
            .map_err(WriteCheckedError::Db)?;
            let snapshot = materialize_snapshot(&txn, self.project_id, &self.root)
                .await
                .map_err(WriteCheckedError::Db)?;
            let store = SnapshotDocStore { records: snapshot };

            let index_write = match write_regenerated_index(&store, &self.root, doc_type) {
                Ok(index_write) => index_write,
                Err(err) => {
                    txn.rollback().await.map_err(WriteCheckedError::Db)?;
                    return Err(err);
                }
            };

            let violations = living_docs_core::check::check_violations(&store, &self.root);
            commit_or_restore(txn, violations, index_write)
                .await
                .map(|_| new_revision)
        })
    }

    /// Looks up the record at `path` and returns its canonical markdown
    /// alongside its current `revision` — the pair Atlas's edit form needs
    /// to pre-fill a `base_revision` for [`update_checked`]'s optimistic-
    /// concurrency precondition (ADR 0016, issue 0011). `Err` with
    /// [`io::ErrorKind::NotFound`] when no record exists at `path`.
    pub fn read_with_revision(&self, path: &Path) -> io::Result<(String, i64)> {
        let relative = self.relative_path(path);
        self.runtime.block_on(async {
            let record = load_record(&self.conn, self.project_id, &relative)
                .await
                .map_err(io::Error::other)?
                .ok_or_else(|| record_not_found(&relative))?;
            let revision = sync::find_record(&self.conn, self.project_id, &relative)
                .await
                .map_err(io::Error::other)?
                .ok_or_else(|| record_not_found(&relative))?
                .revision;
            Ok((serialize::to_canonical_markdown(&record), revision))
        })
    }

    /// Applies [`living_docs_core::commands::supersede::supersede`] — the
    /// exact logic `living-docs supersede` uses, unmodified — against an
    /// in-transaction [`MutableSnapshotDocStore`] staged from the project's
    /// already-persisted records, then commits every record it mutated
    /// through the same revision-aware [`sync::update_existing_record`]
    /// [`update_checked`](Self::update_checked) uses, gated on
    /// `living_docs_core::check::check_violations` inside one transaction —
    /// Atlas's checked supersede verb (ADR 0016, issue 0012). `old`/`new`
    /// are bare record numbers, resolved by `supersede` itself exactly as
    /// the CLI's own argument shape resolves them. A resolution failure (an
    /// unknown number) rolls back before anything is written and surfaces
    /// as [`SupersedeCheckedError::ResolutionFailed`] carrying the CLI's own
    /// message; a failing `check` rolls back and restores every regenerated
    /// `index.md` (see [`commit_or_restore_many`]). The CLI's own unchecked
    /// `--backend db` supersede path (`commands::supersede::run`) is
    /// untouched by this method — it keeps writing through
    /// [`DbDocStore::write`] directly.
    pub fn supersede_checked(
        &self,
        old: &str,
        new: &str,
    ) -> std::result::Result<(), SupersedeCheckedError> {
        self.runtime.block_on(async {
            let txn = self.conn.begin().await?;
            let pre_state = materialize_snapshot(&txn, self.project_id, &self.root).await?;
            let staging = MutableSnapshotDocStore::new(pre_state.clone());

            if let Err(message) =
                living_docs_core::commands::supersede::supersede(&staging, &self.root, old, new)
            {
                txn.rollback().await?;
                return Err(SupersedeCheckedError::ResolutionFailed(message));
            }

            let changed = changed_paths(&pre_state, &staging.into_records());
            for (path, content) in &changed {
                let relative = self.relative_path(path);
                apply_changed_record(&txn, self.project_id, &relative, content).await?;
            }

            let post_snapshot = materialize_snapshot(&txn, self.project_id, &self.root).await?;
            let post_store = SnapshotDocStore {
                records: post_snapshot,
            };

            let index_writes = match regenerate_indices(&post_store, &self.root, &changed) {
                Ok(index_writes) => index_writes,
                Err(err) => {
                    txn.rollback().await?;
                    return Err(err);
                }
            };

            let violations = living_docs_core::check::check_violations(&post_store, &self.root);
            commit_or_restore_many(txn, violations, index_writes).await
        })
    }

    /// Soft-deletes the record at `path` — Atlas's delete verb (ADR 0018,
    /// issue 0013 slice A). Refuses with [`DeleteCheckedError::NotFound`]
    /// when no record exists at `path`, with
    /// [`DeleteCheckedError::IneligibleType`] when its doc type is not in
    /// [`DELETE_ELIGIBLE_TYPES`], and with
    /// [`DeleteCheckedError::HasInboundRelations`] when another record still
    /// points at it. On success, `deleted_at` is set to the current time
    /// inside the same transaction that regenerates the affected type's
    /// `index.md` from a snapshot that already excludes the just-deleted
    /// record (see [`list_record_paths`]) and gates the commit on
    /// `living_docs_core::check::check_violations`, exactly like
    /// [`write_checked`](Self::write_checked)/[`update_checked`](Self::update_checked).
    pub fn delete_checked(&self, path: &Path) -> std::result::Result<(), DeleteCheckedError> {
        let relative = self.relative_path(path);
        let doc_type = doc_type_from_path(path).map_err(to_delete_index_io)?;

        self.runtime.block_on(async {
            let txn = self.conn.begin().await.map_err(DeleteCheckedError::Db)?;
            let existing = ensure_delete_eligible(&txn, self.project_id, &relative).await?;
            ensure_no_inbound_relations(&txn, existing.id, &relative).await?;
            mark_deleted(&txn, existing.id)
                .await
                .map_err(DeleteCheckedError::Db)?;

            let snapshot = materialize_snapshot(&txn, self.project_id, &self.root)
                .await
                .map_err(DeleteCheckedError::Db)?;
            let store = SnapshotDocStore { records: snapshot };

            let index_write = match write_regenerated_index(&store, &self.root, doc_type) {
                Ok(index_write) => index_write,
                Err(err) => {
                    txn.rollback().await.map_err(DeleteCheckedError::Db)?;
                    return Err(to_delete_index_io(err));
                }
            };

            let violations = living_docs_core::check::check_violations(&store, &self.root);
            commit_or_restore_delete(txn, violations, index_write).await
        })
    }
}

/// A mutable, in-memory [`DocStore`] staging area seeded from a project's
/// already-persisted records — the mutable counterpart to the read-only
/// [`SnapshotDocStore`], letting
/// [`living_docs_core::commands::supersede::supersede`] run its own
/// read-modify-write against an in-transaction snapshot exactly as it would
/// against any other `DocStore`, so
/// [`supersede_checked`](DbDocStore::supersede_checked) reuses the CLI's own
/// supersede logic unmodified rather than reimplementing it (issue 0012).
struct MutableSnapshotDocStore {
    records: RefCell<BTreeMap<PathBuf, String>>,
}

impl MutableSnapshotDocStore {
    fn new(records: BTreeMap<PathBuf, String>) -> Self {
        Self {
            records: RefCell::new(records),
        }
    }

    /// Consumes the store, handing back its final (possibly mutated) map —
    /// [`supersede_checked`](DbDocStore::supersede_checked)'s post-mutation
    /// state to diff against the pre-call snapshot.
    fn into_records(self) -> BTreeMap<PathBuf, String> {
        self.records.into_inner()
    }
}

impl DocStore for MutableSnapshotDocStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(self
            .records
            .borrow()
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect())
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.records.borrow().get(path).cloned().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no record at {}", path.display()),
            )
        })
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        self.records
            .borrow_mut()
            .insert(path.to_path_buf(), contents.to_owned());
        Ok(())
    }
}

/// Every root-joined path in `post` whose content differs from (or is
/// absent from) `pre`, paired with its new content —
/// [`supersede_checked`](DbDocStore::supersede_checked)'s diff between the
/// staging store's pre-call snapshot and its post-`supersede` state, so only
/// the records `supersede` actually touched are written back through
/// [`sync::update_existing_record`].
fn changed_paths(
    pre: &BTreeMap<PathBuf, String>,
    post: &BTreeMap<PathBuf, String>,
) -> Vec<(PathBuf, String)> {
    post.iter()
        .filter(|(path, content)| pre.get(*path) != Some(*content))
        .map(|(path, content)| (path.clone(), content.clone()))
        .collect()
}

/// Writes one [`changed_paths`] entry back through the same revision-aware
/// [`sync::update_existing_record`]
/// [`update_checked`](DbDocStore::update_checked) uses — kept out of
/// [`supersede_checked`](DbDocStore::supersede_checked)'s body so that
/// method stays a flat sequence of steps.
async fn apply_changed_record<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    relative: &str,
    contents: &str,
) -> std::result::Result<(), SupersedeCheckedError> {
    let extracted = record::extract_record(Path::new(relative), contents);
    let existing = sync::find_record(conn, project_id, relative)
        .await?
        .ok_or_else(|| {
            SupersedeCheckedError::ResolutionFailed(format!("{relative}: no record at that path"))
        })?;
    sync::update_existing_record(
        conn,
        project_id,
        relative,
        existing.id,
        &extracted,
        existing.revision + 1,
    )
    .await
    .map_err(SupersedeCheckedError::from)
}

/// Regenerates `index.md` for every distinct doc type among `changed`'s
/// paths, from `store`'s in-transaction snapshot — the multi-record sibling
/// of [`write_regenerated_index`],
/// [`supersede_checked`](DbDocStore::supersede_checked)'s own index-write
/// step, since a supersede spans two records that may belong to different
/// doc types.
fn regenerate_indices(
    store: &SnapshotDocStore,
    docs_dir: &Path,
    changed: &[(PathBuf, String)],
) -> std::result::Result<Vec<(PathBuf, Option<String>)>, SupersedeCheckedError> {
    let mut doc_types: Vec<&'static str> = Vec::new();
    for (path, _) in changed {
        let doc_type = doc_type_from_path(path)
            .map_err(|err| SupersedeCheckedError::IndexIo(io::Error::other(err.to_string())))?;
        if !doc_types.contains(&doc_type) {
            doc_types.push(doc_type);
        }
    }

    doc_types
        .into_iter()
        .map(|doc_type| {
            write_regenerated_index(store, docs_dir, doc_type)
                .map_err(|err| SupersedeCheckedError::IndexIo(io::Error::other(err.to_string())))
        })
        .collect()
}

/// [`commit_or_restore`]'s sibling for
/// [`supersede_checked`](DbDocStore::supersede_checked): commits `txn` when
/// `violations` is empty, otherwise rolls back and restores every entry in
/// `index_writes` (there may be up to two, one per doc type a supersede
/// touched) to its pre-call content. Kept separate from `commit_or_restore`
/// rather than generalizing it, since that function's single-index,
/// revision-returning shape is relied on unchanged by
/// [`write_checked`](DbDocStore::write_checked) and
/// [`update_checked`](DbDocStore::update_checked).
async fn commit_or_restore_many(
    txn: DatabaseTransaction,
    violations: Vec<(String, String)>,
    index_writes: Vec<(PathBuf, Option<String>)>,
) -> std::result::Result<(), SupersedeCheckedError> {
    if !violations.is_empty() {
        txn.rollback().await?;
        for (index_path, original) in index_writes {
            restore_index(&index_path, original);
        }
        return Err(SupersedeCheckedError::CheckFailed(violations));
    }
    txn.commit().await?;
    Ok(())
}

/// Fails with [`WriteCheckedError::NotFound`] when no record exists at
/// `path`; otherwise, when `base_revision` is `Some` and does not match the
/// stored `revision`, fails with [`WriteCheckedError::StaleRevision`] —
/// [`DbDocStore::update_checked`]'s single existence-and-currency
/// precondition, kept out of that method's body to hold it to a flat
/// sequence of steps.
async fn ensure_present_and_current<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    path: &str,
    base_revision: Option<i64>,
) -> std::result::Result<entity::Model, WriteCheckedError> {
    let existing = sync::find_record(conn, project_id, path)
        .await
        .map_err(WriteCheckedError::Db)?
        .ok_or_else(|| WriteCheckedError::NotFound(path.to_owned()))?;
    match base_revision {
        Some(expected) if expected != existing.revision => Err(WriteCheckedError::StaleRevision {
            path: path.to_owned(),
            expected,
            actual: existing.revision,
        }),
        _ => Ok(existing),
    }
}

/// Derives `path`'s doc-type token from its parent directory name (the
/// reverse of [`living_docs_core::paths::dir_for`]) so
/// [`DbDocStore::write_checked`] knows which type's `index.md` to
/// regenerate. Defensive: `commands::new::plan` should never produce a path
/// outside a known type directory, so this only fails on a caller-supplied
/// `path` that does not follow that shape.
fn doc_type_from_path(path: &Path) -> std::result::Result<&'static str, WriteCheckedError> {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .and_then(living_docs_core::paths::doc_type_for_dir)
        .ok_or_else(|| {
            WriteCheckedError::InvalidInput(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "{}: cannot derive a doc type from its parent directory",
                    path.display()
                ),
            ))
        })
}

/// Regenerates `doc_type`'s `index.md` from `store`'s in-transaction
/// snapshot and writes it to disk, returning the index's path and its
/// pre-call content (`None` when it did not already exist) so the caller can
/// restore it if the transaction that produced `store` does not end up
/// committing. Kept out of
/// [`write_checked`](DbDocStore::write_checked)'s body so that method stays
/// a flat sequence of steps.
fn write_regenerated_index(
    store: &SnapshotDocStore,
    docs_dir: &Path,
    doc_type: &str,
) -> std::result::Result<(PathBuf, Option<String>), WriteCheckedError> {
    let (index_path, content) =
        living_docs_core::commands::index::compute(store, docs_dir, doc_type, None)
            .map_err(|message| WriteCheckedError::IndexIo(io::Error::other(message)))?;
    let original = fs::read_to_string(&index_path).ok();
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent).map_err(WriteCheckedError::IndexIo)?;
    }
    fs::write(&index_path, &content).map_err(WriteCheckedError::IndexIo)?;
    Ok((index_path, original))
}

/// Restores `index_path` to `original`'s content, or removes it entirely
/// when `original` is `None` (it did not exist before
/// [`write_regenerated_index`] wrote it) — best-effort, since this runs
/// while already unwinding a failed [`DbDocStore::write_checked`] call and
/// has no further error path to report through.
fn restore_index(index_path: &Path, original: Option<String>) {
    match original {
        Some(content) => {
            let _ = fs::write(index_path, content);
        }
        None => {
            let _ = fs::remove_file(index_path);
        }
    }
}

/// [`commit_or_rollback`], extended to restore the type's `index.md` to its
/// pre-call state whenever the transaction does not end up committing —
/// [`write_checked`](DbDocStore::write_checked)'s single point of
/// commit-vs-rollback-and-restore decision, so that method's body stays a
/// flat sequence of steps.
async fn commit_or_restore(
    txn: DatabaseTransaction,
    violations: Vec<(String, String)>,
    index_write: (PathBuf, Option<String>),
) -> std::result::Result<i64, WriteCheckedError> {
    let (index_path, original_index) = index_write;
    match commit_or_rollback(txn, violations).await {
        Ok(revision) => Ok(revision),
        Err(err) => {
            restore_index(&index_path, original_index);
            Err(err)
        }
    }
}

/// Fails with [`WriteCheckedError::AlreadyExists`] when `path` already has a
/// record in `project_id` — [`write_checked`](DbDocStore::write_checked) is
/// create-only, so it never falls back to an update the way
/// [`sync::upsert_record`] does.
async fn ensure_absent<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    path: &str,
) -> std::result::Result<(), WriteCheckedError> {
    let existing = sync::find_record(conn, project_id, path)
        .await
        .map_err(WriteCheckedError::Db)?;
    match existing {
        Some(_) => Err(WriteCheckedError::AlreadyExists(path.to_owned())),
        None => Ok(()),
    }
}

/// Commits `txn` and returns the freshly inserted record's `revision` (always
/// `1`) when `violations` is empty, otherwise rolls `txn` back and returns
/// [`WriteCheckedError::CheckFailed`] — the single commit-or-rollback
/// decision [`write_checked`](DbDocStore::write_checked) delegates to, kept
/// out of that method's body to hold it to a flat sequence of steps.
async fn commit_or_rollback(
    txn: DatabaseTransaction,
    violations: Vec<(String, String)>,
) -> std::result::Result<i64, WriteCheckedError> {
    if !violations.is_empty() {
        txn.rollback().await.map_err(WriteCheckedError::Db)?;
        return Err(WriteCheckedError::CheckFailed(violations));
    }
    txn.commit().await.map_err(WriteCheckedError::Db)?;
    Ok(1)
}

/// A read-only [`DocStore`] snapshot of one project's already-persisted
/// records plus the one record a not-yet-committed
/// [`write_checked`](DbDocStore::write_checked) call just inserted —
/// materialized in memory so `living_docs_core::check::check_violations` can
/// validate the project's resulting state before the transaction commits,
/// with no visibility into the uncommitted row from any other connection.
/// `write` is unsupported: `check` never writes, and nothing else should.
struct SnapshotDocStore {
    records: BTreeMap<PathBuf, String>,
}

impl DocStore for SnapshotDocStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(self
            .records
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect())
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.records.get(path).cloned().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no record at {}", path.display()),
            )
        })
    }

    fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "SnapshotDocStore is read-only",
        ))
    }
}

/// Renders `project_id`'s every record (as it stands within `conn`, which may
/// be an in-flight transaction that has not committed yet) back into its
/// canonical markdown, keyed by its `root`-joined path — the corpus
/// [`SnapshotDocStore`] serves to `check_violations` from inside
/// [`write_checked`](DbDocStore::write_checked)'s transaction.
async fn materialize_snapshot<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    root: &Path,
) -> Result<BTreeMap<PathBuf, String>> {
    let mut records = BTreeMap::new();
    for relative in list_record_paths(conn, project_id).await? {
        if let Some(record) = load_record(conn, project_id, &relative).await? {
            records.insert(
                root.join(&relative),
                serialize::to_canonical_markdown(&record),
            );
        }
    }
    Ok(records)
}

/// Every way [`DbDocStore::write_checked`] or [`DbDocStore::update_checked`]
/// can fail to commit: invalid input (the same identity-shape errors
/// [`DbDocStore::write`] refuses), a record already at that path
/// (`write_checked` only), no record at that path (`update_checked` only),
/// a `base_revision` that no longer matches the stored `revision`
/// (`update_checked` only, ADR 0016's optimistic-concurrency precondition),
/// a failing `check`, a database error, or an I/O failure regenerating the
/// affected type's `index.md`.
#[derive(Debug)]
pub enum WriteCheckedError {
    InvalidInput(io::Error),
    AlreadyExists(String),
    NotFound(String),
    StaleRevision {
        path: String,
        expected: i64,
        actual: i64,
    },
    CheckFailed(Vec<(String, String)>),
    Db(DbErr),
    IndexIo(io::Error),
}

impl std::fmt::Display for WriteCheckedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteCheckedError::InvalidInput(err) => write!(f, "{err}"),
            WriteCheckedError::AlreadyExists(path) => write!(f, "{path} already exists"),
            WriteCheckedError::NotFound(path) => write!(f, "{path}: no record at that path"),
            WriteCheckedError::StaleRevision {
                path,
                expected,
                actual,
            } => write!(
                f,
                "{path}: changed underneath you — reload (expected revision {expected}, \
                 stored revision {actual})"
            ),
            WriteCheckedError::CheckFailed(violations) => {
                write!(f, "check failed: {}", format_violations(violations))
            }
            WriteCheckedError::Db(err) => write!(f, "{err}"),
            WriteCheckedError::IndexIo(err) => write!(f, "regenerating index.md: {err}"),
        }
    }
}

fn format_violations(violations: &[(String, String)]) -> String {
    violations
        .iter()
        .map(|(file, message)| format!("{file}: {message}"))
        .collect::<Vec<_>>()
        .join("; ")
}

impl std::error::Error for WriteCheckedError {}

/// Every way [`DbDocStore::supersede_checked`] can fail to commit: `old` or
/// `new` did not resolve to a record (the CLI's own `supersede`'s message,
/// unmodified), a failing `check`, a database error, or an I/O failure
/// regenerating a touched doc type's `index.md`.
#[derive(Debug)]
pub enum SupersedeCheckedError {
    ResolutionFailed(String),
    CheckFailed(Vec<(String, String)>),
    Db(DbErr),
    IndexIo(io::Error),
}

impl std::fmt::Display for SupersedeCheckedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupersedeCheckedError::ResolutionFailed(message) => write!(f, "{message}"),
            SupersedeCheckedError::CheckFailed(violations) => {
                write!(f, "check failed: {}", format_violations(violations))
            }
            SupersedeCheckedError::Db(err) => write!(f, "{err}"),
            SupersedeCheckedError::IndexIo(err) => write!(f, "regenerating index.md: {err}"),
        }
    }
}

impl std::error::Error for SupersedeCheckedError {}

impl From<DbErr> for SupersedeCheckedError {
    fn from(err: DbErr) -> Self {
        SupersedeCheckedError::Db(err)
    }
}

/// The doc types [`DbDocStore::delete_checked`] will act on (ADR 0018
/// decision 1). ADR 0018 also names OKF `concept` as delete-eligible, but
/// `concept` has no [`living_docs_core::paths::dir_for`]/`doc_type_for_dir`
/// mapping — it is not a doc type modeled anywhere in this codebase today —
/// so this slice scopes eligibility to exactly what already exists as a
/// real record type. Extending to `concept` is out of scope until a future
/// issue introduces it as a modeled doc type. Compared case-insensitively
/// against a record's stored `doc_type`, which carries the frontmatter
/// `type:` value's original casing (e.g. `"Issue"`), not this lowercase CLI
/// token.
const DELETE_ELIGIBLE_TYPES: [&str; 1] = ["issue"];

/// Every way [`DbDocStore::delete_checked`] can fail to commit: no record at
/// that path, the record's doc type is not in [`DELETE_ELIGIBLE_TYPES`],
/// another record's `relations` row still points at it, a failing `check`,
/// a database error, or an I/O failure regenerating the affected type's
/// `index.md`.
#[derive(Debug)]
pub enum DeleteCheckedError {
    NotFound(String),
    IneligibleType { path: String, doc_type: String },
    HasInboundRelations(String),
    CheckFailed(Vec<(String, String)>),
    Db(DbErr),
    IndexIo(io::Error),
}

impl std::fmt::Display for DeleteCheckedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteCheckedError::NotFound(path) => write!(f, "{path}: no record at that path"),
            DeleteCheckedError::IneligibleType { path, doc_type } => write!(
                f,
                "{path}: doc type '{doc_type}' is not eligible for delete"
            ),
            DeleteCheckedError::HasInboundRelations(path) => write!(
                f,
                "{path}: cannot delete a record another record still refers to"
            ),
            DeleteCheckedError::CheckFailed(violations) => {
                write!(f, "check failed: {}", format_violations(violations))
            }
            DeleteCheckedError::Db(err) => write!(f, "{err}"),
            DeleteCheckedError::IndexIo(err) => write!(f, "regenerating index.md: {err}"),
        }
    }
}

impl std::error::Error for DeleteCheckedError {}

/// Maps any [`WriteCheckedError`] [`doc_type_from_path`]/
/// [`write_regenerated_index`] can return into
/// [`DeleteCheckedError::IndexIo`] — the same catch-all
/// [`regenerate_indices`] already uses to bridge those two shared helpers
/// into a caller-specific error type, since in practice both only ever
/// return their own `IndexIo`/`InvalidInput` I/O variant.
fn to_delete_index_io(err: WriteCheckedError) -> DeleteCheckedError {
    DeleteCheckedError::IndexIo(io::Error::other(err.to_string()))
}

/// Fails with [`DeleteCheckedError::NotFound`] when no record exists at
/// `path`; with [`DeleteCheckedError::IneligibleType`] when its doc type is
/// not in [`DELETE_ELIGIBLE_TYPES`] — [`DbDocStore::delete_checked`]'s
/// single existence-and-eligibility precondition, kept out of that method's
/// body to hold it to a flat sequence of steps.
async fn ensure_delete_eligible<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    path: &str,
) -> std::result::Result<entity::Model, DeleteCheckedError> {
    let existing = sync::find_record(conn, project_id, path)
        .await
        .map_err(DeleteCheckedError::Db)?
        .ok_or_else(|| DeleteCheckedError::NotFound(path.to_owned()))?;
    let doc_type_lower = existing.doc_type.to_lowercase();
    if !DELETE_ELIGIBLE_TYPES.contains(&doc_type_lower.as_str()) {
        return Err(DeleteCheckedError::IneligibleType {
            path: path.to_owned(),
            doc_type: existing.doc_type,
        });
    }
    Ok(existing)
}

/// Fails with [`DeleteCheckedError::HasInboundRelations`] when any
/// `relations` row targets `record_id`, regardless of `kind` — unlike
/// [`related_refs_to`], which only looks at supersede edges, a delete must
/// be refused by *any* inbound reference (ADR 0018 decision 3).
async fn ensure_no_inbound_relations<C: ConnectionTrait>(
    conn: &C,
    record_id: i32,
    path: &str,
) -> std::result::Result<(), DeleteCheckedError> {
    let has_inbound = relations::Entity::find()
        .filter(relations::Column::ToRecordId.eq(record_id))
        .one(conn)
        .await
        .map_err(DeleteCheckedError::Db)?
        .is_some();
    if has_inbound {
        return Err(DeleteCheckedError::HasInboundRelations(path.to_owned()));
    }
    Ok(())
}

/// Sets `record_id`'s `deleted_at` to the current Unix time — the soft-
/// delete marker [`DbDocStore::delete_checked`] applies inside its own
/// transaction (ADR 0018 decision 2).
async fn mark_deleted<C: ConnectionTrait>(conn: &C, record_id: i32) -> Result<()> {
    let model = entity::ActiveModel {
        id: ActiveValue::Set(record_id),
        deleted_at: ActiveValue::Set(Some(current_unix_seconds())),
        ..Default::default()
    };
    model.update(conn).await.map(|_| ())
}

fn current_unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

/// [`commit_or_restore`]'s sibling for [`DbDocStore::delete_checked`]:
/// commits `txn` and restores nothing when `violations` is empty; otherwise
/// rolls back and restores `index_write` to its pre-call content — inlined
/// rather than reusing [`commit_or_restore`]/[`commit_or_rollback`], which
/// are relied on unchanged by
/// [`write_checked`](DbDocStore::write_checked)/
/// [`update_checked`](DbDocStore::update_checked) and return a `revision`
/// this verb has none of.
async fn commit_or_restore_delete(
    txn: DatabaseTransaction,
    violations: Vec<(String, String)>,
    index_write: (PathBuf, Option<String>),
) -> std::result::Result<(), DeleteCheckedError> {
    let (index_path, original_index) = index_write;
    if !violations.is_empty() {
        txn.rollback().await.map_err(DeleteCheckedError::Db)?;
        restore_index(&index_path, original_index);
        return Err(DeleteCheckedError::CheckFailed(violations));
    }
    txn.commit().await.map_err(DeleteCheckedError::Db)?;
    Ok(())
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
        let relative = self.relative_path(path);
        let record = self
            .runtime
            .block_on(load_record(&self.conn, self.project_id, &relative))
            .map_err(io::Error::other)?
            .ok_or_else(|| record_not_found(&relative))?;
        Ok(serialize::to_canonical_markdown(&record))
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        let relative = self.relative_path(path);
        let extracted = record::extract_record(Path::new(&relative), contents);
        validate_identity(&relative, &extracted)?;
        self.runtime
            .block_on(sync::upsert_record(
                &self.conn,
                self.project_id,
                &relative,
                extracted,
            ))
            .map_err(io::Error::other)
    }
}

/// The [`io::ErrorKind::NotFound`] error [`DbDocStore::read`] and
/// [`DbDocStore::read_with_revision`] both return for a `relative` path with
/// no record, kept in one place so their messages stay identical.
fn record_not_found(relative: &str) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, format!("no record at {relative}"))
}

/// Refuses a write whose `identity_kind` does not carry exactly one of
/// `number`/`concept_id` (ADR 0007's XOR invariant) — e.g. a numbered doc
/// type whose filename lacks a valid `NNNN` prefix, so
/// [`record::extract_record`] yielded no `number` at all.
fn validate_identity(path: &str, extracted: &ExtractedRecord) -> io::Result<()> {
    let satisfies_xor = match extracted.identity_kind.as_str() {
        record::NUMBER_IDENTITY_KIND => {
            extracted.number.is_some() && extracted.concept_id.is_none()
        }
        record::CONCEPT_IDENTITY_KIND => {
            extracted.concept_id.is_some() && extracted.number.is_none()
        }
        _ => false,
    };
    if satisfies_xor {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "{path}: identity_kind '{}' requires exactly one of number/concept_id \
             (number={:?}, concept_id={:?})",
            extracted.identity_kind, extracted.number, extracted.concept_id
        ),
    ))
}

/// Every non-deleted record's path in `project_id` — used exclusively by
/// [`materialize_snapshot`], which feeds every transactional write's
/// `check_violations` call and index regeneration, so excluding a
/// soft-deleted record (ADR 0018, issue 0013 slice A) here is the single
/// point that keeps it out of every regenerated `index.md` and out of
/// `check_violations`'s view of the corpus for every write path.
async fn list_record_paths<C: ConnectionTrait>(conn: &C, project_id: i32) -> Result<Vec<String>> {
    let records = Records::find()
        .filter(Column::ProjectId.eq(project_id))
        .filter(Column::DeletedAt.is_null())
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
async fn load_record<C: ConnectionTrait>(
    conn: &C,
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

    let frontmatter_tail = sync::load_frontmatter_tail(conn, model.id).await?;
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
        status: model.status,
        frontmatter_tail,
    }))
}

const SUPERSEDE_RELATION_KIND: &str = "supersede";

/// `record_id`'s `supersedes` edge (this record is the relation's source),
/// resolved to the target record's zero-padded `NNNN` number — the same raw
/// form [`crate::record::extract_record`] parses from frontmatter (ADR 0007
/// decision 3). `None` when no such edge exists, or the target carries no
/// `number`.
async fn resolve_supersedes<C: ConnectionTrait>(
    conn: &C,
    record_id: i32,
) -> Result<Option<String>> {
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
async fn resolve_superseded_by<C: ConnectionTrait>(
    conn: &C,
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

async fn resolve_number<C: ConnectionTrait>(conn: &C, record_id: i32) -> Result<Option<String>> {
    let record = Records::find_by_id(record_id).one(conn).await?;
    Ok(record
        .and_then(|record| record.number)
        .map(|number| format!("{number:04}")))
}

async fn load_sorted_tags<C: ConnectionTrait>(conn: &C, record_id: i32) -> Result<Vec<String>> {
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

    #[tokio::test]
    async fn migrate_adds_the_revision_column_defaulted_to_one_for_synced_records() {
        let conn = connect_in_memory()
            .await
            .expect("connect to in-memory sqlite");
        migrate(&conn).await.expect("apply migration");
        let (store, bundle) = sync::test_support::seeded_corpus();
        sync::sync(&conn, &store, &bundle)
            .await
            .expect("sync seeded corpus");

        let records = Records::find().all(&conn).await.expect("query records");

        assert_eq!(records.len(), 2);
        assert!(
            records.iter().all(|record| record.revision == 1),
            "every synced record must default revision to 1: {records:?}"
        );
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

    #[tokio::test]
    async fn records_by_type_orders_by_doc_type_then_number_then_path() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");
        let (store, bundle) = sync::test_support::mixed_type_corpus();
        sync::sync(&conn, &store, &bundle).await.expect("sync");

        let entries = records_by_type(&conn).await.expect("records_by_type");

        assert_eq!(
            entries,
            vec![
                NavEntry {
                    doc_type: "ADR".to_owned(),
                    number: Some(1),
                    title: "First ADR".to_owned(),
                    path: "adr/0001-first-adr.md".to_owned(),
                },
                NavEntry {
                    doc_type: "ADR".to_owned(),
                    number: Some(2),
                    title: "Second ADR".to_owned(),
                    path: "adr/0002-second-adr.md".to_owned(),
                },
                NavEntry {
                    doc_type: "BDR".to_owned(),
                    number: Some(1),
                    title: "First BDR".to_owned(),
                    path: "bdr/0001-first-bdr.md".to_owned(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn records_by_type_dedupes_identical_paths_synced_into_two_projects() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");
        let (store_a, bundle_a) = sync::test_support::mixed_type_corpus();
        sync::sync_project(&conn, &store_a, &bundle_a, "team-a")
            .await
            .expect("sync team-a");
        let (store_b, bundle_b) = sync::test_support::mixed_type_corpus();
        sync::sync_project(&conn, &store_b, &bundle_b, "team-b")
            .await
            .expect("sync team-b");

        let entries = records_by_type(&conn).await.expect("records_by_type");

        assert_eq!(
            entries,
            vec![
                NavEntry {
                    doc_type: "ADR".to_owned(),
                    number: Some(1),
                    title: "First ADR".to_owned(),
                    path: "adr/0001-first-adr.md".to_owned(),
                },
                NavEntry {
                    doc_type: "ADR".to_owned(),
                    number: Some(2),
                    title: "Second ADR".to_owned(),
                    path: "adr/0002-second-adr.md".to_owned(),
                },
                NavEntry {
                    doc_type: "BDR".to_owned(),
                    number: Some(1),
                    title: "First BDR".to_owned(),
                    path: "bdr/0001-first-bdr.md".to_owned(),
                },
            ],
            "records_by_type must return each distinct path exactly once even when \
             two projects sync from identical bundles"
        );
    }

    #[tokio::test]
    async fn records_by_type_returns_an_empty_vector_when_nothing_has_synced() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");

        let entries = records_by_type(&conn).await.expect("records_by_type");

        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn record_meta_resolves_both_supersede_directions_with_status_and_tags() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");
        let (store, bundle) = sync::test_support::superseding_corpus();
        sync::sync(&conn, &store, &bundle).await.expect("sync");

        let superseded = record_meta(&conn, "adr/0001-quokka-caching.md")
            .await
            .expect("query record_meta for superseded record")
            .expect("superseded record exists");
        assert_eq!(superseded.doc_type, "ADR");
        assert_eq!(superseded.status, None);
        assert!(superseded.supersedes.is_empty());
        assert_eq!(
            superseded.superseded_by,
            vec![RelatedRef {
                path: "adr/0002-quokka-caching-v2.md".to_owned(),
                title: "Quokka Caching V2".to_owned(),
            }]
        );
        assert_eq!(superseded.tags, vec!["caching".to_owned()]);

        let superseding = record_meta(&conn, "adr/0002-quokka-caching-v2.md")
            .await
            .expect("query record_meta for superseding record")
            .expect("superseding record exists");
        assert_eq!(superseding.status, Some("Accepted".to_owned()));
        assert_eq!(
            superseding.supersedes,
            vec![RelatedRef {
                path: "adr/0001-quokka-caching.md".to_owned(),
                title: "Quokka Caching".to_owned(),
            }]
        );
        assert!(superseding.superseded_by.is_empty());
        assert_eq!(
            superseding.tags,
            vec!["caching".to_owned(), "performance".to_owned()]
        );
    }

    #[tokio::test]
    async fn record_meta_returns_none_for_an_unknown_path() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");

        let meta = record_meta(&conn, "adr/9999-missing.md")
            .await
            .expect("record_meta on an unknown path is not an error");

        assert!(meta.is_none());
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

    /// A [`DocStore`] with no records, used only to bootstrap the `"default"`
    /// project row [`DbDocStore::new`] expects to already exist, without
    /// ingesting anything from disk — the test-only mirror of the CLI's own
    /// bootstrap store.
    struct NoRecords;

    impl DocStore for NoRecords {
        fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(Vec::new())
        }

        fn read(&self, _path: &Path) -> io::Result<String> {
            Err(io::Error::new(io::ErrorKind::NotFound, "no records yet"))
        }

        fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
            Ok(())
        }
    }

    fn temp_root_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("living-docs-write-checked-{label}-{nanos}"));
        std::fs::create_dir_all(root.join("adr")).expect("create scratch adr dir");
        root
    }

    /// Seeds a real on-disk bundle-root `index.md` linking to `adr/index.md`,
    /// which in turn lists `adr_entries` — `check`'s directory-membership and
    /// reachability invariants read these straight off disk regardless of
    /// backend, so [`DbDocStore::write_checked`]'s in-transaction `check`
    /// needs them present before it is ever called.
    fn seed_index(root: &Path, adr_entries: &[&str]) {
        std::fs::write(root.join("index.md"), "# Index\n\n- [ADRs](adr/index.md)\n")
            .expect("write root index");
        let rows: String = adr_entries
            .iter()
            .map(|entry| format!("- [{entry}]({entry}.md)\n"))
            .collect();
        std::fs::write(
            root.join("adr").join("index.md"),
            format!("# ADRs\n\n{rows}"),
        )
        .expect("write adr index");
    }

    /// Opens a fresh temp sqlite file, migrates it, and bootstraps the
    /// `"default"` project with zero records via [`NoRecords`], then builds
    /// the [`DbDocStore`] under test against it — mirroring
    /// [`db_search_index_bridges_the_sync_search_index_port_without_an_ambient_runtime`]'s
    /// pattern of dropping the setup runtime before handing control to the
    /// store's own.
    fn bootstrap_db_doc_store(label: &str, root: &Path) -> (DbDocStore, PathBuf, String) {
        let (db_path, db_url) = temp_sqlite_url(label);
        let setup_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build setup runtime");
        setup_runtime.block_on(async {
            let conn = connect(&db_url).await.expect("connect");
            migrate(&conn).await.expect("migrate");
            sync::sync(&conn, &NoRecords, root)
                .await
                .expect("bootstrap default project");
        });
        drop(setup_runtime);
        let store = DbDocStore::new(&db_url, root.to_path_buf()).expect("build DbDocStore");
        (store, db_path, db_url)
    }

    fn cleanup_write_checked_fixture(root: &Path, db_path: &Path) {
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_dir(db_path.parent().expect("db path has a parent"));
    }

    const CLEAN_ADR: &str = "---\ntype: ADR\ntitle: Clean Decision\ndescription: A minimal record with no links.\nstatus: Accepted\nsupersedes:\nsuperseded_by:\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Clean Decision\n\nA self-contained decision with no links, so it passes check outright.\n";

    #[test]
    fn write_checked_commits_a_valid_new_record_with_revision_one_and_it_is_immediately_readable() {
        let root = temp_root_dir("commit");
        seed_index(&root, &["0001-clean-decision"]);
        let (store, db_path, db_url) = bootstrap_db_doc_store("write-checked-commit", &root);
        let target = root.join("adr").join("0001-clean-decision.md");

        let revision = store
            .write_checked(&target, CLEAN_ADR)
            .expect("a valid record should commit");
        assert_eq!(revision, 1);

        let read_back = store
            .read(&target)
            .expect("the committed record should be immediately readable");
        assert!(read_back.contains("Clean Decision"));

        let verify_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build verify runtime");
        let stored_revision: i64 = verify_runtime.block_on(async {
            let conn = connect(&db_url).await.expect("reconnect to verify");
            Records::find()
                .filter(Column::Path.eq("adr/0001-clean-decision.md"))
                .one(&conn)
                .await
                .expect("query the committed record")
                .expect("the committed record exists")
                .revision
        });
        assert_eq!(stored_revision, 1);

        cleanup_write_checked_fixture(&root, &db_path);
    }

    const BROKEN_SUPERSEDE_ADR: &str = "---\ntype: ADR\ntitle: Broken Supersede\ndescription: d.\nstatus: Superseded\nsuperseded_by: 9999\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Broken Supersede\n\nBody.\n";

    #[test]
    fn write_checked_rolls_back_and_reports_check_failed_when_a_supersede_target_is_missing() {
        let root = temp_root_dir("rollback");
        seed_index(&root, &["0001-broken-supersede"]);
        let (store, db_path, db_url) = bootstrap_db_doc_store("write-checked-rollback", &root);
        let target = root.join("adr").join("0001-broken-supersede.md");
        let index_path = root.join("adr").join("index.md");
        let index_before = fs::read_to_string(&index_path).expect("read seeded adr/index.md");

        let err = store
            .write_checked(&target, BROKEN_SUPERSEDE_ADR)
            .expect_err("a record failing check must not commit");

        let index_after =
            fs::read_to_string(&index_path).expect("read adr/index.md after rollback");
        assert_eq!(
            index_before, index_after,
            "a failing check must leave adr/index.md byte-identical to its pre-call content"
        );

        match err {
            WriteCheckedError::CheckFailed(violations) => {
                assert!(
                    violations
                        .iter()
                        .any(|(_, message)| message.contains("superseded_by")),
                    "got: {violations:?}"
                );
            }
            other => panic!("expected CheckFailed, got: {other}"),
        }

        let verify_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build verify runtime");
        let still_absent = verify_runtime.block_on(async {
            let conn = connect(&db_url).await.expect("reconnect to verify");
            Records::find()
                .filter(Column::Path.eq("adr/0001-broken-supersede.md"))
                .one(&conn)
                .await
                .expect("query for the rolled-back record")
        });
        assert!(
            still_absent.is_none(),
            "a failing check must leave no row behind"
        );

        cleanup_write_checked_fixture(&root, &db_path);
    }

    #[test]
    fn write_checked_refuses_a_second_record_at_an_already_taken_path() {
        let root = temp_root_dir("already-exists");
        seed_index(&root, &["0001-clean-decision"]);
        let (store, db_path, _db_url) =
            bootstrap_db_doc_store("write-checked-already-exists", &root);
        let target = root.join("adr").join("0001-clean-decision.md");

        store
            .write_checked(&target, CLEAN_ADR)
            .expect("the first write should commit");

        let err = store
            .write_checked(&target, CLEAN_ADR)
            .expect_err("a second write at the same path must be refused");
        match err {
            WriteCheckedError::AlreadyExists(path) => {
                assert_eq!(path, "adr/0001-clean-decision.md");
            }
            other => panic!("expected AlreadyExists, got: {other}"),
        }

        cleanup_write_checked_fixture(&root, &db_path);
    }

    #[test]
    fn write_checked_succeeds_against_a_fresh_bundle_whose_type_index_was_never_pre_seeded_with_the_new_records_link(
    ) {
        let root = temp_root_dir("fresh-index");
        fs::write(root.join("index.md"), "# Index\n\n- [ADRs](adr/index.md)\n")
            .expect("write root index");
        assert!(
            !root.join("adr").join("index.md").exists(),
            "the fixture must not pre-seed adr/index.md — that is the bug under test"
        );
        let (store, db_path, _db_url) = bootstrap_db_doc_store("write-checked-fresh-index", &root);
        let target = root.join("adr").join("0001-clean-decision.md");

        let revision = store.write_checked(&target, CLEAN_ADR).expect(
            "write_checked must succeed on a fresh bundle with no manually pre-seeded index link",
        );

        assert_eq!(revision, 1);
        let index_content = fs::read_to_string(root.join("adr").join("index.md"))
            .expect("write_checked must have regenerated adr/index.md");
        assert!(
            index_content.contains("0001-clean-decision.md"),
            "got: {index_content}"
        );

        cleanup_write_checked_fixture(&root, &db_path);
    }

    const SECOND_CLEAN_ADR: &str = "---\ntype: ADR\ntitle: Second Decision\ndescription: A second minimal record with no links.\nstatus: Accepted\nsupersedes:\nsuperseded_by:\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Second Decision\n\nAnother self-contained decision with no links.\n";

    #[test]
    fn write_checked_composes_two_successful_writes_into_one_index_listing_both_records() {
        let root = temp_root_dir("composition");
        seed_index(&root, &[]);
        let (store, db_path, _db_url) = bootstrap_db_doc_store("write-checked-composition", &root);

        let first_target = root.join("adr").join("0001-clean-decision.md");
        store
            .write_checked(&first_target, CLEAN_ADR)
            .expect("the first write should commit");

        let second_target = root.join("adr").join("0002-second-decision.md");
        store
            .write_checked(&second_target, SECOND_CLEAN_ADR)
            .expect("the second write should commit");

        let index_content = fs::read_to_string(root.join("adr").join("index.md"))
            .expect("read the final regenerated adr/index.md");
        assert!(
            index_content.contains("0001-clean-decision.md"),
            "got: {index_content}"
        );
        assert!(
            index_content.contains("0002-second-decision.md"),
            "got: {index_content}"
        );

        cleanup_write_checked_fixture(&root, &db_path);
    }

    const UPDATED_CLEAN_ADR: &str = "---\ntype: ADR\ntitle: Clean Decision Updated\ndescription: An edited minimal record with no links.\nstatus: Accepted\nsupersedes:\nsuperseded_by:\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Clean Decision Updated\n\nAn edited self-contained decision with no links, so it still passes check.\n";

    fn stored_record(db_url: &str, path: &str) -> entity::Model {
        let verify_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build verify runtime");
        verify_runtime.block_on(async {
            let conn = connect(db_url).await.expect("reconnect to verify");
            Records::find()
                .filter(Column::Path.eq(path))
                .one(&conn)
                .await
                .expect("query the record")
                .expect("the record still exists")
        })
    }

    #[test]
    fn update_checked_commits_a_valid_edit_bumps_revision_and_it_is_immediately_readable() {
        let root = temp_root_dir("update-commit");
        seed_index(&root, &["0001-clean-decision"]);
        let (store, db_path, _db_url) = bootstrap_db_doc_store("update-checked-commit", &root);
        let target = root.join("adr").join("0001-clean-decision.md");
        store
            .write_checked(&target, CLEAN_ADR)
            .expect("seed the record to edit");

        let revision = store
            .update_checked(&target, UPDATED_CLEAN_ADR, Some(1))
            .expect("a valid edit against the current revision should commit");

        assert_eq!(revision, 2);
        let read_back = store.read(&target).expect("the edited record is readable");
        assert!(
            read_back.contains("Clean Decision Updated"),
            "got: {read_back}"
        );

        cleanup_write_checked_fixture(&root, &db_path);
    }

    #[test]
    fn update_checked_with_no_base_revision_skips_the_precondition_and_still_bumps_revision() {
        let root = temp_root_dir("update-no-precondition");
        seed_index(&root, &["0001-clean-decision"]);
        let (store, db_path, _db_url) =
            bootstrap_db_doc_store("update-checked-no-precondition", &root);
        let target = root.join("adr").join("0001-clean-decision.md");
        store
            .write_checked(&target, CLEAN_ADR)
            .expect("seed the record to edit");

        let revision = store
            .update_checked(&target, UPDATED_CLEAN_ADR, None)
            .expect("an edit with no base_revision must skip the precondition");

        assert_eq!(revision, 2);

        cleanup_write_checked_fixture(&root, &db_path);
    }

    #[test]
    fn update_checked_refuses_a_second_edit_reusing_a_now_stale_base_revision_and_leaves_the_first_commit_intact(
    ) {
        let root = temp_root_dir("update-stale");
        seed_index(&root, &["0001-clean-decision"]);
        let (store, db_path, db_url) = bootstrap_db_doc_store("update-checked-stale", &root);
        let target = root.join("adr").join("0001-clean-decision.md");
        store
            .write_checked(&target, CLEAN_ADR)
            .expect("seed the record to edit");

        store
            .update_checked(&target, UPDATED_CLEAN_ADR, Some(1))
            .expect("the first edit against revision 1 should commit");

        const SECOND_EDIT: &str = "---\ntype: ADR\ntitle: Clean Decision Conflicting Edit\ndescription: A conflicting edit that should never land.\nstatus: Accepted\nsupersedes:\nsuperseded_by:\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Clean Decision Conflicting Edit\n\nThis body must never be stored.\n";
        let err = store
            .update_checked(&target, SECOND_EDIT, Some(1))
            .expect_err("reusing the original, now-stale revision must be refused");
        match err {
            WriteCheckedError::StaleRevision {
                path,
                expected,
                actual,
            } => {
                assert_eq!(path, "adr/0001-clean-decision.md");
                assert_eq!(expected, 1);
                assert_eq!(actual, 2);
            }
            other => panic!("expected StaleRevision, got: {other}"),
        }

        let stored = stored_record(&db_url, "adr/0001-clean-decision.md");
        assert_eq!(
            stored.revision, 2,
            "a stale rejection must not bump the revision again"
        );
        assert_eq!(
            stored.title, "Clean Decision Updated",
            "a stale rejection must leave the first commit's content exactly as it landed, \
             never overwritten by the rejected second submission"
        );

        cleanup_write_checked_fixture(&root, &db_path);
    }

    #[test]
    fn update_checked_refuses_editing_a_path_with_no_existing_record() {
        let root = temp_root_dir("update-not-found");
        seed_index(&root, &["0001-clean-decision"]);
        let (store, db_path, _db_url) = bootstrap_db_doc_store("update-checked-not-found", &root);
        let target = root.join("adr").join("0001-clean-decision.md");

        let err = store
            .update_checked(&target, CLEAN_ADR, None)
            .expect_err("editing a path with no existing record must be refused");

        match err {
            WriteCheckedError::NotFound(path) => assert_eq!(path, "adr/0001-clean-decision.md"),
            other => panic!("expected NotFound, got: {other}"),
        }

        cleanup_write_checked_fixture(&root, &db_path);
    }

    #[test]
    fn update_checked_rolls_back_and_reports_check_failed_when_the_edit_introduces_a_broken_link() {
        let root = temp_root_dir("update-rollback");
        seed_index(&root, &["0001-clean-decision"]);
        let (store, db_path, db_url) = bootstrap_db_doc_store("update-checked-rollback", &root);
        let target = root.join("adr").join("0001-clean-decision.md");
        store
            .write_checked(&target, CLEAN_ADR)
            .expect("seed the record to edit");

        let err = store
            .update_checked(&target, BROKEN_SUPERSEDE_ADR, Some(1))
            .expect_err("an edit that fails check must not commit");

        match err {
            WriteCheckedError::CheckFailed(violations) => {
                assert!(
                    violations
                        .iter()
                        .any(|(_, message)| message.contains("superseded_by")),
                    "got: {violations:?}"
                );
            }
            other => panic!("expected CheckFailed, got: {other}"),
        }

        let stored = stored_record(&db_url, "adr/0001-clean-decision.md");
        assert_eq!(stored.revision, 1, "a failing check must not bump revision");
        assert_eq!(stored.title, "Clean Decision");

        cleanup_write_checked_fixture(&root, &db_path);
    }

    #[test]
    fn read_with_revision_returns_the_current_content_and_revision() {
        let root = temp_root_dir("read-with-revision");
        seed_index(&root, &["0001-clean-decision"]);
        let (store, db_path, _db_url) = bootstrap_db_doc_store("read-with-revision", &root);
        let target = root.join("adr").join("0001-clean-decision.md");
        store
            .write_checked(&target, CLEAN_ADR)
            .expect("seed the record");
        store
            .update_checked(&target, UPDATED_CLEAN_ADR, Some(1))
            .expect("edit the record");

        let (content, revision) = store
            .read_with_revision(&target)
            .expect("read_with_revision should succeed for an existing record");

        assert_eq!(revision, 2);
        assert!(content.contains("Clean Decision Updated"), "got: {content}");

        cleanup_write_checked_fixture(&root, &db_path);
    }

    #[test]
    fn read_with_revision_returns_not_found_for_a_missing_path() {
        let root = temp_root_dir("read-with-revision-missing");
        seed_index(&root, &[]);
        let (store, db_path, _db_url) = bootstrap_db_doc_store("read-with-revision-missing", &root);
        let target = root.join("adr").join("0001-clean-decision.md");

        let err = store
            .read_with_revision(&target)
            .expect_err("reading a missing record must fail");

        assert_eq!(err.kind(), io::ErrorKind::NotFound);

        cleanup_write_checked_fixture(&root, &db_path);
    }

    struct SeedStore {
        files: BTreeMap<PathBuf, String>,
    }

    impl DocStore for SeedStore {
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

    /// Bootstraps a fresh temp sqlite file with `adr_files` synced straight
    /// in (bypassing `write_checked`'s own `check` gate entirely) — the
    /// `supersede_checked` check-gate-failure test needs a record that is
    /// already broken before `supersede_checked` ever runs, which
    /// `write_checked` would refuse to seed in the first place.
    fn bootstrap_db_doc_store_seeded(
        label: &str,
        root: &Path,
        adr_files: &[(&str, &str)],
    ) -> (DbDocStore, PathBuf, String) {
        let (db_path, db_url) = temp_sqlite_url(label);
        let files = adr_files
            .iter()
            .map(|(name, contents)| (root.join("adr").join(name), (*contents).to_owned()))
            .collect();
        let setup_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build setup runtime");
        setup_runtime.block_on(async {
            let conn = connect(&db_url).await.expect("connect");
            migrate(&conn).await.expect("migrate");
            sync::sync(&conn, &SeedStore { files }, root)
                .await
                .expect("bootstrap seeded project");
        });
        drop(setup_runtime);
        let store = DbDocStore::new(&db_url, root.to_path_buf()).expect("build DbDocStore");
        (store, db_path, db_url)
    }

    const OLD_SUPERSEDE_ADR: &str = "---\ntype: ADR\ntitle: Old Decision\ndescription: d.\nstatus: Proposed\nsupersedes:\nsuperseded_by:\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Old Decision\n\nBody.\n";
    const NEW_SUPERSEDE_ADR: &str = "---\ntype: ADR\ntitle: New Decision\ndescription: d.\nstatus: Proposed\nsupersedes:\nsuperseded_by:\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# New Decision\n\nBody.\n";

    #[test]
    fn supersede_checked_produces_the_same_frontmatter_the_cli_unchecked_path_produces_and_bumps_both_revisions(
    ) {
        let root = temp_root_dir("supersede-checked-commit");
        seed_index(&root, &["0001-old-decision", "0002-new-decision"]);
        let (store, db_path, db_url) = bootstrap_db_doc_store("supersede-checked-commit", &root);
        store
            .write_checked(
                &root.join("adr").join("0001-old-decision.md"),
                OLD_SUPERSEDE_ADR,
            )
            .expect("seed old record");
        store
            .write_checked(
                &root.join("adr").join("0002-new-decision.md"),
                NEW_SUPERSEDE_ADR,
            )
            .expect("seed new record");

        store
            .supersede_checked("1", "2")
            .expect("a check-passing supersede should commit");

        let old = store
            .read(&root.join("adr").join("0001-old-decision.md"))
            .expect("old record readable");
        let new = store
            .read(&root.join("adr").join("0002-new-decision.md"))
            .expect("new record readable");
        assert!(old.contains("status: Superseded"), "got: {old}");
        assert!(old.contains("superseded_by: 0002"), "got: {old}");
        assert!(new.contains("supersedes: 0001"), "got: {new}");

        assert_eq!(
            stored_record(&db_url, "adr/0001-old-decision.md").revision,
            2
        );
        assert_eq!(
            stored_record(&db_url, "adr/0002-new-decision.md").revision,
            2
        );

        cleanup_write_checked_fixture(&root, &db_path);
    }

    #[test]
    fn supersede_checked_rejects_a_nonexistent_target_number_with_the_cli_message_and_leaves_records_unchanged(
    ) {
        let root = temp_root_dir("supersede-checked-missing");
        seed_index(&root, &["0001-old-decision"]);
        let (store, db_path, db_url) = bootstrap_db_doc_store("supersede-checked-missing", &root);
        store
            .write_checked(
                &root.join("adr").join("0001-old-decision.md"),
                OLD_SUPERSEDE_ADR,
            )
            .expect("seed old record");

        let err = store
            .supersede_checked("1", "99")
            .expect_err("an unknown target number must be refused");

        match err {
            SupersedeCheckedError::ResolutionFailed(message) => {
                assert!(
                    message.contains("no record found for 0099"),
                    "got: {message}"
                );
            }
            other => panic!("expected ResolutionFailed, got: {other}"),
        }

        let stored = stored_record(&db_url, "adr/0001-old-decision.md");
        assert_eq!(
            stored.revision, 1,
            "a resolution failure must not bump revision"
        );
        assert_eq!(
            stored.status.as_deref(),
            Some("Proposed"),
            "a resolution failure must leave the old record's content untouched"
        );

        cleanup_write_checked_fixture(&root, &db_path);
    }

    const PRE_BROKEN_ADR: &str = "---\ntype: ADR\ntitle: Pre Broken\ndescription: d.\nstatus: Superseded\nsuperseded_by: 9999\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Pre Broken\n\nBody.\n";

    #[test]
    fn supersede_checked_rolls_back_everything_when_an_unrelated_record_already_fails_check() {
        let root = temp_root_dir("supersede-checked-rollback");
        seed_index(
            &root,
            &["0001-old-decision", "0002-new-decision", "0003-pre-broken"],
        );
        let (store, db_path, db_url) = bootstrap_db_doc_store_seeded(
            "supersede-checked-rollback",
            &root,
            &[
                ("0001-old-decision.md", OLD_SUPERSEDE_ADR),
                ("0002-new-decision.md", NEW_SUPERSEDE_ADR),
                ("0003-pre-broken.md", PRE_BROKEN_ADR),
            ],
        );
        let index_path = root.join("adr").join("index.md");
        let index_before = fs::read_to_string(&index_path).expect("read seeded adr/index.md");

        let err = store
            .supersede_checked("1", "2")
            .expect_err("a corpus that already fails check must not commit");

        match err {
            SupersedeCheckedError::CheckFailed(violations) => {
                assert!(
                    violations
                        .iter()
                        .any(|(_, message)| message.contains("superseded_by")),
                    "got: {violations:?}"
                );
            }
            other => panic!("expected CheckFailed, got: {other}"),
        }

        let index_after =
            fs::read_to_string(&index_path).expect("read adr/index.md after rollback");
        assert_eq!(
            index_before, index_after,
            "a failing check must leave adr/index.md byte-identical to its pre-call content"
        );

        let old = stored_record(&db_url, "adr/0001-old-decision.md");
        let new = stored_record(&db_url, "adr/0002-new-decision.md");
        assert_eq!(
            old.revision, 1,
            "a failing check must not bump the old record's revision"
        );
        assert_eq!(
            new.revision, 1,
            "a failing check must not bump the new record's revision"
        );
        assert_eq!(
            old.status.as_deref(),
            Some("Proposed"),
            "a failing check must leave the old record's content untouched"
        );

        cleanup_write_checked_fixture(&root, &db_path);
    }

    /// [`temp_root_dir`], plus a scratch `issues` directory — the delete
    /// tests need an eligible `issue` record alongside the `adr` fixtures
    /// [`temp_root_dir`] already provisions.
    fn temp_root_dir_with_issues(label: &str) -> PathBuf {
        let root = temp_root_dir(label);
        std::fs::create_dir_all(root.join("issues")).expect("create scratch issues dir");
        root
    }

    /// [`seed_index`], extended with an `issues` directory link and listing
    /// — the delete tests need both an `adr` and an `issues` type index
    /// reachable from the bundle root for `check` to pass.
    fn seed_index_with_issues(root: &Path, adr_entries: &[&str], issue_entries: &[&str]) {
        std::fs::write(
            root.join("index.md"),
            "# Index\n\n- [ADRs](adr/index.md)\n- [Issues](issues/index.md)\n",
        )
        .expect("write root index");
        write_type_index(root, "adr", adr_entries);
        write_type_index(root, "issues", issue_entries);
    }

    fn write_type_index(root: &Path, dir: &str, entries: &[&str]) {
        let rows: String = entries
            .iter()
            .map(|entry| format!("- [{entry}]({entry}.md)\n"))
            .collect();
        std::fs::write(
            root.join(dir).join("index.md"),
            format!("# Index\n\n{rows}"),
        )
        .expect("write type index");
    }

    const CLEAN_ISSUE: &str = "---\ntype: Issue\ntitle: Clean Issue\ndescription: A minimal issue record with no links.\nstatus: Open\nsupersedes:\nsuperseded_by:\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Clean Issue\n\nA self-contained issue record about quokka onboarding, so it passes check outright.\n";

    #[test]
    fn delete_checked_soft_deletes_an_eligible_issue_with_no_inbound_relations() {
        let root = temp_root_dir_with_issues("delete-success");
        seed_index_with_issues(&root, &[], &["0001-clean-issue"]);
        let (store, db_path, db_url) = bootstrap_db_doc_store("delete-checked-success", &root);
        let target = root.join("issues").join("0001-clean-issue.md");
        store
            .write_checked(&target, CLEAN_ISSUE)
            .expect("seed the issue to delete");

        store
            .delete_checked(&target)
            .expect("an eligible issue with no inbound relations should soft-delete");

        let verify_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build verify runtime");
        verify_runtime.block_on(async {
            let conn = connect(&db_url).await.expect("reconnect to verify");

            let meta = record_meta(&conn, "issues/0001-clean-issue.md")
                .await
                .expect("record_meta query")
                .expect("record_meta must still return a soft-deleted record");
            assert!(
                meta.deleted_at.is_some(),
                "deleted_at must be set after a successful delete_checked"
            );

            let found = record_by_path(&conn, "issues/0001-clean-issue.md")
                .await
                .expect("record_by_path query")
                .expect("record_by_path must still return a soft-deleted record");
            assert_eq!(found.title, "Clean Issue");

            let nav_entries = records_by_type(&conn).await.expect("records_by_type");
            assert!(
                !nav_entries
                    .iter()
                    .any(|entry| entry.path == "issues/0001-clean-issue.md"),
                "a soft-deleted record must not appear in the nav tree: {nav_entries:?}"
            );

            let hits = search(&conn, "onboarding").await.expect("search");
            assert!(
                hits.is_empty(),
                "a soft-deleted record must not appear in search results: {hits:?}"
            );
        });

        cleanup_write_checked_fixture(&root, &db_path);
    }

    #[test]
    fn delete_checked_refuses_a_non_issue_record_and_leaves_deleted_at_unset() {
        let root = temp_root_dir_with_issues("delete-ineligible");
        seed_index_with_issues(&root, &["0001-clean-decision"], &[]);
        let (store, db_path, db_url) = bootstrap_db_doc_store("delete-checked-ineligible", &root);
        let target = root.join("adr").join("0001-clean-decision.md");
        store
            .write_checked(&target, CLEAN_ADR)
            .expect("seed the adr record");

        let err = store
            .delete_checked(&target)
            .expect_err("an ADR record must be refused for delete");

        match err {
            DeleteCheckedError::IneligibleType { path, doc_type } => {
                assert_eq!(path, "adr/0001-clean-decision.md");
                assert_eq!(doc_type, "ADR");
            }
            other => panic!("expected IneligibleType, got: {other}"),
        }

        let stored = stored_record(&db_url, "adr/0001-clean-decision.md");
        assert_eq!(
            stored.deleted_at, None,
            "a refused delete must leave deleted_at unset"
        );

        cleanup_write_checked_fixture(&root, &db_path);
    }

    const ISSUE_DELETE_TARGET: &str = "---\ntype: Issue\ntitle: Target Issue\ndescription: d.\nstatus: Open\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Target Issue\n\nBody.\n";
    const ISSUE_SUPERSEDING_SOURCE: &str = "---\ntype: Issue\ntitle: Source Issue\ndescription: d.\nstatus: Open\nsupersedes: 1\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Source Issue\n\nBody.\n";

    #[test]
    fn delete_checked_refuses_a_record_with_an_inbound_relation_and_leaves_deleted_at_unset() {
        let root = temp_root_dir_with_issues("delete-inbound-relations");
        seed_index_with_issues(&root, &[], &["0001-target-issue", "0002-source-issue"]);
        let (store, db_path, db_url) =
            bootstrap_db_doc_store("delete-checked-inbound-relations", &root);
        let target = root.join("issues").join("0001-target-issue.md");
        store
            .write_checked(&target, ISSUE_DELETE_TARGET)
            .expect("seed the target issue");
        store
            .write_checked(
                &root.join("issues").join("0002-source-issue.md"),
                ISSUE_SUPERSEDING_SOURCE,
            )
            .expect("seed the issue that supersedes (and thus points at) the target");

        let err = store
            .delete_checked(&target)
            .expect_err("a record another record's relations row points at must be refused");

        match err {
            DeleteCheckedError::HasInboundRelations(path) => {
                assert_eq!(path, "issues/0001-target-issue.md");
            }
            other => panic!("expected HasInboundRelations, got: {other}"),
        }
        let stored = stored_record(&db_url, "issues/0001-target-issue.md");
        assert_eq!(
            stored.deleted_at, None,
            "a refused delete must leave deleted_at unset"
        );

        cleanup_write_checked_fixture(&root, &db_path);
    }

    #[test]
    fn delete_checked_refuses_deleting_a_path_with_no_existing_record() {
        let root = temp_root_dir_with_issues("delete-not-found");
        seed_index_with_issues(&root, &[], &[]);
        let (store, db_path, _db_url) = bootstrap_db_doc_store("delete-checked-not-found", &root);
        let target = root.join("issues").join("0001-clean-issue.md");

        let err = store
            .delete_checked(&target)
            .expect_err("deleting a path with no existing record must be refused");

        match err {
            DeleteCheckedError::NotFound(path) => assert_eq!(path, "issues/0001-clean-issue.md"),
            other => panic!("expected NotFound, got: {other}"),
        }

        cleanup_write_checked_fixture(&root, &db_path);
    }
}
