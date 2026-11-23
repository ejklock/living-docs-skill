use super::*;
use crate::doc_type::spec_for_dir;

fn kinds_found(directory: Option<&str>, contents: &str) -> Vec<&'static str> {
    let spec = directory.and_then(spec_for_dir);
    findings(spec, contents)
        .into_iter()
        .map(|(kind, _)| kind)
        .collect()
}

#[test]
fn a_clean_adr_has_no_finding() {
    let contents = "---\ntype: ADR\nstatus: Accepted\ntimestamp: 2026-11-20T10:00:00Z\n---\n";
    assert!(kinds_found(Some("adr"), contents).is_empty());
}

#[test]
fn an_adr_with_every_defect_has_each_finding_once() {
    let contents = "---\ntype: BDR\nstatus: active\nsuperseded_by: 0002\n---\n";
    assert_eq!(kinds_found(Some("adr"), contents), vec![kinds::RECORD_TYPE]);
    assert!(successor_violation(None, contents, Some("active")).is_some());
}

#[test]
fn a_registry_type_outside_a_typed_directory_is_judged_by_its_declared_type() {
    let contents = "---\ntype: ADR\nstatus: active\n---\n";
    assert_eq!(
        kinds_found(None, contents),
        vec![kinds::STATUS_VOCABULARY, kinds::TIMESTAMP]
    );
}

#[test]
fn a_type_outside_the_registry_only_gets_the_successor_rule() {
    let contents = "---\ntype: Concept\nstatus: active\nsuperseded_by: 0002\n---\n";
    assert!(kinds_found(None, contents).is_empty());
    let message = successor_violation(None, contents, Some("active")).expect("a violation");
    assert!(message.contains("0002") && message.ends_with("(invariant 4)"));
}
