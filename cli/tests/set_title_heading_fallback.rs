mod common;

use common::{index_bundle, living_docs, stdout_of, temp_bundle, write, Scratch};
use std::fs;
use std::path::{Path, PathBuf};

const REL: &str = "issues/0001-alpha.md";

fn issue(title: &str, body: &str) -> String {
    format!(
        "---\ntype: Issue\ntitle: {title}\ndescription: d\nowner: o\nstatus: Open\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n{body}"
    )
}

fn bundle_with(label: &str, record: &str) -> Scratch {
    let bundle = temp_bundle("set-title-heading-fallback", label);
    write(&bundle, REL, record);
    index_bundle(&bundle);
    bundle
}

fn set_title(bundle: &Path, title: &str) {
    let output = living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap()])
        .args(["write", "set", "issue/0001", "title", title])
        .output()
        .expect("failed to run living-docs set");
    assert!(output.status.success(), "{}", stdout_of(&output));
}

fn fmt_after_title_edit(bundle: &Path, title: &str) {
    let path = bundle.join(REL);
    let edited = fs::read_to_string(&path)
        .unwrap()
        .replace("title: Alpha\n", &format!("title: {title}\n"));
    fs::write(&path, edited).unwrap();
    let output = living_docs()
        .args(["manage", "fmt", bundle.to_str().unwrap(), "--plain"])
        .output()
        .expect("failed to run living-docs fmt");
    assert!(output.status.success(), "{}", stdout_of(&output));
}

fn record_file(bundle: &Path) -> PathBuf {
    fs::read_dir(bundle.join("issues"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "md") && !p.ends_with("index.md"))
        .expect("issue record")
}

fn body_of(bundle: &Path) -> String {
    let contents = fs::read_to_string(record_file(bundle)).unwrap();
    contents.split_once("---\n\n").unwrap().1.to_owned()
}

const NO_TITLE: &str = "### Scope\n\nx\n\n# References\n\ny\n";
const LEADING: &str = "# Beta\n\n### Scope\n\n# References\n\ny\n";

const AFTER_SECTION: &str = "## Scope\n\nx\n\n# Beta\n\n# References\n\ny\n";

#[test]
fn set_title_renames_a_title_heading_placed_after_a_section_heading() {
    let bundle = bundle_with("set-after", &issue("Alpha", AFTER_SECTION));
    set_title(&bundle, "Gamma");
    assert_eq!(body_of(&bundle), AFTER_SECTION.replace("# Beta", "# Gamma"));
}

#[test]
fn fmt_renames_a_title_heading_placed_after_a_section_heading() {
    let bundle = bundle_with("fmt-after", &issue("Alpha", AFTER_SECTION));
    fmt_after_title_edit(&bundle, "Gamma");
    assert_eq!(body_of(&bundle), AFTER_SECTION.replace("# Beta", "# Gamma"));
}

#[test]
fn set_title_leaves_a_later_section_heading_alone() {
    let bundle = bundle_with("set-none", &issue("Alpha", NO_TITLE));
    set_title(&bundle, "Gamma");
    assert_eq!(body_of(&bundle), NO_TITLE);
}

#[test]
fn fmt_leaves_a_later_section_heading_alone() {
    let bundle = bundle_with("fmt-none", &issue("Alpha", NO_TITLE));
    fmt_after_title_edit(&bundle, "Gamma");
    assert_eq!(body_of(&bundle), NO_TITLE);
}

const LOWER_REFERENCES: &str = "### Scope\n\nx\n\n# references\n\ny\n";
const UPPER_REFERENCES: &str = "### Scope\n\nx\n\n# REFERENCES\n\ny\n";

#[test]
fn set_title_leaves_a_lower_case_section_heading_alone() {
    let bundle = bundle_with("set-lower", &issue("Alpha", LOWER_REFERENCES));
    set_title(&bundle, "Gamma");
    assert_eq!(body_of(&bundle), LOWER_REFERENCES);
}

#[test]
fn fmt_leaves_a_lower_case_section_heading_alone() {
    let bundle = bundle_with("fmt-lower", &issue("Alpha", LOWER_REFERENCES));
    fmt_after_title_edit(&bundle, "Gamma");
    assert_eq!(body_of(&bundle), LOWER_REFERENCES);
}

#[test]
fn set_title_leaves_an_upper_case_section_heading_alone() {
    let bundle = bundle_with("set-upper", &issue("Alpha", UPPER_REFERENCES));
    set_title(&bundle, "Gamma");
    assert_eq!(body_of(&bundle), UPPER_REFERENCES);
}

#[test]
fn set_title_renames_a_leading_level_one_heading() {
    let bundle = bundle_with("set-lead", &issue("Alpha", LEADING));
    set_title(&bundle, "Gamma");
    assert_eq!(body_of(&bundle), LEADING.replace("# Beta", "# Gamma"));
}

#[test]
fn fmt_renames_a_leading_level_one_heading() {
    let bundle = bundle_with("fmt-lead", &issue("Alpha", LEADING));
    fmt_after_title_edit(&bundle, "Gamma");
    assert_eq!(body_of(&bundle), LEADING.replace("# Beta", "# Gamma"));
}
