//! Write-path fitness tests for `DbDocStore` (ADR 0007, issue 0006 slice
//! 0006-C2): `write` parses canonical markdown into path-derived identity,
//! upserts by `(project_id, path)`, replaces the frontmatter tail, and
//! best-effort resolves supersede relations and tags against the project's
//! already-persisted records; a write/read round trip is a fixed point.
//! Also covers the `sync.rs::insert_record` call-site fix (a concept doc
//! synced under a non-empty bundle root gets a project-relative
//! `concept_id`) and backend-agnostic number allocation via
//! `next_number_from_store`, matching between an fs-backed store and a
//! `DbDocStore` seeded through `write`.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use db_store::entity::{records, relations, tags};
use db_store::{connect, migrate, sync_project, DbDocStore};
use living_docs_core::commands::next::next_number_from_store;
use living_docs_core::store::DocStore;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

struct EmptyStore;

impl DocStore for EmptyStore {
    fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    fn read(&self, _path: &Path) -> io::Result<String> {
        Err(io::Error::new(io::ErrorKind::NotFound, "not found"))
    }

    fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
        Ok(())
    }
}

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

/// Mirrors `fs-store`'s recursive `.md` walk (not reused directly —
/// `living-docs-core` and `db-store` cannot depend on the `fs-store`
/// adapter crate, and every test file in this crate already carries its own
/// small local `DocStore`, per `parity.rs`).
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

fn temp_sqlite_url(label: &str) -> (PathBuf, String) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let path = std::env::temp_dir()
        .join(format!("living-docs-db-store-authoring-{label}-{nanos}"))
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

fn scratch_bundle_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "living-docs-db-store-authoring-bundle-{label}-{nanos}"
    ))
}

fn write_scratch_doc(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create scratch bundle directory");
    }
    fs::write(path, contents).expect("write scratch bundle file");
}

/// Creates a fresh SQLite file, migrates it, and syncs an empty corpus so
/// `project_slug` exists — `DbDocStore::for_project`/`new` only open an
/// already-synced project, and `write` needs no seeded records of its own.
fn setup_empty_project(bundle: &Path, project_slug: &str, label: &str) -> (PathBuf, String) {
    let (db_path, db_url) = temp_sqlite_url(label);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build setup runtime");
    runtime.block_on(async {
        let conn = connect(&db_url).await.expect("connect");
        migrate(&conn).await.expect("migrate");
        sync_project(&conn, &EmptyStore, bundle, project_slug)
            .await
            .expect("sync empty corpus to create the project row");
    });
    (db_path, db_url)
}

async fn project_id_for(conn: &DatabaseConnection, slug: &str) -> i32 {
    db_store::entity::projects::Entity::find()
        .filter(db_store::entity::projects::Column::Slug.eq(slug))
        .one(conn)
        .await
        .expect("query project")
        .unwrap_or_else(|| panic!("project '{slug}' was not synced"))
        .id
}

async fn record_row(conn: &DatabaseConnection, project_id: i32, path: &str) -> records::Model {
    records::Entity::find()
        .filter(records::Column::ProjectId.eq(project_id))
        .filter(records::Column::Path.eq(path))
        .one(conn)
        .await
        .expect("query record")
        .unwrap_or_else(|| panic!("record at {path} was not persisted"))
}

const ADR_ONE: &str =
    "---\ntype: ADR\ntitle: First Decision\ndescription: d.\nstatus: Accepted\n---\n# First Decision\n\nBody.\n";

#[test]
fn write_then_read_round_trips_a_new_numbered_record() {
    let bundle = scratch_bundle_root("round-trip");
    let (db_path, db_url) = setup_empty_project(&bundle, "default", "round-trip");
    let db_store = DbDocStore::new(&db_url, bundle.clone()).expect("open db doc store");
    let target = bundle.join("adr").join("0001-first-decision.md");

    db_store
        .write(&target, ADR_ONE)
        .expect("write should persist the new record");
    let markdown = db_store.read(&target).expect("read the written record");

    let reparsed = db_store::record::extract_record(&target, &markdown);
    let original = db_store::record::extract_record(&target, ADR_ONE);
    assert_eq!(reparsed.doc_type, original.doc_type);
    assert_eq!(reparsed.title, original.title);
    assert_eq!(reparsed.description, original.description);
    assert_eq!(reparsed.number, original.number);
    assert_eq!(reparsed.concept_id, original.concept_id);
    assert_eq!(reparsed.identity_kind, original.identity_kind);

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn write_replaces_an_existing_records_frontmatter_tail_on_rewrite() {
    let bundle = scratch_bundle_root("tail-replace");
    let (db_path, db_url) = setup_empty_project(&bundle, "default", "tail-replace");
    let db_store = DbDocStore::new(&db_url, bundle.clone()).expect("open db doc store");
    let target = bundle.join("adr").join("0001-decision.md");

    let first = "---\ntype: ADR\ntitle: Decision\ndescription: d.\nstatus: Draft\ntracker: JIRA-1\n---\n# Decision\n\nBody.\n";
    db_store
        .write(&target, first)
        .expect("first write should succeed");

    let second = "---\ntype: ADR\ntitle: Decision\ndescription: d.\nstatus: Accepted\nlabels: important\n---\n# Decision\n\nBody.\n";
    db_store
        .write(&target, second)
        .expect("second write should succeed and replace the tail");

    let markdown = db_store.read(&target).expect("read the rewritten record");
    assert!(markdown.contains("status: Accepted"));
    assert!(markdown.contains("labels: important"));
    assert!(
        !markdown.contains("tracker:"),
        "the stale tracker field from the first write must not survive a rewrite"
    );
    assert!(
        !markdown.contains("Draft"),
        "the stale status value from the first write must not survive a rewrite"
    );

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn write_resolves_supersedes_against_an_existing_project_record() {
    let bundle = scratch_bundle_root("relations");
    let (db_path, db_url) = setup_empty_project(&bundle, "default", "relations");
    let db_store = DbDocStore::new(&db_url, bundle.clone()).expect("open db doc store");

    let old_target = bundle.join("adr").join("0001-old-decision.md");
    let old_doc = "---\ntype: ADR\ntitle: Old Decision\ndescription: d.\nstatus: Superseded\n---\n# Old Decision\n\nBody.\n";
    db_store
        .write(&old_target, old_doc)
        .expect("seed the superseded record");

    let new_target = bundle.join("adr").join("0002-new-decision.md");
    let new_doc = "---\ntype: ADR\ntitle: New Decision\ndescription: d.\nstatus: Accepted\nsupersedes: 0001\n---\n# New Decision\n\nBody.\n";
    db_store
        .write(&new_target, new_doc)
        .expect("write the superseding record");

    let markdown = db_store
        .read(&new_target)
        .expect("read the superseding record");
    assert!(markdown.contains("supersedes: 0001"));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build introspection runtime");
    let relation_count: usize = runtime.block_on(async {
        let conn = connect(&db_url).await.expect("connect for introspection");
        let project_id = project_id_for(&conn, "default").await;
        let old_id = record_row(&conn, project_id, "adr/0001-old-decision.md")
            .await
            .id;
        let new_id = record_row(&conn, project_id, "adr/0002-new-decision.md")
            .await
            .id;
        relations::Entity::find()
            .filter(relations::Column::ProjectId.eq(project_id))
            .filter(relations::Column::FromRecordId.eq(new_id))
            .filter(relations::Column::ToRecordId.eq(old_id))
            .all(&conn)
            .await
            .expect("query relations")
            .len()
    });
    drop(runtime);

    assert_eq!(
        relation_count, 1,
        "exactly one supersede relation must link the new record to the old one"
    );

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn write_replaces_tag_links_on_rewrite() {
    let bundle = scratch_bundle_root("tags");
    let (db_path, db_url) = setup_empty_project(&bundle, "default", "tags");
    let db_store = DbDocStore::new(&db_url, bundle.clone()).expect("open db doc store");
    let target = bundle.join("adr").join("0001-tagged.md");

    let first = "---\ntype: ADR\ntitle: Tagged\ndescription: d.\ntags: [caching, performance]\n---\n# Tagged\n\nBody.\n";
    db_store
        .write(&target, first)
        .expect("first write should succeed");

    let second =
        "---\ntype: ADR\ntitle: Tagged\ndescription: d.\ntags: [performance]\n---\n# Tagged\n\nBody.\n";
    db_store
        .write(&target, second)
        .expect("second write should succeed");

    let markdown = db_store.read(&target).expect("read the rewritten record");
    assert!(markdown.contains("tags:"));
    assert!(markdown.contains("performance"));
    assert!(
        !markdown.contains("caching"),
        "a tag dropped by rewrite must no longer link to the record"
    );

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build introspection runtime");
    let tag_names: Vec<String> = runtime.block_on(async {
        let conn = connect(&db_url).await.expect("connect for introspection");
        let project_id = project_id_for(&conn, "default").await;
        tags::Entity::find()
            .filter(tags::Column::ProjectId.eq(project_id))
            .filter(tags::Column::Name.eq("performance"))
            .all(&conn)
            .await
            .expect("query tags")
            .into_iter()
            .map(|tag| tag.name)
            .collect()
    });
    drop(runtime);

    assert_eq!(tag_names, vec!["performance".to_owned()]);

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn write_refuses_a_numbered_doc_type_whose_filename_lacks_a_valid_nnnn_prefix() {
    let bundle = scratch_bundle_root("xor-violation");
    let (db_path, db_url) = setup_empty_project(&bundle, "default", "xor-violation");
    let db_store = DbDocStore::new(&db_url, bundle.clone()).expect("open db doc store");

    let target = bundle.join("adr").join("no-number-here.md");
    let contents =
        "---\ntype: ADR\ntitle: Missing Number\ndescription: d.\n---\n# Missing Number\n\nBody.\n";

    let error = db_store
        .write(&target, contents)
        .expect_err("a numbered doc type with no NNNN prefix must be refused");
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("no-number-here.md"));

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn write_succeeds_and_is_retrievable_for_a_well_formed_numbered_record() {
    let bundle = scratch_bundle_root("xor-well-formed");
    let (db_path, db_url) = setup_empty_project(&bundle, "default", "xor-well-formed");
    let db_store = DbDocStore::new(&db_url, bundle.clone()).expect("open db doc store");

    let target = bundle.join("adr").join("0001-well-formed.md");
    let contents =
        "---\ntype: ADR\ntitle: Well Formed\ndescription: d.\n---\n# Well Formed\n\nBody.\n";

    db_store
        .write(&target, contents)
        .expect("a well-formed write must succeed");
    let markdown = db_store
        .read(&target)
        .expect("the well-formed record must be retrievable afterward");
    assert!(markdown.contains("title: Well Formed"));

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&bundle);
}

const CONCEPT_DOC: &str =
    "---\ntype: Glossary\ntitle: Findability\ndescription: d.\n---\n# Findability\n\nBody.\n";

#[tokio::test]
async fn sync_project_derives_a_bundle_relative_concept_id_under_a_non_empty_bundle_root() {
    let conn = connect_in_memory_and_migrated().await;
    let bundle = PathBuf::from("/nested/bundle-root");
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("glossary").join("findability.md"),
        CONCEPT_DOC.to_owned(),
    );
    let store = MemoryStore { files };

    sync_project(&conn, &store, &bundle, "team-a")
        .await
        .expect("sync team-a");

    let project_id = project_id_for(&conn, "team-a").await;
    let record = record_row(&conn, project_id, "glossary/findability.md").await;

    assert_eq!(
        record.concept_id,
        Some("glossary/findability".to_owned()),
        "concept_id must be project-relative, carrying no bundle-root prefix"
    );
}

async fn connect_in_memory_and_migrated() -> DatabaseConnection {
    let conn = db_store::connect_in_memory()
        .await
        .expect("connect to in-memory sqlite");
    migrate(&conn).await.expect("migrate");
    conn
}

#[test]
fn next_number_from_store_returns_the_same_result_on_fs_backed_and_db_backed_stores() {
    let bundle = scratch_bundle_root("next-number-parity");
    let adr_one = bundle.join("adr").join("0001-first.md");
    let adr_three = bundle.join("adr").join("0003-third.md");
    write_scratch_doc(&adr_one, ADR_ONE);
    write_scratch_doc(
        &adr_three,
        "---\ntype: ADR\ntitle: Third Decision\ndescription: d.\n---\n# Third Decision\n\nBody.\n",
    );

    let fs_next = next_number_from_store(&LocalFsStore, &bundle, "adr")
        .expect("fs-backed next_number_from_store should succeed");
    assert_eq!(fs_next, 4);

    let (db_path, db_url) = setup_empty_project(&bundle, "default", "next-number-parity");
    let db_store = DbDocStore::new(&db_url, bundle.clone()).expect("open db doc store");
    db_store
        .write(&adr_one, ADR_ONE)
        .expect("seed 0001 via write");
    db_store
        .write(
            &adr_three,
            "---\ntype: ADR\ntitle: Third Decision\ndescription: d.\n---\n# Third Decision\n\nBody.\n",
        )
        .expect("seed 0003 via write");

    let db_next = next_number_from_store(&db_store, &bundle, "adr")
        .expect("db-backed next_number_from_store should succeed");

    assert_eq!(db_next, fs_next);

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn next_number_from_store_returns_one_for_an_empty_project() {
    let bundle = scratch_bundle_root("next-number-empty");
    let (db_path, db_url) = setup_empty_project(&bundle, "default", "next-number-empty");
    let db_store = DbDocStore::new(&db_url, bundle.clone()).expect("open db doc store");

    let next = next_number_from_store(&db_store, &bundle, "adr")
        .expect("next_number_from_store should succeed on an empty project");

    assert_eq!(next, 1);

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&bundle);
}
