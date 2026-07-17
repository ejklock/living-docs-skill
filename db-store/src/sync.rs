//! Full-rebuild sync from a [`living_docs_core::store::DocStore`] into the
//! `records` read-model, plus its backend-native search index (ADR 0004,
//! issue 0002 slice S2b; ParadeDB branch issue 0004 slice 0004-B;
//! default-project assignment issue 0005 slice 0005-A; per-project ingestion
//! + relations/tags issue 0005 slice 0005-B).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use living_docs_core::store::DocStore;
use sea_orm::sea_query::Query;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend,
    DbErr, EntityTrait, QueryFilter, Statement, TransactionTrait,
};

use crate::entity::{frontmatter_fields, projects, record_tags, relations, tags};
use crate::entity::{ActiveModel, Column, Entity as Records, Model};
use crate::record::{extract_record, is_reserved, ExtractedRecord};
use crate::Result;

/// The slug [`sync`] assigns every record to: the single project every
/// caller that does not (yet) think in terms of named projects keeps
/// syncing into, unchanged since issue 0005 slice 0005-A.
const DEFAULT_PROJECT_SLUG: &str = "default";

/// Rebuilds the single default project's records exactly as slice 0005-A
/// did, for every caller that has not been upgraded to name a project.
/// Equivalent to `sync_project(conn, store, bundle, "default")`.
pub async fn sync(conn: &DatabaseConnection, store: &dyn DocStore, bundle: &Path) -> Result<usize> {
    sync_project(conn, store, bundle, DEFAULT_PROJECT_SLUG).await
}

/// Rebuilds `project_slug`'s slice of the `records`/`relations`/`tags`/
/// `record_tags` tables and the backend-native search index, from every
/// non-reserved `.md` doc `store` lists under `bundle`, in one transaction.
/// Idempotent: running twice over an unchanged corpus yields identical rows
/// for this project. Only this project's rows are cleared first — a
/// re-sync never touches another project's records, relations, or tags.
/// The project is upserted by `project_slug` (rooted at `bundle` on first
/// use). Insertion is two-pass: every record lands first, then each
/// record's `supersedes`/`superseded_by` frontmatter is resolved against
/// its *own* project's just-inserted records and tags are attached — a
/// target that does not resolve within the project is skipped, not
/// inserted as a dangling relation. Returns the number of records inserted.
pub async fn sync_project(
    conn: &DatabaseConnection,
    store: &dyn DocStore,
    bundle: &Path,
    project_slug: &str,
) -> Result<usize> {
    let paths = store.list(bundle).map_err(io_err_to_db_err)?;
    let txn = conn.begin().await?;

    let project_id = ensure_project(&txn, project_slug, bundle).await?;
    clear_project(&txn, project_id).await?;

    let mut inserted = Vec::new();
    for path in paths {
        if is_reserved(&path) {
            continue;
        }
        inserted.push(insert_record(&txn, store, bundle, &path, project_id).await?);
    }
    let count = inserted.len();

    insert_supersede_relations(&txn, project_id, &inserted).await?;
    insert_tags(&txn, project_id, &inserted).await?;

    rebuild_search_index(&txn).await?;
    txn.commit().await?;
    Ok(count)
}

/// A single record just inserted this sync run, carrying the frontmatter
/// slice_id 0005-B needs to resolve relations and tags in the following
/// passes.
struct InsertedRecord {
    id: i32,
    relative_path: String,
    supersedes: Option<String>,
    superseded_by: Option<String>,
    tags: Vec<String>,
}

/// Finds `slug`'s project, inserting it (rooted at `bundle`) the first time
/// a sync targets it. Returns the project's id either way.
async fn ensure_project<C: ConnectionTrait>(conn: &C, slug: &str, bundle: &Path) -> Result<i32> {
    if let Some(existing) = projects::Entity::find()
        .filter(projects::Column::Slug.eq(slug))
        .one(conn)
        .await?
    {
        return Ok(existing.id);
    }

    let inserted = projects::ActiveModel {
        slug: ActiveValue::Set(slug.to_owned()),
        name: ActiveValue::Set(slug.to_owned()),
        root_path: ActiveValue::Set(Some(bundle.to_string_lossy().into_owned())),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    Ok(inserted.id)
}

/// Deletes `project_id`'s rows from `record_tags`, `relations`, `tags`, and
/// `records`, in FK-safe order, leaving every other project's rows intact.
async fn clear_project<C: ConnectionTrait>(conn: &C, project_id: i32) -> Result<()> {
    delete_record_tags_for_project(conn, project_id).await?;
    relations::Entity::delete_many()
        .filter(relations::Column::ProjectId.eq(project_id))
        .exec(conn)
        .await?;
    tags::Entity::delete_many()
        .filter(tags::Column::ProjectId.eq(project_id))
        .exec(conn)
        .await?;
    Records::delete_many()
        .filter(Column::ProjectId.eq(project_id))
        .exec(conn)
        .await?;
    Ok(())
}

/// `record_tags` carries no `project_id` of its own, so scoping its delete
/// to `project_id` goes through the owning record. Built with SeaORM's
/// query builder (`in_subquery`), not a raw placeholder, so it renders
/// `?`/`$1` correctly on both SQLite and Postgres/ParadeDB.
async fn delete_record_tags_for_project<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
) -> Result<()> {
    let project_record_ids = Query::select()
        .column(Column::Id)
        .from(Records)
        .and_where(Column::ProjectId.eq(project_id))
        .to_owned();

    record_tags::Entity::delete_many()
        .filter(record_tags::Column::RecordId.in_subquery(project_record_ids))
        .exec(conn)
        .await
        .map(|_| ())
}

async fn insert_record<C: ConnectionTrait>(
    conn: &C,
    store: &dyn DocStore,
    bundle: &Path,
    path: &Path,
    project_id: i32,
) -> Result<InsertedRecord> {
    let relative = relative_path(bundle, path);
    let contents = store.read(path).map_err(io_err_to_db_err)?;
    let extracted = extract_record(Path::new(&relative), &contents);
    let frontmatter_tail = extracted.frontmatter_tail;

    let inserted = ActiveModel {
        project_id: ActiveValue::Set(project_id),
        path: ActiveValue::Set(relative.clone()),
        doc_type: ActiveValue::Set(extracted.doc_type),
        number: ActiveValue::Set(extracted.number),
        concept_id: ActiveValue::Set(extracted.concept_id),
        identity_kind: ActiveValue::Set(extracted.identity_kind),
        title: ActiveValue::Set(extracted.title),
        description: ActiveValue::Set(extracted.description),
        body: ActiveValue::Set(extracted.body),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    insert_frontmatter_tail(conn, inserted.id, &frontmatter_tail).await?;

    Ok(InsertedRecord {
        id: inserted.id,
        relative_path: relative,
        supersedes: extracted.supersedes,
        superseded_by: extracted.superseded_by,
        tags: extracted.tags,
    })
}

/// Inserts one `frontmatter_fields` row per tail entry, `ordinal` set to
/// its position in `tail` so the tail reconstructs by ascending `ordinal`
/// in the same order it was encountered in the source frontmatter.
async fn insert_frontmatter_tail<C: ConnectionTrait>(
    conn: &C,
    record_id: i32,
    tail: &[(String, String)],
) -> Result<()> {
    for (ordinal, (key, value)) in tail.iter().enumerate() {
        frontmatter_fields::ActiveModel {
            record_id: ActiveValue::Set(record_id),
            key: ActiveValue::Set(key.clone()),
            value: ActiveValue::Set(value.clone()),
            ordinal: ActiveValue::Set(ordinal as i32),
            ..Default::default()
        }
        .insert(conn)
        .await?;
    }
    Ok(())
}

fn relative_path(bundle: &Path, path: &Path) -> String {
    path.strip_prefix(bundle)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

/// Resolves every inserted record's `supersedes`/`superseded_by` target
/// against this same sync run's other records and inserts one
/// `kind = "supersede"` relation per resolved link. A record that declares
/// both sides of the same link (the common case left by
/// `living-docs supersede`) yields exactly one row, not two.
async fn insert_supersede_relations<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    inserted: &[InsertedRecord],
) -> Result<()> {
    let lookup = build_relation_lookup(inserted);
    let mut seen = HashSet::new();

    for record in inserted {
        let dir = record_dir(&record.relative_path);

        if let Some(target_id) = record
            .supersedes
            .as_deref()
            .and_then(|raw| resolve_target(&lookup, &dir, raw))
        {
            insert_supersede_relation(conn, project_id, &mut seen, record.id, target_id).await?;
        }

        if let Some(source_id) = record
            .superseded_by
            .as_deref()
            .and_then(|raw| resolve_target(&lookup, &dir, raw))
        {
            insert_supersede_relation(conn, project_id, &mut seen, source_id, record.id).await?;
        }
    }

    Ok(())
}

/// Maps `(sibling directory, zero-padded NNNN)` to a record id, mirroring
/// how `living_docs_core::check::records` resolves a `supersedes`/
/// `superseded_by` target to a sibling `<NNNN>-*.md` file.
fn build_relation_lookup(inserted: &[InsertedRecord]) -> HashMap<(String, String), i32> {
    inserted
        .iter()
        .filter_map(|record| relation_key(&record.relative_path).map(|key| (key, record.id)))
        .collect()
}

fn relation_key(relative_path: &str) -> Option<(String, String)> {
    let path = Path::new(relative_path);
    let dir = path.parent()?.to_string_lossy().into_owned();
    let number = numeric_prefix(path.file_name()?.to_str()?)?;
    Some((dir, number))
}

fn numeric_prefix(filename: &str) -> Option<String> {
    let stem = filename.strip_suffix(".md")?;
    let digits: String = stem.chars().take_while(char::is_ascii_digit).collect();
    normalize_number(&digits)
}

fn normalize_number(raw: &str) -> Option<String> {
    let parsed: u32 = raw.trim().parse().ok()?;
    Some(format!("{parsed:04}"))
}

fn record_dir(relative_path: &str) -> String {
    Path::new(relative_path)
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn resolve_target(
    lookup: &HashMap<(String, String), i32>,
    dir: &str,
    raw_target: &str,
) -> Option<i32> {
    let number = normalize_number(raw_target)?;
    lookup.get(&(dir.to_owned(), number)).copied()
}

/// The `relations.kind` value every supersede edge carries, whether
/// resolved by a full sync ([`insert_supersede_relations`]) or a single
/// write ([`resolve_write_relations`]).
const SUPERSEDE_RELATION_KIND: &str = "supersede";

async fn insert_supersede_relation<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    seen: &mut HashSet<(i32, i32)>,
    from_record_id: i32,
    to_record_id: i32,
) -> Result<()> {
    if !seen.insert((from_record_id, to_record_id)) {
        return Ok(());
    }

    relations::ActiveModel {
        project_id: ActiveValue::Set(project_id),
        from_record_id: ActiveValue::Set(from_record_id),
        to_record_id: ActiveValue::Set(to_record_id),
        kind: ActiveValue::Set(SUPERSEDE_RELATION_KIND.to_owned()),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    Ok(())
}

/// Upserts each inserted record's tags by `(project_id, name)` and links
/// them via `record_tags`. Safe against the `UNIQUE(project_id, name)`
/// constraint because [`clear_project`] already emptied this project's tags
/// before either pass runs, so a name is inserted at most once per run.
async fn insert_tags<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    inserted: &[InsertedRecord],
) -> Result<()> {
    let mut tag_ids: HashMap<String, i32> = HashMap::new();

    for record in inserted {
        for name in &record.tags {
            let tag_id = ensure_tag(conn, project_id, &mut tag_ids, name).await?;
            record_tags::ActiveModel {
                record_id: ActiveValue::Set(record.id),
                tag_id: ActiveValue::Set(tag_id),
            }
            .insert(conn)
            .await?;
        }
    }

    Ok(())
}

async fn ensure_tag<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    cache: &mut HashMap<String, i32>,
    name: &str,
) -> Result<i32> {
    if let Some(&id) = cache.get(name) {
        return Ok(id);
    }

    let inserted = tags::ActiveModel {
        project_id: ActiveValue::Set(project_id),
        name: ActiveValue::Set(name.to_owned()),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    cache.insert(name.to_owned(), inserted.id);
    Ok(inserted.id)
}

/// Upserts one record by `(project_id, path)` from an already-extracted
/// write, replacing its frontmatter tail and best-effort re-resolving its
/// `supersedes`/`superseded_by` relations and tags against the project's
/// already-persisted records (ADR 0007, issue 0006 slice 0006-C2). Runs in
/// its own transaction so a mid-write failure leaves no partial row.
pub(crate) async fn upsert_record(
    conn: &DatabaseConnection,
    project_id: i32,
    path: &str,
    extracted: ExtractedRecord,
) -> Result<()> {
    let txn = conn.begin().await?;

    let record_id = upsert_record_row(&txn, project_id, path, &extracted).await?;
    replace_frontmatter_tail(&txn, record_id, &extracted.frontmatter_tail).await?;
    resolve_write_relations(&txn, project_id, path, record_id, &extracted).await?;
    replace_write_tags(&txn, project_id, record_id, &extracted.tags).await?;

    txn.commit().await?;
    Ok(())
}

async fn upsert_record_row<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    path: &str,
    extracted: &ExtractedRecord,
) -> Result<i32> {
    match find_record(conn, project_id, path).await? {
        Some(existing) => update_record_row(conn, existing.id, extracted).await,
        None => insert_record_row(conn, project_id, path, extracted).await,
    }
}

async fn find_record<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    path: &str,
) -> Result<Option<Model>> {
    Records::find()
        .filter(Column::ProjectId.eq(project_id))
        .filter(Column::Path.eq(path))
        .one(conn)
        .await
}

async fn insert_record_row<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    path: &str,
    extracted: &ExtractedRecord,
) -> Result<i32> {
    let inserted = ActiveModel {
        project_id: ActiveValue::Set(project_id),
        path: ActiveValue::Set(path.to_owned()),
        doc_type: ActiveValue::Set(extracted.doc_type.clone()),
        number: ActiveValue::Set(extracted.number),
        concept_id: ActiveValue::Set(extracted.concept_id.clone()),
        identity_kind: ActiveValue::Set(extracted.identity_kind.clone()),
        title: ActiveValue::Set(extracted.title.clone()),
        description: ActiveValue::Set(extracted.description.clone()),
        body: ActiveValue::Set(extracted.body.clone()),
        ..Default::default()
    }
    .insert(conn)
    .await?;
    Ok(inserted.id)
}

async fn update_record_row<C: ConnectionTrait>(
    conn: &C,
    record_id: i32,
    extracted: &ExtractedRecord,
) -> Result<i32> {
    let model = ActiveModel {
        id: ActiveValue::Set(record_id),
        doc_type: ActiveValue::Set(extracted.doc_type.clone()),
        number: ActiveValue::Set(extracted.number),
        concept_id: ActiveValue::Set(extracted.concept_id.clone()),
        identity_kind: ActiveValue::Set(extracted.identity_kind.clone()),
        title: ActiveValue::Set(extracted.title.clone()),
        description: ActiveValue::Set(extracted.description.clone()),
        body: ActiveValue::Set(extracted.body.clone()),
        ..Default::default()
    };
    let updated = model.update(conn).await?;
    Ok(updated.id)
}

async fn replace_frontmatter_tail<C: ConnectionTrait>(
    conn: &C,
    record_id: i32,
    tail: &[(String, String)],
) -> Result<()> {
    frontmatter_fields::Entity::delete_many()
        .filter(frontmatter_fields::Column::RecordId.eq(record_id))
        .exec(conn)
        .await?;
    insert_frontmatter_tail(conn, record_id, tail).await
}

/// Resolves `record_id`'s `supersedes`/`superseded_by` frontmatter against
/// the project's already-persisted records — not just this write's own
/// batch, the way [`insert_supersede_relations`] resolves during a full
/// sync — and links any match. A target that does not (yet) exist is
/// skipped, not inserted as a dangling relation; the FK is the backstop.
async fn resolve_write_relations<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    path: &str,
    record_id: i32,
    extracted: &ExtractedRecord,
) -> Result<()> {
    let dir = record_dir(path);

    if let Some(target_id) =
        resolve_write_target(conn, project_id, &dir, extracted.supersedes.as_deref()).await?
    {
        insert_supersede_relation_if_absent(conn, project_id, record_id, target_id).await?;
    }
    if let Some(source_id) =
        resolve_write_target(conn, project_id, &dir, extracted.superseded_by.as_deref()).await?
    {
        insert_supersede_relation_if_absent(conn, project_id, source_id, record_id).await?;
    }
    Ok(())
}

async fn resolve_write_target<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    dir: &str,
    raw_target: Option<&str>,
) -> Result<Option<i32>> {
    let Some(raw_target) = raw_target else {
        return Ok(None);
    };
    let Some(number) = normalize_number(raw_target) else {
        return Ok(None);
    };
    find_record_id_in_dir(conn, project_id, dir, &number).await
}

/// The id of the record in `project_id` whose path sits directly under
/// `dir` and whose `number` matches `zero_padded_number`, mirroring
/// [`relation_key`]'s sibling-directory resolution but querying persisted
/// rows instead of one sync run's in-memory batch.
async fn find_record_id_in_dir<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    dir: &str,
    zero_padded_number: &str,
) -> Result<Option<i32>> {
    let Ok(number) = zero_padded_number.parse::<i32>() else {
        return Ok(None);
    };
    let prefix = format!("{dir}/");
    let candidates = Records::find()
        .filter(Column::ProjectId.eq(project_id))
        .filter(Column::Number.eq(number))
        .all(conn)
        .await?;
    Ok(candidates
        .into_iter()
        .find(|record| record.path.starts_with(&prefix))
        .map(|record| record.id))
}

async fn insert_supersede_relation_if_absent<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    from_record_id: i32,
    to_record_id: i32,
) -> Result<()> {
    let exists = relations::Entity::find()
        .filter(relations::Column::Kind.eq(SUPERSEDE_RELATION_KIND))
        .filter(relations::Column::FromRecordId.eq(from_record_id))
        .filter(relations::Column::ToRecordId.eq(to_record_id))
        .one(conn)
        .await?
        .is_some();
    if exists {
        return Ok(());
    }
    insert_supersede_relation(
        conn,
        project_id,
        &mut HashSet::new(),
        from_record_id,
        to_record_id,
    )
    .await
}

/// Replaces `record_id`'s tag links with `tag_names`, creating any tag
/// `project_id` does not already have. Looks tags up by name against the
/// database rather than assuming absence the way [`insert_tags`]'s cache
/// does during a from-empty sync run, so re-writing a record that reuses an
/// existing project tag does not violate `UNIQUE(project_id, name)`.
async fn replace_write_tags<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    record_id: i32,
    tag_names: &[String],
) -> Result<()> {
    record_tags::Entity::delete_many()
        .filter(record_tags::Column::RecordId.eq(record_id))
        .exec(conn)
        .await?;

    for name in tag_names {
        let tag_id = find_or_create_tag(conn, project_id, name).await?;
        record_tags::ActiveModel {
            record_id: ActiveValue::Set(record_id),
            tag_id: ActiveValue::Set(tag_id),
        }
        .insert(conn)
        .await?;
    }
    Ok(())
}

async fn find_or_create_tag<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    name: &str,
) -> Result<i32> {
    if let Some(existing) = tags::Entity::find()
        .filter(tags::Column::ProjectId.eq(project_id))
        .filter(tags::Column::Name.eq(name))
        .one(conn)
        .await?
    {
        return Ok(existing.id);
    }

    let inserted = tags::ActiveModel {
        project_id: ActiveValue::Set(project_id),
        name: ActiveValue::Set(name.to_owned()),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    Ok(inserted.id)
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

    /// A single-record corpus at `bundle_root`, always relative-pathed
    /// `adr/0001-quokka-caching.md` regardless of `bundle_root` — lets a
    /// test sync two different projects that each carry a record at the
    /// same relative path, to exercise project-scoped path lookups.
    pub(crate) fn single_record_corpus_at(
        bundle_root: &str,
        title: &str,
    ) -> (MemoryStore, PathBuf) {
        let bundle = PathBuf::from(bundle_root);
        let doc = format!(
            "---\ntype: ADR\ntitle: {title}\ndescription: d.\nstatus: Accepted\n---\n# {title}\n\nBody.\n"
        );
        let mut files = BTreeMap::new();
        files.insert(bundle.join("adr").join("0001-quokka-caching.md"), doc);
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

    #[tokio::test]
    async fn sync_project_upserts_a_named_project_and_scopes_records_to_it() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");
        let (store, bundle) = seeded_corpus();

        let count = sync_project(&conn, &store, &bundle, "team-a")
            .await
            .expect("sync_project");

        assert_eq!(count, 2);
        let project = projects::Entity::find()
            .filter(projects::Column::Slug.eq("team-a"))
            .one(&conn)
            .await
            .expect("query project")
            .expect("sync_project upserts the named project");
        let records = all_records(&conn).await;
        assert_eq!(records.len(), 2);
        let stored = Records::find()
            .filter(Column::ProjectId.eq(project.id))
            .all(&conn)
            .await
            .expect("query records for project");
        assert_eq!(stored.len(), 2);
    }
}
