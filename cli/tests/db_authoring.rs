use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("living-docs-db-authoring-test-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn temp_sqlite_url(label: &str) -> (PathBuf, String) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir()
        .join(format!("living-docs-db-authoring-db-{label}-{nanos}"))
        .join("index.db");
    let url = format!("sqlite://{}?mode=rwc", path.display());
    (path, url)
}

fn run_db(db_url: &str, docs: &Path, args: &[&str]) -> Output {
    let mut full_args = vec!["--backend", "db", "--docs-dir", docs.to_str().unwrap()];
    full_args.extend_from_slice(args);
    living_docs()
        .env("DATABASE_URL", db_url)
        .args(full_args)
        .output()
        .expect("failed to run living-docs")
}

fn run_fs(docs: &Path, args: &[&str]) -> Output {
    let mut full_args = vec!["--docs-dir", docs.to_str().unwrap()];
    full_args.extend_from_slice(args);
    living_docs()
        .args(full_args)
        .output()
        .expect("failed to run living-docs")
}

fn run_sync(db_url: &str, docs: &Path) -> Output {
    living_docs()
        .env("DATABASE_URL", db_url)
        .args(["--docs-dir", docs.to_str().unwrap(), "db", "sync"])
        .output()
        .expect("failed to run living-docs db sync")
}

fn run_check(docs: &Path) -> Output {
    living_docs()
        .args(["check", docs.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs check")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn broken_link_count(stdout: &str) -> usize {
    stdout
        .lines()
        .filter(|line| line.contains("broken link ->"))
        .count()
}

fn cleanup(docs: &Path, db_path: &Path) {
    let _ = fs::remove_dir_all(docs);
    let _ = fs::remove_file(db_path);
    let _ = fs::remove_dir(db_path.parent().unwrap());
}

/// A minimal on-disk check-passing skeleton: a bundle-root `index.md`
/// linking to `adr/index.md`, which in turn links to the `NNNN-<slug>.md`
/// records `new --backend db` will author — the two `index.md` files stay
/// fs-only by design (ADR 0007: `index.md`/`log.md` are never synced to
/// `db-store`), so a fixture that also runs `check` seeds them directly on
/// disk regardless of which backend authors the records themselves.
fn seed_index_skeleton(docs: &Path, adr_entries: &[&str]) {
    fs::create_dir_all(docs.join("adr")).unwrap();
    fs::write(docs.join("index.md"), "# Index\n\n- [ADRs](adr/index.md)\n").unwrap();
    let rows: String = adr_entries
        .iter()
        .map(|entry| format!("- [{entry}]({entry}.md)\n"))
        .collect();
    fs::write(
        docs.join("adr").join("index.md"),
        format!("# ADRs\n\n{rows}"),
    )
    .unwrap();
}

#[test]
fn backend_db_new_persists_the_record_and_a_second_new_allocates_past_it() {
    let docs = temp_dir("retrieve");
    let (db_path, db_url) = temp_sqlite_url("retrieve");

    let first = run_db(&db_url, &docs, &["new", "adr", "First"]);
    assert!(first.status.success(), "stderr: {}", stderr_of(&first));
    assert!(!docs.join("adr/0001-first.md").exists());

    let second = run_db(&db_url, &docs, &["new", "adr", "Second"]);
    assert!(second.status.success(), "stderr: {}", stderr_of(&second));
    let printed_path = stdout_of(&second).trim().to_string();
    assert!(
        printed_path.ends_with("adr/0002-second.md"),
        "got: {printed_path}"
    );

    cleanup(&docs, &db_path);
}

/// `new`'s scaffolded ADR body carries example placeholder links
/// (`research/NNNN-<slug>.md`, etc.) that are broken by design until an
/// author fills them in — so a fresh `new` output does not itself pass
/// `check` in either backend. What must match between backends is the
/// *verdict*: the same exit code and the same violations on the same
/// (deterministically templated) content, proving `--backend db check`
/// validates through the identical `check` core as file-mode.
#[test]
fn backend_db_check_reaches_the_same_verdict_as_file_mode_check_on_the_equivalent_corpus() {
    let docs_fs = temp_dir("verdict-fs");
    let docs_db = temp_dir("verdict-db");
    let (db_path, db_url) = temp_sqlite_url("verdict");

    let new_fs = run_fs(&docs_fs, &["new", "adr", "X"]);
    assert!(new_fs.status.success(), "stderr: {}", stderr_of(&new_fs));
    seed_index_skeleton(&docs_fs, &["0001-x"]);

    let new_db = run_db(&db_url, &docs_db, &["new", "adr", "X"]);
    assert!(new_db.status.success(), "stderr: {}", stderr_of(&new_db));
    seed_index_skeleton(&docs_db, &["0001-x"]);

    let check_fs = run_check(&docs_fs);
    let check_db = run_db(&db_url, &docs_db, &["check"]);

    assert_eq!(
        check_fs.status.code(),
        check_db.status.code(),
        "fs stdout: {}\ndb stdout: {}",
        stdout_of(&check_fs),
        stdout_of(&check_db)
    );
    assert_eq!(
        broken_link_count(&stdout_of(&check_fs)),
        broken_link_count(&stdout_of(&check_db)),
        "fs stdout: {}\ndb stdout: {}",
        stdout_of(&check_fs),
        stdout_of(&check_db)
    );

    let _ = fs::remove_dir_all(&docs_fs);
    cleanup(&docs_db, &db_path);
}

#[test]
fn backend_db_check_ignores_a_record_that_only_exists_on_disk() {
    let docs = temp_dir("check-catches-gap");
    let (db_path, db_url) = temp_sqlite_url("check-catches-gap");
    seed_index_skeleton(&docs, &["0001-x"]);

    let new_output = run_db(&db_url, &docs, &["new", "adr", "X"]);
    assert!(
        new_output.status.success(),
        "stderr: {}",
        stderr_of(&new_output)
    );

    fs::write(
        docs.join("adr").join("0002-orphan.md"),
        "---\ntype: ADR\ntitle: Orphan\n---\n# Orphan\n",
    )
    .unwrap();

    let check_output = run_db(&db_url, &docs, &["check"]);
    assert!(
        !stdout_of(&check_output).contains("0002-orphan"),
        "an on-disk-only record the db backend never lists must not surface in its check output: {}",
        stdout_of(&check_output)
    );

    cleanup(&docs, &db_path);
}

const CLEAN_RECORD: &str = "---\ntype: ADR\ntitle: Clean Decision\ndescription: A minimal record with no placeholder links.\nstatus: Accepted\nsupersedes:\nsuperseded_by:\ntags: []\ntimestamp: 2026-07-17T00:00:00Z\n---\n\n# Clean Decision\n\n## Context\n\nA minimal, self-contained decision record with no links, used to exercise\nthe db-mode export round trip.\n\n## Decision\n\nWe will keep this fixture intentionally free of links.\n";

/// Seeds a flat (no type-subdirectory) fs corpus with one link-free record,
/// syncs it into the db, and returns the docs dir plus db location — the
/// db-mode `export` fitness-function tests build on this shared corpus.
fn seeded_clean_db_corpus(label: &str) -> (PathBuf, PathBuf, String) {
    let docs = temp_dir(&format!("{label}-docs"));
    let (db_path, db_url) = temp_sqlite_url(label);
    fs::write(
        docs.join("index.md"),
        "# Index\n\n- [Clean Decision](0001-clean.md)\n",
    )
    .unwrap();
    fs::write(docs.join("0001-clean.md"), CLEAN_RECORD).unwrap();

    let sync_output = run_sync(&db_url, &docs);
    assert!(
        sync_output.status.success(),
        "stderr: {}",
        stderr_of(&sync_output)
    );

    (docs, db_path, db_url)
}

#[test]
fn backend_db_export_writes_a_tree_that_passes_file_mode_check() {
    let (docs, db_path, db_url) = seeded_clean_db_corpus("export-check");
    let out_dir = temp_dir("export-check-out");
    fs::remove_dir_all(&out_dir).unwrap();

    let export_output = run_db(&db_url, &docs, &["export", out_dir.to_str().unwrap()]);
    assert!(
        export_output.status.success(),
        "stderr: {}",
        stderr_of(&export_output)
    );
    assert!(out_dir.join("0001-clean.md").exists());

    fs::create_dir_all(&out_dir).unwrap();
    fs::copy(docs.join("index.md"), out_dir.join("index.md")).unwrap();

    let check_output = run_check(&out_dir);
    assert!(
        check_output.status.success(),
        "stderr: {}\nstdout: {}",
        stderr_of(&check_output),
        stdout_of(&check_output)
    );
    assert!(stdout_of(&check_output).contains("no invariant violations"));

    let _ = fs::remove_dir_all(&out_dir);
    cleanup(&docs, &db_path);
}

#[test]
fn backend_db_export_is_idempotent_producing_byte_identical_output_on_a_second_run() {
    let (docs, db_path, db_url) = seeded_clean_db_corpus("export-idempotent");
    let out_dir = temp_dir("export-idempotent-out");
    fs::remove_dir_all(&out_dir).unwrap();

    let first_export = run_db(&db_url, &docs, &["export", out_dir.to_str().unwrap()]);
    assert!(
        first_export.status.success(),
        "stderr: {}",
        stderr_of(&first_export)
    );
    let first_bytes = fs::read(out_dir.join("0001-clean.md")).expect("first export written");

    let second_export = run_db(&db_url, &docs, &["export", out_dir.to_str().unwrap()]);
    assert!(
        second_export.status.success(),
        "stderr: {}",
        stderr_of(&second_export)
    );
    let second_bytes = fs::read(out_dir.join("0001-clean.md")).expect("second export written");

    assert_eq!(first_bytes, second_bytes, "export is not idempotent");

    let _ = fs::remove_dir_all(&out_dir);
    cleanup(&docs, &db_path);
}

#[test]
fn backend_db_supersede_persists_status_and_relation_through_the_port_and_round_trips_on_export() {
    let docs = temp_dir("supersede-db");
    let (db_path, db_url) = temp_sqlite_url("supersede-db");

    let old = run_db(&db_url, &docs, &["new", "adr", "Old Decision"]);
    assert!(old.status.success(), "stderr: {}", stderr_of(&old));
    let new = run_db(&db_url, &docs, &["new", "adr", "New Decision"]);
    assert!(new.status.success(), "stderr: {}", stderr_of(&new));

    let supersede_output = run_db(&db_url, &docs, &["supersede", "0001", "0002"]);
    assert!(
        supersede_output.status.success(),
        "stderr: {}",
        stderr_of(&supersede_output)
    );

    seed_index_skeleton(&docs, &["0001-old-decision", "0002-new-decision"]);
    let check_output = run_db(&db_url, &docs, &["check"]);
    assert!(
        !stdout_of(&check_output).contains("invariant 4"),
        "supersede must resolve cleanly for the db backend's own check pass: {}",
        stdout_of(&check_output)
    );

    let out_dir = temp_dir("supersede-db-export");
    fs::remove_dir_all(&out_dir).unwrap();
    let export_output = run_db(&db_url, &docs, &["export", out_dir.to_str().unwrap()]);
    assert!(
        export_output.status.success(),
        "stderr: {}",
        stderr_of(&export_output)
    );

    let old_contents = fs::read_to_string(out_dir.join("adr/0001-old-decision.md")).unwrap();
    let new_contents = fs::read_to_string(out_dir.join("adr/0002-new-decision.md")).unwrap();

    assert!(
        old_contents.contains("status: Superseded"),
        "got: {old_contents}"
    );
    assert!(
        old_contents.contains("superseded_by: 0002"),
        "got: {old_contents}"
    );
    assert!(
        new_contents.contains("supersedes: 0001"),
        "got: {new_contents}"
    );

    let _ = fs::remove_dir_all(&out_dir);
    cleanup(&docs, &db_path);
}

#[test]
fn backend_db_supersede_fails_when_a_record_number_does_not_exist() {
    let docs = temp_dir("supersede-db-missing");
    let (db_path, db_url) = temp_sqlite_url("supersede-db-missing");

    let only = run_db(&db_url, &docs, &["new", "adr", "Only Decision"]);
    assert!(only.status.success(), "stderr: {}", stderr_of(&only));

    let output = run_db(&db_url, &docs, &["supersede", "0001", "0099"]);

    assert!(!output.status.success());
    assert!(
        stderr_of(&output).contains("no record found"),
        "got: {}",
        stderr_of(&output)
    );

    cleanup(&docs, &db_path);
}

#[test]
fn backend_db_index_regenerates_the_filesystem_index_from_db_records_matching_file_mode_byte_for_byte(
) {
    let docs_fs = temp_dir("index-fs");
    let docs_db = temp_dir("index-db");
    let (db_path, db_url) = temp_sqlite_url("index-parity");

    assert!(run_fs(&docs_fs, &["new", "adr", "First Decision"])
        .status
        .success());
    assert!(run_fs(&docs_fs, &["new", "adr", "Second Decision"])
        .status
        .success());
    let index_fs = run_fs(&docs_fs, &["index", "adr"]);
    assert!(
        index_fs.status.success(),
        "stderr: {}",
        stderr_of(&index_fs)
    );

    assert!(run_db(&db_url, &docs_db, &["new", "adr", "First Decision"])
        .status
        .success());
    assert!(
        run_db(&db_url, &docs_db, &["new", "adr", "Second Decision"])
            .status
            .success()
    );
    let index_db = run_db(&db_url, &docs_db, &["index", "adr"]);
    assert!(
        index_db.status.success(),
        "stderr: {}",
        stderr_of(&index_db)
    );

    let fs_bytes = fs::read(docs_fs.join("adr/index.md")).expect("fs index.md written");
    let db_bytes = fs::read(docs_db.join("adr/index.md")).expect("db index.md written");
    assert_eq!(
        fs_bytes, db_bytes,
        "db-mode index output diverges from fs-mode"
    );

    let _ = fs::remove_dir_all(&docs_fs);
    cleanup(&docs_db, &db_path);
}

#[test]
fn default_backend_new_and_check_reach_the_same_verdict_as_the_explicit_fs_backend() {
    let docs_default = temp_dir("no-flag-regression");
    let docs_explicit = temp_dir("explicit-fs-regression");

    let new_default = run_fs(&docs_default, &["new", "adr", "Same Title"]);
    assert!(
        new_default.status.success(),
        "stderr: {}",
        stderr_of(&new_default)
    );
    assert!(docs_default.join("adr/0001-same-title.md").exists());
    seed_index_skeleton(&docs_default, &["0001-same-title"]);

    let new_explicit = living_docs()
        .args([
            "--backend",
            "fs",
            "--docs-dir",
            docs_explicit.to_str().unwrap(),
            "new",
            "adr",
            "Same Title",
        ])
        .output()
        .expect("failed to run living-docs");
    assert!(
        new_explicit.status.success(),
        "stderr: {}",
        stderr_of(&new_explicit)
    );
    assert!(docs_explicit.join("adr/0001-same-title.md").exists());
    seed_index_skeleton(&docs_explicit, &["0001-same-title"]);

    let check_default = run_check(&docs_default);
    let check_explicit = run_check(&docs_explicit);

    assert_eq!(check_default.status.code(), check_explicit.status.code());
    assert_eq!(
        broken_link_count(&stdout_of(&check_default)),
        broken_link_count(&stdout_of(&check_explicit))
    );

    let _ = fs::remove_dir_all(&docs_default);
    let _ = fs::remove_dir_all(&docs_explicit);
}
