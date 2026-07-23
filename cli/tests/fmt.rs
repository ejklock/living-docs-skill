use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn run_fmt(bundle: &Path) -> Output {
    living_docs()
        .args(["fmt", bundle.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs fmt")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn temp_bundle(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir()
        .join(format!("living-docs-fmt-test-{label}-{nanos}"))
        .join("docs");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(bundle: &Path, rel: &str, contents: &str) {
    let path = bundle.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn read(bundle: &Path, rel: &str) -> String {
    fs::read_to_string(bundle.join(rel)).unwrap()
}

const NON_CANONICAL_RECORD: &str = "---\ntitle: Quokka Caching\ntype: ADR\ndescription: Adopt quokka caching.\ntags: [performance, caching]\nstatus: Accepted\n---\n# Quokka Caching\n\nAdopt an aggressive quokka caching strategy.\n";

const CANONICAL_RECORD: &str = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\nstatus: Accepted\ntags: [caching, performance]\n---\n\n# Quokka Caching\n\nAdopt an aggressive quokka caching strategy.\n";

#[test]
fn fmt_rewrites_a_non_canonical_record_preserving_body_and_author_owned_values() {
    let bundle = temp_bundle("non-canonical");
    write(&bundle, "index.md", "# Index\n\n- [Doc](adr/0001-doc.md)\n");
    write(&bundle, "adr/0001-doc.md", NON_CANONICAL_RECORD);

    let output = run_fmt(&bundle);
    let stdout = stdout_of(&output);

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert!(
        stdout.contains("0001-doc.md"),
        "expected the rewritten path in stdout, got:\n{stdout}"
    );
    assert!(
        stdout.contains("1 record(s) rewritten."),
        "expected the summary count, got:\n{stdout}"
    );

    let rewritten = read(&bundle, "adr/0001-doc.md");
    assert_eq!(rewritten, CANONICAL_RECORD);
    assert!(rewritten.contains("title: Quokka Caching"));
    assert!(rewritten.contains("description: Adopt quokka caching."));
    assert!(rewritten.contains("tags: [caching, performance]"));
    assert!(rewritten.ends_with("Adopt an aggressive quokka caching strategy.\n"));

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn fmt_is_idempotent_a_second_run_reports_zero_changes_and_bytes_are_identical() {
    let bundle = temp_bundle("idempotent");
    write(&bundle, "index.md", "# Index\n\n- [Doc](adr/0001-doc.md)\n");
    write(&bundle, "adr/0001-doc.md", NON_CANONICAL_RECORD);

    let first = run_fmt(&bundle);
    assert!(first.status.success(), "stderr: {:?}", first.stderr);
    let after_first = read(&bundle, "adr/0001-doc.md");

    let second = run_fmt(&bundle);
    let second_stdout = stdout_of(&second);

    assert!(second.status.success(), "stderr: {:?}", second.stderr);
    assert!(
        second_stdout.contains("0 record(s) rewritten."),
        "expected zero changes on the second run, got:\n{second_stdout}"
    );
    assert!(
        !second_stdout.contains("0001-doc.md"),
        "expected no rewritten path on the second run, got:\n{second_stdout}"
    );
    assert_eq!(read(&bundle, "adr/0001-doc.md"), after_first);

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn fmt_never_touches_reserved_index_or_log_files() {
    let bundle = temp_bundle("reserved");
    let index_contents = "# Index\n\n- [Doc](adr/0001-doc.md)\n";
    let log_contents = "# Log\n\nEntries.\n";
    write(&bundle, "index.md", index_contents);
    write(&bundle, "adr/log.md", log_contents);
    write(&bundle, "adr/0001-doc.md", NON_CANONICAL_RECORD);

    let output = run_fmt(&bundle);

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert_eq!(read(&bundle, "index.md"), index_contents);
    assert_eq!(read(&bundle, "adr/log.md"), log_contents);

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn fmt_leaves_a_record_with_no_frontmatter_untouched() {
    let bundle = temp_bundle("no-frontmatter");
    let plain = "# Just a heading\n\nNo frontmatter here at all.\n";
    write(&bundle, "index.md", "# Index\n\n- [Plain](plain.md)\n");
    write(&bundle, "plain.md", plain);

    let output = run_fmt(&bundle);
    let stdout = stdout_of(&output);

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert!(stdout.contains("0 record(s) rewritten."));
    assert_eq!(read(&bundle, "plain.md"), plain);

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn fmt_reports_a_missing_bundle_root_with_exit_code_two() {
    let bundle = temp_bundle("missing").join("does-not-exist");

    let output = run_fmt(&bundle);

    assert_eq!(output.status.code(), Some(2));

    let _ = fs::remove_dir_all(bundle.parent().unwrap());
}
