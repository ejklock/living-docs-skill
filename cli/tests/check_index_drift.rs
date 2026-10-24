mod common;

use common::{living_docs, run_check, stdout_of, temp_bundle};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

const BODY: &str = "### Scope\n\nOne thing.\n\n### Acceptance\n\n- It works.\n";
const REPAIR: &str = "living-docs index issue";

fn new_issue(docs: &Path, title: &str) {
    let body = docs.parent().unwrap().join("body.txt");
    fs::write(&body, BODY).unwrap();
    let output = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "new", "issue", title])
        .args(["--plain", "--body"])
        .arg(&body)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn indexed_bundle(label: &str) -> PathBuf {
    let docs = temp_bundle("check-index-drift", label);
    fs::write(docs.join("index.md"), "# Docs\n\n* [Issues](issues/)\n").unwrap();
    new_issue(&docs, "First");
    new_issue(&docs, "Second");
    docs
}

fn index_of(docs: &Path) -> PathBuf {
    docs.join("issues").join("index.md")
}

fn edit_index(docs: &Path, edit: &dyn Fn(&str) -> String) {
    let path = index_of(docs);
    fs::write(&path, edit(&fs::read_to_string(&path).unwrap())).unwrap();
}

fn assert_one_drift_violation(output: &Output) {
    let report = stdout_of(output);
    assert_eq!(output.status.code(), Some(1), "{report}");
    let lines: Vec<&str> = report.lines().filter(|l| l.contains(REPAIR)).collect();
    assert_eq!(lines.len(), 1, "{report}");
    assert!(lines[0].contains("issues/index.md"), "{report}");
    assert!(report.contains("1 violation(s)"), "{report}");
}

fn row_with(index: &str, needle: &str) -> String {
    index
        .lines()
        .find(|l| l.contains(needle))
        .unwrap()
        .to_owned()
}

#[test]
fn a_freshly_indexed_bundle_reports_no_drift() {
    let docs = indexed_bundle("fresh");
    let output = run_check(&docs);
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}

#[test]
fn a_hand_edited_status_in_an_index_fails_check_naming_the_index_and_the_repair() {
    let docs = indexed_bundle("status");
    edit_index(&docs, &|text| text.replace("- open", "- done"));
    assert_one_drift_violation(&run_check(&docs));
}

#[test]
fn a_duplicated_row_in_an_index_fails_check_naming_the_index_and_the_repair() {
    let docs = indexed_bundle("duplicate");
    edit_index(&docs, &|text| {
        let row = row_with(text, "0001");
        text.replace(&row, &format!("{row}\n{row}"))
    });
    assert_one_drift_violation(&run_check(&docs));
}

#[test]
fn rows_out_of_order_in_an_index_fail_check_naming_the_index_and_the_repair() {
    let docs = indexed_bundle("order");
    edit_index(&docs, &|text| {
        let (first, second) = (row_with(text, "0001"), row_with(text, "0002"));
        text.replace(&first, "@@")
            .replace(&second, &first)
            .replace("@@", &second)
    });
    assert_one_drift_violation(&run_check(&docs));
}

#[test]
fn indexing_repairs_a_drifted_index_so_check_passes_again() {
    let docs = indexed_bundle("repair");
    edit_index(&docs, &|text| text.replace("- open", "- done"));
    let indexed = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "index", "--plain"])
        .output()
        .unwrap();
    assert!(indexed.status.success());
    assert_eq!(run_check(&docs).status.code(), Some(0));
}

fn to_crlf(dir: &Path, only_indexes: bool) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let is_index = path.file_name().is_some_and(|name| name == "index.md");
        if path.is_dir() {
            to_crlf(&path, only_indexes);
        } else if path.extension().is_some_and(|ext| ext == "md") && (is_index || !only_indexes) {
            let text = fs::read_to_string(&path).unwrap();
            fs::write(&path, text.replace("\r\n", "\n").replace('\n', "\r\n")).unwrap();
        }
    }
}

#[test]
fn a_crlf_bundle_written_by_the_cli_reports_no_drift() {
    for (label, only_indexes) in [("crlf-indexes", true), ("crlf-all", false)] {
        let docs = indexed_bundle(label);
        to_crlf(&docs, only_indexes);
        new_issue(&docs, "Third");
        let index = fs::read_to_string(index_of(&docs)).unwrap();
        assert!(index.contains("\r\n") && !index.replace("\r\n", "").contains('\n'));
        let output = run_check(&docs);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{label}: {}",
            stdout_of(&output)
        );
    }
}

#[test]
fn this_repositorys_own_docs_report_no_drift() {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("docs");
    let output = run_check(&docs);
    let report = stdout_of(&output);
    assert!(!report.contains("living-docs index"), "{report}");
    assert_eq!(output.status.code(), Some(0), "{report}");
}

#[test]
fn a_preamble_kept_by_index_is_compared_as_written() {
    let docs = indexed_bundle("preamble");
    edit_index(&docs, &|text| {
        text.replacen(
            "# Issues\n",
            "# Issues\n\nHand-written intro kept by index.\n",
            1,
        )
    });
    let output = run_check(&docs);
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
    edit_index(&docs, &|text| text.replace("- open", "- done"));
    assert_one_drift_violation(&run_check(&docs));
}

fn run_scoped(docs: &Path, changed: &Path) -> Output {
    living_docs()
        .args(["check", "--plain", "--changed-files"])
        .arg(changed)
        .arg("--")
        .arg(docs)
        .output()
        .unwrap()
}

#[test]
fn changed_files_reports_drift_only_when_the_drifted_index_is_among_them() {
    let docs = indexed_bundle("scoped");
    edit_index(&docs, &|text| text.replace("- open", "- done"));
    let with_index = run_scoped(&docs, &index_of(&docs));
    assert_one_drift_violation(&with_index);
    let record = docs.join("issues").join("0001-first.md");
    let without_index = run_scoped(&docs, &record);
    assert_eq!(
        without_index.status.code(),
        Some(0),
        "{}",
        stdout_of(&without_index)
    );
}
