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
    let dir = std::env::temp_dir().join(format!("living-docs-new-test-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_new(docs: &Path, doc_type: &str, title: &str) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "new", doc_type, title])
        .output()
        .expect("failed to run living-docs")
}

#[test]
fn new_scaffolds_0001_on_an_empty_tree() {
    let docs = temp_dir("empty");

    let output = run_new(&docs, "adr", "My Title");

    assert!(output.status.success());
    let printed_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(
        printed_path.ends_with("adr/0001-my-title.md"),
        "got: {printed_path}"
    );
    assert!(docs.join("adr/0001-my-title.md").exists());

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn new_allocates_the_next_number_past_an_existing_record() {
    let docs = temp_dir("existing");
    let adr_dir = docs.join("adr");
    fs::create_dir_all(&adr_dir).unwrap();
    fs::write(adr_dir.join("0001-old.md"), "---\ntype: ADR\n---\n# Old\n").unwrap();

    let output = run_new(&docs, "adr", "Second Title");

    assert!(output.status.success());
    assert!(docs.join("adr/0002-second-title.md").exists());

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn new_slugifies_the_title_to_lowercase_kebab_case() {
    let docs = temp_dir("slugify");

    let output = run_new(&docs, "adr", "Some Complex, Title!!");

    assert!(output.status.success());
    assert!(docs.join("adr/0001-some-complex-title.md").exists());

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn new_maps_issue_to_the_plural_issues_directory() {
    let docs = temp_dir("issue-dir");

    let output = run_new(&docs, "issue", "Broken Link Checker");

    assert!(output.status.success());
    let path = docs.join("issues/0001-broken-link-checker.md");
    assert!(path.exists());
    let contents = fs::read_to_string(&path).unwrap();
    assert!(contents.contains("type: Issue"));

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn new_fills_type_status_proposed_and_an_iso8601_timestamp() {
    let docs = temp_dir("frontmatter");

    let output = run_new(&docs, "bdr", "Search Autocomplete");

    assert!(output.status.success());
    let contents = fs::read_to_string(docs.join("bdr/0001-search-autocomplete.md")).unwrap();

    assert!(contents.contains("type: BDR"));
    assert!(contents.contains("status: Proposed"));

    let timestamp_line = contents
        .lines()
        .find(|l| l.starts_with("timestamp:"))
        .unwrap();
    let value = timestamp_line.trim_start_matches("timestamp:").trim();
    assert_eq!(value.len(), 20, "unexpected timestamp: {value}");
    assert!(value.ends_with('Z'));
    assert_eq!(value.as_bytes()[10], b'T');

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn new_preserves_body_placeholders_and_guidance_comments_verbatim() {
    let docs = temp_dir("placeholders");

    let output = run_new(&docs, "adr", "Preserve Body");

    assert!(output.status.success());
    let contents = fs::read_to_string(docs.join("adr/0001-preserve-body.md")).unwrap();

    assert!(contents.contains(
        "<!-- Status lives in frontmatter (`status`), not a body line. When superseding a"
    ));
    assert!(contents.contains("We will <the choice, in active voice — specific and testable>."));
    assert!(contents.contains("status: Proposed"));
    assert!(contents.contains("# Proposed | Accepted | Superseded | Deprecated"));

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn new_rejects_an_unsupported_doc_type() {
    let docs = temp_dir("unsupported");

    let output = run_new(&docs, "constitution", "Root Rules");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unsupported doc type"));
    assert!(!docs.join("constitution").exists());

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn new_honors_the_docs_dir_flag_across_repeated_calls() {
    let docs = temp_dir("repeated");

    let first = run_new(&docs, "prd", "Repeated Title");
    let second = run_new(&docs, "prd", "Repeated Title");

    assert!(first.status.success());
    assert!(second.status.success());
    assert!(docs.join("prd/0001-repeated-title.md").exists());
    assert!(docs.join("prd/0002-repeated-title.md").exists());

    let _ = fs::remove_dir_all(&docs);
}
