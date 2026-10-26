mod common;

use common::{fixture, living_docs, temp_bundle};
use std::fs;
use std::path::{Path, PathBuf};

fn copy_tree(from: &Path, to: &Path, crlf: bool) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target, crlf);
        } else if crlf && entry.path().extension().is_some_and(|ext| ext == "md") {
            let text = fs::read_to_string(entry.path()).unwrap();
            fs::write(&target, text.replace('\n', "\r\n")).unwrap();
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Runs `check` on a fixture and on its CRLF twin, returning both outputs
/// with the twin's bundle path rewritten to the original's.
fn check_twins(name: &str, label: &str) -> ((i32, String), (i32, String)) {
    let lf = temp_bundle("check-crlf", &format!("{label}-lf"));
    let crlf = temp_bundle("check-crlf", &format!("{label}-crlf"));
    copy_tree(&fixture(name), &lf, false);
    copy_tree(&fixture(name), &crlf, true);
    let run = |bundle: &Path| {
        let out = living_docs()
            .args(["check", bundle.to_str().unwrap(), "--plain", "--advisories"])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout).replace(bundle.to_str().unwrap(), "BUNDLE");
        (out.status.code().unwrap(), text)
    };
    (run(&lf), run(&crlf))
}

#[test]
fn check_reports_a_clean_crlf_bundle_as_clean_like_its_lf_twin() {
    let (lf, crlf) = check_twins("01-fence-link-clean", "clean");
    assert_eq!(lf.0, 0, "the fixture is clean: {}", lf.1);
    assert_eq!(crlf, lf);
}

#[test]
fn check_reports_a_dirty_crlf_bundle_with_the_findings_of_its_lf_twin() {
    for (name, label) in [
        ("07-supersede-broken", "supersede"),
        ("09-okf-canonical", "canonical"),
        ("02-fence-link-dirty", "links"),
    ] {
        let (lf, crlf) = check_twins(name, label);
        assert!(
            lf.0 != 0 || lf.1.contains("advisor"),
            "{name} has findings: {}",
            lf.1
        );
        assert_eq!(crlf, lf, "{name}");
    }
}

/// A clean bundle whose `foo.md` record is rewritten by `edit`.
fn clean_bundle_with(label: &str, edit: &dyn Fn(&str) -> String) -> PathBuf {
    let bundle = temp_bundle("check-crlf", label);
    copy_tree(&fixture("01-fence-link-clean"), &bundle, false);
    let record = bundle.join("foo.md");
    fs::write(&record, edit(&fs::read_to_string(&record).unwrap())).unwrap();
    bundle
}

fn check_with(bundle: &Path, flags: &[&str]) -> (i32, String) {
    let out = living_docs()
        .arg("check")
        .arg(bundle)
        .args(flags)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn check_flags_a_mixed_record_once_without_a_false_frontmatter_finding() {
    let first_break_only = |text: &str| text.replacen('\n', "\r\n", 1);
    let last_break_only = |text: &str| {
        let at = text.trim_end().len();
        format!("{}\r\n{}", &text[..at], &text[at + 1..])
    };
    for (label, edit) in [
        ("first", &first_break_only as &dyn Fn(&str) -> String),
        ("last", &last_break_only),
    ] {
        let bundle = clean_bundle_with(&format!("mixed-{label}"), edit);
        let (code, text) = check_with(&bundle, &["--plain"]);
        assert_eq!(code, 1, "{label}: {text}");
        assert_eq!(text.matches("mixed line endings").count(), 1, "{text}");
        assert!(
            text.contains("foo.md") && text.contains("1 violation(s)"),
            "{text}"
        );
        assert!(!text.contains("non-empty"), "{text}");
        let (_, json) = check_with(&bundle, &["--json"]);
        assert!(
            json.contains("mixed line endings") && json.contains("foo.md"),
            "{json}"
        );
    }
}

#[test]
fn check_does_not_flag_files_that_are_not_records_or_that_use_one_ending() {
    let bundle = clean_bundle_with("not-records", &|text| text.replace('\n', "\r\n"));
    let index = "---\r\nokf_version: 0.1\n---\r\n# Index\r\n\n- [Foo](./foo.md)\n";
    fs::write(bundle.join("index.md"), index).unwrap();
    fs::write(bundle.join("log.md"), "# Log\r\n\nentry\n").unwrap();
    let (code, text) = check_with(&bundle, &["--plain"]);
    assert!(!text.contains("mixed line endings"), "{text}");
    assert_eq!(code, 0, "{text}");
}

#[test]
fn check_does_not_flag_mixed_prose_without_frontmatter_as_a_mixed_record() {
    let bundle = clean_bundle_with("mixed-prose", &|text| text.to_string());
    let index = "# Index\n\n- [Foo](./foo.md)\n- [Notes](./notes.md)\n";
    fs::write(bundle.join("index.md"), index).unwrap();
    fs::write(
        bundle.join("notes.md"),
        "# Notes\r\n\nplain prose\nwithout frontmatter\r\n",
    )
    .unwrap();
    let (code, text) = check_with(&bundle, &["--plain"]);
    assert_eq!(code, 1, "{text}");
    assert!(!text.contains("mixed line endings"), "{text}");
    assert!(
        text.contains("1 violation(s)") && text.contains("notes.md"),
        "{text}"
    );
}

#[test]
fn check_changed_files_reports_a_mixed_record_only_when_it_changed() {
    let bundle = clean_bundle_with("mixed-scoped", &|text| text.replacen('\n', "\r\n", 1));
    let scoped = |changed: &str| {
        let out = living_docs()
            .args(["check", "--plain", "--changed-files"])
            .arg(bundle.join(changed))
            .arg("--")
            .arg(&bundle)
            .output()
            .unwrap();
        (
            out.status.code().unwrap(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let (code, text) = scoped("foo.md");
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("mixed line endings"), "{text}");
    let (code, text) = scoped("index.md");
    assert_eq!(code, 0, "{text}");
}
