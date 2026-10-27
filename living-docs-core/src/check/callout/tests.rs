use super::*;
use crate::test_support::MapStore;
use std::collections::BTreeMap;
use std::process::ExitCode;

fn exit_code_is_success(code: ExitCode) -> bool {
    format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
}

#[test]
fn check_callouts_reports_a_retired_record_without_its_callout() {
    let (store, all_md) = MapStore::seeded(&[
        (
            "/bundle/adr/0001-old.md",
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0002\n---\n# Old\n",
        ),
        (
            "/bundle/adr/0002-new.md",
            "---\ntype: ADR\nstatus: Accepted\n---\n# New\n",
        ),
    ]);
    let mut reporter = Reporter::new();

    check_callouts(&store, &all_md, &mut reporter);

    let code = reporter.finish();
    assert!(!exit_code_is_success(code));
}

#[test]
fn check_callouts_passes_a_retired_record_whose_callout_matches_expected() {
    let old_path = PathBuf::from("/bundle/adr/0001-old.md");
    let new_path = PathBuf::from("/bundle/adr/0002-new.md");
    let mut files = BTreeMap::new();
    files.insert(old_path.clone(), String::new());
    files.insert(new_path.clone(), "---\ntype: ADR\n---\n# New\n".to_string());
    let probe = MapStore {
        files: files.clone(),
    };
    let successor = callout::successor_filename(&probe, &old_path, "0002");
    let line = callout::expected(Some("Superseded"), Some(successor.as_str())).unwrap();

    files.insert(
        old_path.clone(),
        format!(
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0002\n---\n\n{line}\n\n# Old\n"
        ),
    );
    let store = MapStore { files };
    let all_md = vec![old_path, new_path];
    let mut reporter = Reporter::new();

    check_callouts(&store, &all_md, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}

#[test]
fn check_callouts_reports_an_active_record_opening_with_a_retired_callout() {
    let line = callout::expected(Some("Deprecated"), None).unwrap();
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/adr/0003-active.md",
        &format!("---\ntype: ADR\nstatus: Accepted\n---\n\n{line}\n\n# Active\n"),
    )]);
    let mut reporter = Reporter::new();

    check_callouts(&store, &all_md, &mut reporter);

    assert!(!exit_code_is_success(reporter.finish()));
}

#[test]
fn check_callouts_passes_an_active_record_without_a_callout() {
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/adr/0004-active.md",
        "---\ntype: ADR\nstatus: Accepted\n---\n# Active\n",
    )]);
    let mut reporter = Reporter::new();

    check_callouts(&store, &all_md, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}

fn run_callouts(seed: &[(&str, &str)]) -> Vec<(String, String)> {
    let (store, all_md) = MapStore::seeded(seed);
    let mut reporter = Reporter::new();
    check_callouts(&store, &all_md, &mut reporter);
    reporter
        .violations
        .into_iter()
        .map(|v| (v.file, v.message))
        .collect()
}

fn retired(superseded_by: &str, successor: &str) -> String {
    let line = callout::expected(Some("Superseded"), Some(successor)).unwrap();
    format!("---\ntype: ADR\nstatus: Superseded\nsuperseded_by: {superseded_by}\n---\n\n{line}\n\n# Old\n")
}

#[test]
fn a_callout_names_the_slugged_successor_the_bare_one_or_the_bare_number() {
    let slugged = run_callouts(&[
        ("/b/adr/0001-o.md", &retired("0002", "0002-n.md")),
        ("/b/adr/0002-n.md", "---\ntype: ADR\n---\n# N\n"),
    ]);
    let bare = run_callouts(&[
        ("/b/adr/0001-o.md", &retired("0002", "0002.md")),
        ("/b/adr/0002.md", "---\ntype: ADR\n---\n# N\n"),
    ]);
    let absent = run_callouts(&[("/b/adr/0001-o.md", &retired("0002", "0002"))]);

    assert!(slugged.is_empty(), "slugged successor: {slugged:?}");
    assert!(bare.is_empty(), "bare successor: {bare:?}");
    assert!(absent.is_empty(), "absent successor: {absent:?}");
}

#[test]
fn two_retired_records_in_one_directory_each_resolve_their_own_successor() {
    let found = run_callouts(&[
        ("/b/adr/0001-a.md", &retired("0002", "0002-b.md")),
        ("/b/adr/0002-b.md", "---\ntype: ADR\n---\n# B\n"),
        ("/b/adr/0003-c.md", &retired("0004", "0004-d.md")),
        ("/b/adr/0004-d.md", "---\ntype: ADR\n---\n# D\n"),
    ]);

    assert!(found.is_empty());
}

#[test]
fn successors_resolve_per_directory_without_leaking_across_directories() {
    let found = run_callouts(&[
        ("/b/adr/0001-a.md", &retired("0002", "0002-adr-new.md")),
        ("/b/adr/0002-adr-new.md", "---\ntype: ADR\n---\n# B\n"),
        ("/b/issues/0001-a.md", &retired("0002", "0002-issue-new.md")),
        ("/b/issues/0002-issue-new.md", "---\ntype: ADR\n---\n# B\n"),
    ]);

    assert!(found.is_empty());
}

#[test]
fn a_mismatched_callout_is_reported_for_that_record_only() {
    let found = run_callouts(&[
        ("/b/adr/0001-a.md", &retired("0002", "0002-wrong.md")),
        ("/b/adr/0002-b.md", "---\ntype: ADR\n---\n# B\n"),
    ]);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, "/b/adr/0001-a.md");
}
