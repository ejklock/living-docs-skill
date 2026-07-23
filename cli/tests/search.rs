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
    let dir = std::env::temp_dir().join(format!("living-docs-search-test-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_record(
    docs: &Path,
    dir: &str,
    filename: &str,
    title: &str,
    description: &str,
    body: &str,
) {
    let type_dir = docs.join(dir);
    fs::create_dir_all(&type_dir).unwrap();
    let contents = format!(
        "---\ntype: ADR\ntitle: {title}\ndescription: {description}\nstatus: Accepted\n---\n\n# {title}\n\n{body}\n"
    );
    fs::write(type_dir.join(filename), contents).unwrap();
}

fn run(cwd: &Path, docs: &Path, args: &[&str]) -> Output {
    let mut full_args = vec!["--docs-dir", docs.to_str().unwrap()];
    full_args.extend_from_slice(args);
    living_docs()
        .current_dir(cwd)
        .args(full_args)
        .output()
        .expect("failed to run living-docs")
}

#[test]
fn db_sync_then_search_finds_the_seeded_record_and_ranks_it_first_while_a_no_match_query_prints_nothing(
) {
    let docs = temp_dir("docs");
    let cwd = temp_dir("cwd");
    write_record(
        &docs,
        "adr",
        "0001-quokka-caching.md",
        "Quokka Caching Strategy",
        "Adopt aggressive quokka caching for search results.",
        "We adopt an aggressive quokka caching strategy for search results.",
    );
    write_record(
        &docs,
        "adr",
        "0002-unrelated.md",
        "Unrelated Decision",
        "Something else entirely.",
        "This document discusses logging conventions.",
    );

    let sync_output = run(&cwd, &docs, &["--engine", "sqlite", "db", "sync"]);
    assert!(
        sync_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&sync_output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&sync_output.stdout).contains("Indexed 2 records."),
        "stdout: {}",
        String::from_utf8_lossy(&sync_output.stdout)
    );
    assert!(cwd.join(".living-docs").join("index.db").exists());

    let hit_output = run(&cwd, &docs, &["--engine", "sqlite", "search", "quokka"]);
    assert!(
        hit_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&hit_output.stderr)
    );
    let hit_stdout = String::from_utf8_lossy(&hit_output.stdout);
    assert!(
        hit_stdout.contains("adr/0001-quokka-caching.md"),
        "got: {hit_stdout}"
    );
    assert!(
        hit_stdout.contains("Quokka Caching Strategy"),
        "got: {hit_stdout}"
    );
    assert!(
        !hit_stdout.contains("Unrelated Decision"),
        "got: {hit_stdout}"
    );

    let miss_output = run(
        &cwd,
        &docs,
        &["--engine", "sqlite", "search", "zzzznomatch"],
    );
    assert!(
        miss_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&miss_output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&miss_output.stdout)
            .trim()
            .is_empty(),
        "expected no output for a no-match query, got: {}",
        String::from_utf8_lossy(&miss_output.stdout)
    );

    let _ = fs::remove_dir_all(&docs);
    let _ = fs::remove_dir_all(&cwd);
}

#[test]
fn search_before_a_db_sync_fails_with_a_helpful_hint() {
    let docs = temp_dir("docs-no-sync");
    let cwd = temp_dir("cwd-no-sync");
    write_record(
        &docs,
        "adr",
        "0001-only.md",
        "Only Decision",
        "The only seeded record.",
        "Body text.",
    );

    let output = run(&cwd, &docs, &["--engine", "sqlite", "search", "decision"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("living-docs db sync"), "got: {stderr}");

    let _ = fs::remove_dir_all(&docs);
    let _ = fs::remove_dir_all(&cwd);
}

#[test]
fn search_defaults_to_paradedb_and_requires_database_url_when_none_is_set() {
    let docs = temp_dir("docs-default-engine");
    let cwd = temp_dir("cwd-default-engine");

    let output = living_docs()
        .current_dir(&cwd)
        .env_remove("DATABASE_URL")
        .args(["--docs-dir", docs.to_str().unwrap(), "search", "anything"])
        .output()
        .expect("failed to run living-docs");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("DATABASE_URL"), "got: {stderr}");

    let _ = fs::remove_dir_all(&docs);
    let _ = fs::remove_dir_all(&cwd);
}

#[test]
fn db_sync_defaults_to_paradedb_and_requires_database_url_when_none_is_set() {
    let docs = temp_dir("docs-default-sync-engine");
    let cwd = temp_dir("cwd-default-sync-engine");
    write_record(
        &docs,
        "adr",
        "0001-only.md",
        "Only Decision",
        "The only seeded record.",
        "Body text.",
    );

    let output = living_docs()
        .current_dir(&cwd)
        .env_remove("DATABASE_URL")
        .args(["--docs-dir", docs.to_str().unwrap(), "db", "sync"])
        .output()
        .expect("failed to run living-docs");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("DATABASE_URL"), "got: {stderr}");

    let _ = fs::remove_dir_all(&docs);
    let _ = fs::remove_dir_all(&cwd);
}
