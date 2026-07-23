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
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let printed_path = stdout.lines().next().expect("stdout has a first line");
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

/// ADR 0019, AC ac-s4-1: `new`'s title argument is CLI-filled into the
/// scaffold's frontmatter `title:` line, for every doc type that carries a
/// title placeholder.
#[test]
fn new_fills_the_frontmatter_title_from_the_argument_for_every_doc_type() {
    for (doc_type, dir_name) in [
        ("adr", "adr"),
        ("bdr", "bdr"),
        ("prd", "prd"),
        ("issue", "issues"),
    ] {
        let docs = temp_dir(&format!("title-{doc_type}"));

        let output = run_new(&docs, doc_type, "My Decision");
        assert!(output.status.success());

        let path = docs.join(format!("{dir_name}/0001-my-decision.md"));
        let contents = fs::read_to_string(&path).unwrap();
        let title_line = contents
            .lines()
            .find(|line| line.starts_with("title:"))
            .unwrap_or_else(|| panic!("{doc_type}: no title: line, got:\n{contents}"));
        assert_eq!(
            title_line, "title: My Decision",
            "{doc_type}: got:\n{contents}"
        );

        let _ = fs::remove_dir_all(&docs);
    }
}

/// A title requiring YAML quoting is filled using the same canonical
/// quoting `living-docs fmt`/`check` expect (`record::format_scalar`), not a
/// local rule — so the scaffold stays a canonical round-trip fixed point.
#[test]
fn new_quotes_a_title_containing_a_colon_exactly_as_the_canonical_serializer_would() {
    let docs = temp_dir("title-quoted");

    let output = run_new(&docs, "adr", "Caching: A Deep Dive");

    assert!(output.status.success());
    let contents = fs::read_to_string(docs.join("adr/0001-caching-a-deep-dive.md")).unwrap();
    assert!(
        contents.contains("title: \"Caching: A Deep Dive\"\n"),
        "got:\n{contents}"
    );

    let _ = fs::remove_dir_all(&docs);
}

/// ADR 0019, AC ac-s4-2: `new`'s stdout carries the created path followed by
/// the body-only instruction, naming status/supersede/index as the CLI
/// verbs that own frontmatter and indexes.
#[test]
fn new_stdout_ends_with_the_body_only_instruction_after_the_created_path() {
    let docs = temp_dir("instruction");

    let output = run_new(&docs, "adr", "Instructed Decision");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let mut lines = stdout.lines();
    let first_line = lines.next().expect("stdout has a first line");
    assert!(first_line.ends_with("adr/0001-instructed-decision.md"));
    let instruction_line = lines.next().expect("stdout has a second line");
    assert!(instruction_line.contains("Write ONLY the body below the closing"));
    assert!(instruction_line.contains("living-docs status"));
    assert!(instruction_line.contains("supersede"));
    assert!(instruction_line.contains("index"));

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
