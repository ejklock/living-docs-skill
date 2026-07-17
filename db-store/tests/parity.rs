//! Read-side fitness tests for db-store's `DocStore` adapter (ADR 0007,
//! issue 0006 slice 0006-B; identity sourced from the record's path rather
//! than frontmatter, issue 0006 slice 0006-C1): `DbDocStore::read` is a
//! fixed point of `extract_record`/`to_canonical_markdown` for a synced
//! record, `check` runs identically over an fs-backed and a db-backed
//! `DocStore` for the repo's own `docs/` corpus, a seeded frontmatter
//! violation fails on both backends, and the reconstructed frontmatter
//! tail preserves both its field order and its concrete ordinal sequence.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use db_store::entity::{frontmatter_fields, projects, records};
use db_store::{connect, migrate, sync_project, DbDocStore};
use living_docs_core::store::DocStore;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

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

/// Mirrors `fs-store`'s recursive `.md` walk (not reused directly — every
/// test file in this crate carries its own small local `DocStore`, the
/// established convention in `ingestion.rs`/`dual_engine.rs`).
struct LocalFsStore;

impl DocStore for LocalFsStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        let mut found = Vec::new();
        collect_md_files(root, &mut found);
        found.sort();
        Ok(found)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)
    }
}

fn collect_md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

const OLD_DOC: &str = "---\ntype: ADR\ntitle: Quokka Caching Strategy\ndescription: Adopt quokka-based caching for the read model.\nstatus: Superseded\nsuperseded_by: 0002\ntags: [caching]\nlabels: legacy\ntracker: JIRA-100\ntimestamp: 2026-07-01T00:00:00Z\n---\n# 0001. Quokka Caching Strategy\n\nBody.\n";
const NEW_DOC: &str = "---\ntype: ADR\ntitle: Improved Caching Strategy\ndescription: Supersedes quokka caching.\nstatus: Accepted\nsupersedes: 0001\ntags: [caching, performance]\ntracker: JIRA-101\ntimestamp: 2026-07-17T00:00:00Z\n---\n# 0002. Improved Caching Strategy\n\nBody.\n";

fn supersede_corpus() -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from("/bundle-parity-supersede");
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

const CONCEPT_DOC: &str = "---\ntype: Glossary\ntitle: Findability\ndescription: The ease of locating a doc via search or convention.\nstatus: Active\ntags: [glossary]\ntimestamp: 2026-07-17T00:00:00Z\n---\n# Findability\n\nBody.\n";

fn concept_corpus() -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from("/bundle-parity-concept");
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("glossary").join("findability.md"),
        CONCEPT_DOC.to_owned(),
    );
    (MemoryStore { files }, bundle)
}

const TAILED_DOC: &str = "---\ntype: ADR\ntitle: Tailed Decision\ndescription: d.\nstatus: Accepted\nlabels: important\nblocked_by: 0002\ntracker: JIRA-42\ntimestamp: 2026-07-17T00:00:00Z\n---\n# 0001. Tailed Decision\n\nBody.\n";

fn tailed_corpus() -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from("/bundle-parity-tailed");
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("adr").join("0001-tailed-decision.md"),
        TAILED_DOC.to_owned(),
    );
    (MemoryStore { files }, bundle)
}

fn temp_sqlite_url(label: &str) -> (PathBuf, String) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let path = std::env::temp_dir()
        .join(format!("living-docs-db-store-parity-{label}-{nanos}"))
        .join("index.db");
    let url = format!("sqlite://{}?mode=rwc", path.display());
    (path, url)
}

fn cleanup_sqlite_file(db_path: &Path) {
    let _ = fs::remove_file(db_path);
    if let Some(parent) = db_path.parent() {
        let _ = fs::remove_dir(parent);
    }
}

/// Asserts the fields AC B1 (issue 0006 slice 0006-B) names as the fixed
/// point: `doc_type`, `title`, `description`, the typed identity,
/// `supersedes`/`superseded_by`, `tags`, and the frontmatter tail. `body`
/// is deliberately excluded — the canonical serializer always inserts a
/// blank line before the body (AC B3), so a source body that already
/// started with a blank line round-trips with an extra one; AC B1 does not
/// list `body` among the fields the fixed point covers.
fn assert_round_trips(reparsed: &db_store::ExtractedRecord, original: &db_store::ExtractedRecord) {
    assert_eq!(reparsed.doc_type, original.doc_type);
    assert_eq!(reparsed.title, original.title);
    assert_eq!(reparsed.description, original.description);
    assert_eq!(reparsed.number, original.number);
    assert_eq!(reparsed.concept_id, original.concept_id);
    assert_eq!(reparsed.identity_kind, original.identity_kind);
    assert_eq!(reparsed.supersedes, original.supersedes);
    assert_eq!(reparsed.superseded_by, original.superseded_by);
    assert_eq!(reparsed.tags, original.tags);
    assert_eq!(reparsed.frontmatter_tail, original.frontmatter_tail);
}

/// Runs `migrate` + `sync_project` on a dedicated setup runtime, then drops
/// it — [`DbDocStore`] opens its own runtime + connection afterward, and a
/// sync trait method's `block_on` panics if called from inside an already
/// -running tokio runtime (mirrors the `DbSearchIndex` test in `lib.rs`).
/// A file-backed SQLite database, not `sqlite::memory:`, is required so the
/// two connections (this one, and `DbDocStore`'s own) see the same data —
/// an in-memory database is only visible to the connection that opened it.
fn setup_synced_db(url: &str, store: &dyn DocStore, bundle: &Path, project_slug: &str) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build setup runtime");
    runtime.block_on(async {
        let conn = connect(url).await.expect("connect");
        migrate(&conn).await.expect("migrate");
        sync_project(&conn, store, bundle, project_slug)
            .await
            .expect("sync project");
    });
}

#[test]
fn db_read_of_a_synced_numbered_record_reparses_to_the_same_extracted_record() {
    let (db_path, db_url) = temp_sqlite_url("fixed-point-numbered");
    let (store, bundle) = supersede_corpus();
    setup_synced_db(&db_url, &store, &bundle, "team-a");

    let db_store =
        DbDocStore::for_project(&db_url, bundle.clone(), "team-a").expect("open db doc store");
    let path = bundle.join("adr").join("0001-quokka-caching.md");

    let markdown = db_store.read(&path).expect("read canonical markdown");
    let reparsed = db_store::record::extract_record(&path, &markdown);
    let original = db_store::record::extract_record(&path, OLD_DOC);

    assert_round_trips(&reparsed, &original);

    cleanup_sqlite_file(&db_path);
}

#[test]
fn db_read_of_the_supersede_target_reparses_with_its_own_supersedes_field() {
    let (db_path, db_url) = temp_sqlite_url("fixed-point-supersedes");
    let (store, bundle) = supersede_corpus();
    setup_synced_db(&db_url, &store, &bundle, "team-a");

    let db_store =
        DbDocStore::for_project(&db_url, bundle.clone(), "team-a").expect("open db doc store");
    let path = bundle.join("adr").join("0002-improved-caching.md");

    let markdown = db_store.read(&path).expect("read canonical markdown");
    let reparsed = db_store::record::extract_record(&path, &markdown);
    let original = db_store::record::extract_record(&path, NEW_DOC);

    assert_round_trips(&reparsed, &original);
    assert_eq!(reparsed.supersedes, Some("0001".to_owned()));

    cleanup_sqlite_file(&db_path);
}

#[test]
fn db_read_of_a_synced_concept_record_reparses_to_the_same_extracted_record() {
    let (db_path, db_url) = temp_sqlite_url("fixed-point-concept");
    let (store, bundle) = concept_corpus();
    setup_synced_db(&db_url, &store, &bundle, "team-a");

    let db_store =
        DbDocStore::for_project(&db_url, bundle.clone(), "team-a").expect("open db doc store");
    let path = bundle.join("glossary").join("findability.md");

    let markdown = db_store.read(&path).expect("read canonical markdown");
    let reparsed = db_store::record::extract_record(&path, &markdown);
    let original = db_store::record::extract_record(&path, CONCEPT_DOC);

    assert_round_trips(&reparsed, &original);

    cleanup_sqlite_file(&db_path);
}

#[test]
fn db_read_of_an_unknown_path_returns_a_not_found_error() {
    let (db_path, db_url) = temp_sqlite_url("read-unknown-path");
    let (store, bundle) = concept_corpus();
    setup_synced_db(&db_url, &store, &bundle, "team-a");

    let db_store =
        DbDocStore::for_project(&db_url, bundle.clone(), "team-a").expect("open db doc store");
    let missing = bundle.join("glossary").join("does-not-exist.md");

    let error = db_store
        .read(&missing)
        .expect_err("reading an unsynced path must fail");
    assert_eq!(error.kind(), io::ErrorKind::NotFound);

    cleanup_sqlite_file(&db_path);
}

#[test]
fn db_doc_store_write_reports_it_is_deferred_to_slice_0006_c() {
    let (db_path, db_url) = temp_sqlite_url("write-deferred");
    let (store, bundle) = concept_corpus();
    setup_synced_db(&db_url, &store, &bundle, "team-a");

    let db_store =
        DbDocStore::for_project(&db_url, bundle.clone(), "team-a").expect("open db doc store");
    let target = bundle.join("glossary").join("new-concept.md");

    let error = db_store
        .write(&target, "content")
        .expect_err("write is not yet implemented");
    assert!(error.to_string().contains("0006-C"));

    cleanup_sqlite_file(&db_path);
}

async fn record_row(conn: &DatabaseConnection, project_id: i32, path: &str) -> records::Model {
    records::Entity::find()
        .filter(records::Column::ProjectId.eq(project_id))
        .filter(records::Column::Path.eq(path))
        .one(conn)
        .await
        .expect("query record")
        .unwrap_or_else(|| panic!("record at {path} was not synced"))
}

async fn project_id_for(conn: &DatabaseConnection, slug: &str) -> i32 {
    projects::Entity::find()
        .filter(projects::Column::Slug.eq(slug))
        .one(conn)
        .await
        .expect("query project")
        .unwrap_or_else(|| panic!("project '{slug}' was not synced"))
        .id
}

#[test]
fn reconstructed_tail_preserves_both_field_order_and_the_concrete_ordinal_sequence() {
    let (db_path, db_url) = temp_sqlite_url("field-order-and-ordinals");
    let (store, bundle) = tailed_corpus();
    setup_synced_db(&db_url, &store, &bundle, "team-a");

    let setup_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build introspection runtime");
    let (ordinals, keys) = setup_runtime.block_on(async {
        let conn = connect(&db_url).await.expect("connect for introspection");
        let project_id = project_id_for(&conn, "team-a").await;
        let record = record_row(&conn, project_id, "adr/0001-tailed-decision.md").await;
        let tail = frontmatter_fields::Entity::find()
            .filter(frontmatter_fields::Column::RecordId.eq(record.id))
            .order_by_asc(frontmatter_fields::Column::Ordinal)
            .all(&conn)
            .await
            .expect("query frontmatter_fields");
        let ordinals: Vec<i32> = tail.iter().map(|row| row.ordinal).collect();
        let keys: Vec<String> = tail.iter().map(|row| row.key.clone()).collect();
        (ordinals, keys)
    });
    drop(setup_runtime);

    assert_eq!(
        ordinals,
        vec![0, 1, 2, 3, 4],
        "ordinals must be the sequential 0..N encounter positions, not a constant"
    );
    assert_eq!(
        keys,
        vec!["status", "labels", "blocked_by", "tracker", "timestamp"]
    );

    let db_store =
        DbDocStore::for_project(&db_url, bundle.clone(), "team-a").expect("open db doc store");
    let path = bundle.join("adr").join("0001-tailed-decision.md");
    let markdown = db_store.read(&path).expect("read canonical markdown");

    assert_eq!(
        markdown,
        "---\n\
         type: ADR\n\
         title: Tailed Decision\n\
         description: d.\n\
         status: Accepted\n\
         labels: important\n\
         blocked_by: 0002\n\
         tracker: JIRA-42\n\
         timestamp: 2026-07-17T00:00:00Z\n\
         ---\n\
         \n\
         # 0001. Tailed Decision\n\n\
         Body.\n"
    );
    assert!(!markdown.contains("number:"));

    cleanup_sqlite_file(&db_path);
}

fn repo_docs_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("docs")
        .canonicalize()
        .expect("the repo's own docs/ directory must exist")
}

#[test]
fn check_over_db_store_matches_check_over_fs_store_for_the_repos_own_docs() {
    let docs_root = repo_docs_root();
    let (db_path, db_url) = temp_sqlite_url("check-parity-docs");
    setup_synced_db(&db_url, &LocalFsStore, &docs_root, "default");

    let db_store = DbDocStore::new(&db_url, docs_root.clone()).expect("open db doc store");

    let fs_verdict = living_docs_core::check::run(&LocalFsStore, &docs_root);
    let db_verdict = living_docs_core::check::run(&db_store, &docs_root);

    assert_eq!(
        format!("{fs_verdict:?}"),
        format!("{db_verdict:?}"),
        "check must reach the same verdict over fs-store and db-store for the same corpus"
    );

    cleanup_sqlite_file(&db_path);
}

fn write_scratch_doc(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create scratch bundle directory");
    }
    fs::write(path, contents).expect("write scratch bundle file");
}

fn scratch_bundle_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "living-docs-db-store-parity-bundle-{label}-{nanos}"
    ))
}

/// A minimal bundle with one seeded violation: `adr/0001-broken.md` carries
/// no frontmatter at all, which `check`'s per-file OKF invariant rejects.
/// Everything else (root/dir indexes, membership, reachability) is
/// deliberately well-formed so the violation is the only difference between
/// a clean run and this one.
fn seed_frontmatter_violation_bundle(root: &Path) {
    write_scratch_doc(
        &root.join("index.md"),
        "# Index\n\n- [ADRs](/adr/index.md)\n",
    );
    write_scratch_doc(
        &root.join("adr").join("index.md"),
        "# ADR Index\n\n- [Broken](/adr/0001-broken.md)\n",
    );
    write_scratch_doc(
        &root.join("adr").join("0001-broken.md"),
        "# Broken\n\nThis record carries no frontmatter at all.\n",
    );
}

#[test]
fn check_over_db_store_fails_identically_to_fs_store_on_a_seeded_frontmatter_violation() {
    let root = scratch_bundle_root("frontmatter-violation");
    seed_frontmatter_violation_bundle(&root);

    let (db_path, db_url) = temp_sqlite_url("check-parity-violation");
    setup_synced_db(&db_url, &LocalFsStore, &root, "default");
    let db_store = DbDocStore::new(&db_url, root.clone()).expect("open db doc store");

    let fs_verdict = living_docs_core::check::run(&LocalFsStore, &root);
    let db_verdict = living_docs_core::check::run(&db_store, &root);

    assert_ne!(
        format!("{fs_verdict:?}"),
        format!("{:?}", std::process::ExitCode::SUCCESS),
        "sanity: fs-store must fail on the seeded violation"
    );
    assert_eq!(
        format!("{fs_verdict:?}"),
        format!("{db_verdict:?}"),
        "a seeded frontmatter violation must fail identically on both backends"
    );

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&root);
}
