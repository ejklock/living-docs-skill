use super::{Advisory, Report, Violation};
use std::path::PathBuf;

fn report() -> Report {
    Report {
        bundle: "docs".to_string(),
        docs: 2,
        violations: vec![
            Violation {
                file: "docs/adr/0001-clean.md".to_string(),
                message: "broken link -> docs/nowhere.md".to_string(),
                anchors: Vec::new(),
            },
            Violation {
                file: "docs/adr/0002-dirty.md".to_string(),
                message: "unfilled {{PLACEHOLDER}}".to_string(),
                anchors: Vec::new(),
            },
        ],
        advisories: vec![Advisory {
            file: "docs/adr/0002-dirty.md".to_string(),
            kind: "size".to_string(),
            message: "SIZE body too long".to_string(),
        }],
        ok: false,
    }
}

#[test]
fn scoping_to_a_record_keeps_only_that_record_s_findings() {
    let scoped = report().scoped_to(&[PathBuf::from("docs/adr/0002-dirty.md")]);

    assert_eq!(scoped.violations.len(), 1);
    assert_eq!(scoped.violations[0].file, "docs/adr/0002-dirty.md");
    assert_eq!(scoped.advisories.len(), 1);
}

#[test]
fn scoping_to_a_record_with_no_findings_turns_the_verdict_green() {
    let scoped = report().scoped_to(&[PathBuf::from("docs/adr/0003-untouched.md")]);

    assert!(scoped.ok, "violations: {:?}", scoped.violations);
    assert!(scoped.violations.is_empty());
}

#[test]
fn an_absolute_scope_path_matches_the_report_s_relative_one() {
    let scoped = report().scoped_to(&[PathBuf::from("/repo/docs/adr/0002-dirty.md")]);

    assert_eq!(scoped.violations.len(), 1, "got: {:?}", scoped.violations);
}

#[test]
fn a_relative_scope_path_matches_an_absolute_report_path() {
    let mut absolute = report();
    absolute.violations[1].file = "/repo/docs/adr/0002-dirty.md".to_string();

    let scoped = absolute.scoped_to(&[PathBuf::from("docs/adr/0002-dirty.md")]);

    assert_eq!(scoped.violations.len(), 1, "got: {:?}", scoped.violations);
}

#[test]
fn a_sibling_whose_name_is_a_substring_is_not_matched() {
    let mut similar = report();
    similar.violations[1].file = "docs/adr/0002-dirty-too.md".to_string();

    let scoped = similar.scoped_to(&[PathBuf::from("docs/adr/0002-dirty.md")]);

    assert!(scoped.ok, "violations: {:?}", scoped.violations);
}
