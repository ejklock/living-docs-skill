use super::*;
use crate::test_support::MapStore;
use std::collections::BTreeMap;

fn prd(status: &str, body: &str) -> String {
    format!("---\ntype: PRD\ntitle: P\nstatus: {status}\n---\n\n{body}\n")
}

fn bdr(body: &str) -> String {
    format!("---\ntype: BDR\ntitle: B\nstatus: Draft\n---\n\n{body}\n")
}

fn run_over(files: Vec<(&str, String)>) -> Reporter {
    let store = MapStore {
        files: files
            .iter()
            .map(|(path, contents)| (PathBuf::from(path), contents.clone()))
            .collect::<BTreeMap<_, _>>(),
    };
    let all_md: Vec<PathBuf> = files.iter().map(|(path, _)| PathBuf::from(path)).collect();
    let mut reporter = Reporter::new();
    check_requirement_traceability(&store, &all_md, &mut reporter);
    reporter
}

fn messages(entries: &[(String, String)]) -> Vec<&str> {
    entries.iter().map(|(_, message)| message.as_str()).collect()
}

#[test]
fn implemented_prd_with_an_uncovered_id_is_a_violation() {
    let reporter = run_over(vec![(
        "/b/prd/0001-p.md",
        prd("Implemented", "- **FR-1** — The system shall respond."),
    )]);

    assert!(reporter.advisories.is_empty());
    assert_eq!(reporter.violations.len(), 1);
    assert!(messages(&reporter.violations)[0].contains("TRACE FR-1"));
}

#[test]
fn accepted_prd_with_an_uncovered_id_is_an_advisory_never_a_violation() {
    let reporter = run_over(vec![(
        "/b/prd/0001-p.md",
        prd("Accepted", "- **FR-1** — The system shall respond."),
    )]);

    assert!(reporter.violations.is_empty());
    assert_eq!(reporter.advisories.len(), 1);
    assert!(messages(&reporter.advisories)[0].contains("TRACE FR-1"));
}

#[test]
fn draft_and_superseded_prds_are_out_of_scope() {
    for status in ["Draft", "Superseded"] {
        let reporter = run_over(vec![(
            "/b/prd/0001-p.md",
            prd(status, "- **FR-1** — The system shall respond."),
        )]);

        assert!(reporter.violations.is_empty(), "{status} must not violate");
        assert!(reporter.advisories.is_empty(), "{status} must not advise");
    }
}

#[test]
fn an_implemented_prd_defining_no_ids_passes_untouched() {
    let reporter = run_over(vec![(
        "/b/prd/0001-p.md",
        prd("Implemented", "1. The system shall respond."),
    )]);

    assert!(reporter.violations.is_empty());
    assert!(reporter.advisories.is_empty());
}

#[test]
fn a_bdr_linking_the_prd_and_citing_the_id_covers_it() {
    let reporter = run_over(vec![
        (
            "/b/prd/0001-p.md",
            prd("Implemented", "- **FR-1** — The system shall respond."),
        ),
        (
            "/b/bdr/0002-b.md",
            bdr("Covers [PRD](/prd/0001-p.md).\n\n- Proves: FR-1"),
        ),
    ]);

    assert!(reporter.violations.is_empty());
    assert!(reporter.advisories.is_empty());
}

#[test]
fn a_bdr_citing_the_id_without_linking_the_prd_earns_no_coverage() {
    let reporter = run_over(vec![
        (
            "/b/prd/0001-p.md",
            prd("Implemented", "- **FR-1** — The system shall respond."),
        ),
        ("/b/bdr/0002-b.md", bdr("- Proves: FR-1")),
    ]);

    assert_eq!(reporter.violations.len(), 1);
}

#[test]
fn nfr_ids_participate_and_are_never_mistaken_for_fr_ids() {
    let reporter = run_over(vec![
        (
            "/b/prd/0001-p.md",
            prd("Implemented", "| NFR-1 | Performance | scenario | CI floor |"),
        ),
        (
            "/b/bdr/0002-b.md",
            bdr("Covers [PRD](/prd/0001-p.md).\n\n- Proves: FR-1"),
        ),
    ]);

    assert_eq!(reporter.violations.len(), 1);
    assert!(messages(&reporter.violations)[0].contains("TRACE NFR-1"));
}

#[test]
fn each_uncovered_id_is_reported_once_even_when_defined_twice() {
    let reporter = run_over(vec![(
        "/b/prd/0001-p.md",
        prd("Implemented", "- **FR-1** — a\n\nAcceptance: FR-1 holds."),
    )]);

    assert_eq!(reporter.violations.len(), 1);
}

#[test]
fn requirement_ids_extracts_boundary_checked_fr_and_nfr_tokens() {
    let ids = requirement_ids("FR-1 NFR-2 (FR-10) PRD-0007-FR-3 FR-4.");

    let expected: Vec<&str> = vec!["FR-1", "FR-10", "FR-3", "FR-4", "NFR-2"];
    assert_eq!(ids.iter().map(String::as_str).collect::<Vec<_>>(), expected);
}

#[test]
fn requirement_ids_rejects_non_id_shapes() {
    assert!(requirement_ids("XFR-1 ANFR-2 FR- FR-1x NFRs FR-N").is_empty());
}
