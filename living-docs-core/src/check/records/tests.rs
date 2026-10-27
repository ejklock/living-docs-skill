use super::*;
use crate::test_support::MapStore;
use std::process::ExitCode;

#[test]
fn is_reserved_matches_index_and_log_only() {
    assert!(is_reserved("index.md"));
    assert!(is_reserved("log.md"));
    assert!(!is_reserved("foo.md"));
}

fn sibling_ids_for(paths: &[&str]) -> SiblingIds {
    let all_md: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    SiblingIds::new(&all_md)
}

#[test]
fn sibling_ids_match_dash_prefixed_and_bare_forms() {
    let dir = Path::new("/bundle/adr");
    let ids = sibling_ids_for(&["/bundle/adr/0007-old.md", "/bundle/adr/0042.md"]);

    assert!(ids.contains(dir, "0007"));
    assert!(!ids.contains(dir, "9999"));
    assert!(ids.contains(dir, "0042"));
}

#[test]
fn sibling_ids_ignore_matches_outside_the_directory() {
    let ids = sibling_ids_for(&["/bundle/bdr/0007-old.md"]);

    assert!(!ids.contains(Path::new("/bundle/adr"), "0007"));
}

#[test]
fn sibling_ids_match_a_slugged_id_by_its_stem_or_a_longer_slug() {
    let dir = Path::new("/bundle/adr");
    let longer = sibling_ids_for(&["/bundle/adr/0002-x-y.md"]);
    let exact = sibling_ids_for(&["/bundle/adr/0002-x.md"]);

    assert!(longer.contains(dir, "0002-x"));
    assert!(exact.contains(dir, "0002-x"));
    assert!(!exact.contains(dir, "0002-x-y"));
}

fn supersede_reports(superseded_by: &str, siblings: &[&str]) -> Vec<String> {
    let old = format!("---\ntype: ADR\nstatus: Superseded\n{superseded_by}\n---\n# Old\n");
    let mut seed = vec![(OLD_ADR, old.as_str())];
    seed.extend(siblings.iter().map(|path| (*path, NEW_ADR_ACCEPTED)));
    let (store, all_md) = MapStore::seeded(&seed);
    let mut reporter = Reporter::new();
    check_supersede_chain(&store, &all_md, &mut reporter);
    reporter.violations.into_iter().map(|v| v.message).collect()
}

#[test]
fn check_supersede_chain_resolves_a_numeric_target_to_a_slugged_or_bare_sibling() {
    assert!(supersede_reports("superseded_by: 0002", &["/bundle/adr/0002-x.md"]).is_empty());
    assert!(supersede_reports("superseded_by: 0002", &["/bundle/adr/0002.md"]).is_empty());
}

#[test]
fn check_supersede_chain_reports_a_target_found_only_elsewhere_or_only_as_a_longer_number() {
    let expected = "superseded_by: 0002 has no matching record in /bundle/adr (invariant 4)";
    for sibling in ["/bundle/other/0002-x.md", "/bundle/adr/00020-y.md"] {
        assert_eq!(
            supersede_reports("superseded_by: 0002", &[sibling]),
            vec![expected.to_string()],
            "{sibling}"
        );
    }
}

#[test]
fn check_supersede_chain_resolves_a_slugged_target() {
    assert!(supersede_reports("superseded_by: 0002-x", &["/bundle/adr/0002-x-y.md"]).is_empty());
    assert!(supersede_reports("superseded_by: 0002-x", &["/bundle/adr/0002-x.md"]).is_empty());
}

#[test]
fn check_supersede_chain_reports_an_empty_target() {
    assert_eq!(
        supersede_reports("superseded_by:", &[]),
        vec!["status: Superseded but superseded_by is empty (invariant 4)".to_string()]
    );
}

fn exit_code_is_success(code: ExitCode) -> bool {
    format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
}

const OLD_ADR: &str = "/bundle/adr/0001-old.md";
const OLD_ADR_SUPERSEDED: &str =
    "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0002\n---\n# Old\n";
const NEW_ADR: &str = "/bundle/adr/0002-new.md";
const NEW_ADR_ACCEPTED: &str = "---\ntype: ADR\nstatus: Accepted\n---\n# New\n";

#[test]
fn check_frontmatter_and_format_accepts_content_the_store_serves_with_no_disk_backing() {
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/adr/0001-title.md",
        "---\ntype: ADR\n---\n# Title\n",
    )]);
    let root_index = PathBuf::from("/bundle/index.md");
    let mut reporter = Reporter::new();

    check_frontmatter_and_format(&store, &all_md, &root_index, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}

#[test]
fn check_frontmatter_and_format_reports_content_the_store_serves_as_missing_frontmatter() {
    let (store, all_md) = MapStore::seeded(&[("/bundle/adr/0001-title.md", "# No frontmatter\n")]);
    let root_index = PathBuf::from("/bundle/index.md");
    let mut reporter = Reporter::new();

    check_frontmatter_and_format(&store, &all_md, &root_index, &mut reporter);

    assert!(!exit_code_is_success(reporter.finish()));
}

#[test]
fn check_supersede_chain_reports_a_target_absent_from_all_md() {
    let (store, all_md) = MapStore::seeded(&[(OLD_ADR, OLD_ADR_SUPERSEDED)]);
    let mut reporter = Reporter::new();

    check_supersede_chain(&store, &all_md, &mut reporter);

    assert!(!exit_code_is_success(reporter.finish()));
}

#[test]
fn check_supersede_chain_passes_when_the_target_is_present_in_all_md() {
    let (store, all_md) =
        MapStore::seeded(&[(OLD_ADR, OLD_ADR_SUPERSEDED), (NEW_ADR, NEW_ADR_ACCEPTED)]);
    let mut reporter = Reporter::new();

    check_supersede_chain(&store, &all_md, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}

#[test]
fn check_owner_requirement_advises_on_an_adr_without_owner_and_stays_exit_zero() {
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/adr/0001-title.md",
        "---\ntype: ADR\ntitle: Title\n---\n# Title\n",
    )]);
    let mut reporter = Reporter::new();

    check_owner_requirement(&store, &all_md, false, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}

#[test]
fn check_owner_requirement_reports_a_violation_on_an_adr_without_owner_when_required() {
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/adr/0001-title.md",
        "---\ntype: ADR\ntitle: Title\n---\n# Title\n",
    )]);
    let mut reporter = Reporter::new();

    check_owner_requirement(&store, &all_md, true, &mut reporter);

    assert!(!exit_code_is_success(reporter.finish()));
}

#[test]
fn check_owner_requirement_passes_when_an_adr_carries_an_owner() {
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/adr/0001-title.md",
        "---\ntype: ADR\ntitle: Title\nowner: alice\n---\n# Title\n",
    )]);
    let mut reporter = Reporter::new();

    check_owner_requirement(&store, &all_md, true, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}

fn heading_advisories(reporter: &Reporter) -> Vec<&str> {
    reporter
        .advisories
        .iter()
        .filter(|advisory| advisory.kind == "heading")
        .map(|advisory| advisory.message.as_str())
        .collect()
}

fn heading_drift_adr(status: &str) -> (MapStore, Vec<PathBuf>) {
    MapStore::seeded(&[(
        "/bundle/adr/0001-a-decision.md",
        &format!(
            "---\ntype: ADR\ntitle: A Decision\nstatus: {status}\n---\n\n# 0001. Not The Title\n"
        ),
    )])
}

fn assert_heading_drift(status: &str, expected: &[&str]) {
    let (store, all_md) = heading_drift_adr(status);
    let mut reporter = Reporter::new();

    check_heading_matches_title(&store, &all_md, &mut reporter);

    assert_eq!(heading_advisories(&reporter), expected);
}

#[test]
fn check_heading_matches_title_advises_on_a_live_heading_drift() {
    let (store, all_md) = heading_drift_adr("Accepted");
    let mut reporter = Reporter::new();

    check_heading_matches_title(&store, &all_md, &mut reporter);

    let messages = heading_advisories(&reporter);
    assert_eq!(messages.len(), 1);
    assert!(messages[0].contains("HEADING"), "{messages:?}");
}

fn heading_advisories_for(path: &str, record: &str) -> Vec<String> {
    let (store, all_md) = MapStore::seeded(&[(path, record)]);
    let mut reporter = Reporter::new();
    check_heading_matches_title(&store, &all_md, &mut reporter);
    heading_advisories(&reporter)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

const HAND_FRONT: &str = "---\ntype: ADR\ntitle: Hand\nstatus: Accepted\n---\n\n";

#[test]
fn check_heading_ignores_a_fenced_numbered_line_before_the_real_heading() {
    let record = format!("{HAND_FRONT}```\n# 0003. Fake\n```\n\n# 0003. Hand\n");
    assert!(heading_advisories_for("/b/adr/0003-hand.md", &record).is_empty());
}

#[test]
fn check_heading_names_the_real_heading_when_the_fenced_one_also_differs() {
    let record = format!("{HAND_FRONT}```\n# 0003. Fake\n```\n\n# 0003. Real\n");
    let messages = heading_advisories_for("/b/adr/0003-hand.md", &record);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(messages[0].contains("'Real'"), "{messages:?}");
}

#[test]
fn check_heading_ignores_a_record_whose_only_heading_is_a_section() {
    let record = format!("{HAND_FRONT}## Context\n\nText.\n");
    assert!(heading_advisories_for("/b/adr/0003-hand.md", &record).is_empty());
}

#[test]
fn check_heading_keeps_judging_an_unnumbered_record_by_its_first_heading() {
    let record = "---\ntype: Architecture View\ntitle: Overview\n---\n\n# Something else\n";
    let messages = heading_advisories_for("/b/architecture/overview.md", record);
    assert_eq!(messages.len(), 1, "{messages:?}");
}

#[test]
fn check_heading_matches_title_skips_a_deprecated_records_drift() {
    assert_heading_drift("Deprecated", &[]);
}

#[test]
fn check_heading_matches_title_skips_a_superseded_records_drift() {
    assert_heading_drift("Superseded", &[]);
}

#[test]
fn check_heading_matches_title_skips_a_case_insensitive_deprecated_status() {
    assert_heading_drift("deprecated", &[]);
}

#[test]
fn check_supersede_chain_still_reports_a_retired_records_invariant_violation() {
    let (store, all_md) = MapStore::seeded(&[(
        OLD_ADR,
        "---\ntype: ADR\nstatus: Superseded\n---\n# 0001. Old\n",
    )]);
    let mut reporter = Reporter::new();

    check_supersede_chain(&store, &all_md, &mut reporter);

    assert!(!exit_code_is_success(reporter.finish()));
}

#[test]
fn check_owner_requirement_never_flags_a_type_whose_registry_row_does_not_require_it() {
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/issues/0001-title.md",
        "---\ntype: Issue\ntitle: Title\n---\n# Title\n",
    )]);
    let mut reporter = Reporter::new();

    check_owner_requirement(&store, &all_md, true, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}
