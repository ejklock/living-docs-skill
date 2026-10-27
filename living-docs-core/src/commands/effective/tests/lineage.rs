//! The lineage line for a record that replaces several records, across
//! chains, diamonds, cycles and self-references.

use super::*;

/// A numbered ADR whose `supersedes` is the flow list of `ancestors`.
fn adr_replacing(number: &str, status: &str, ancestors: &[&str]) -> (String, String) {
    let list = ancestors.join(", ");
    let fm = format!(
        "---\ntype: ADR\ntitle: ADR {number}\ndescription: d\nstatus: {status}\nsupersedes: [{list}]\n---\n\nbody\n"
    );
    (adr(number, status, None, "").0, fm)
}

fn lineage_listing(files: &[(String, String)]) -> String {
    let pairs: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, c)| (p.as_str(), c.as_str()))
        .collect();
    compile(&store_of(&pairs), Path::new("docs"), &options(None, false))
}

#[test]
fn a_diamond_lineage_names_the_shared_origin_earliest_and_both_branches_intermediate() {
    let root = adr("0001", "Superseded", None, "origin");
    let left = adr_replacing("0002", "Superseded", &["0001"]);
    let right = adr_replacing("0003", "Superseded", &["0001"]);
    let head = adr_replacing("0004", "Accepted", &["0002", "0003"]);

    let out = lineage_listing(&[root, left, right, head]);

    assert!(
        out.contains("(supersedes 0001 via 0002, 0003)"),
        "diamond lineage:\n{out}"
    );
}

#[test]
fn a_two_record_cycle_terminates_with_the_last_visited_record_as_earliest() {
    let head = adr("0003", "Accepted", Some("0001"), "live");
    let one = adr_replacing("0001", "Superseded", &["0002"]);
    let two = adr_replacing("0002", "Superseded", &["0001"]);

    let out = lineage_listing(&[head, one, two]);

    assert!(
        out.contains("(supersedes 0002 via 0001)"),
        "cycle lineage:\n{out}"
    );
}

#[test]
fn a_record_that_supersedes_itself_terminates_naming_itself_once() {
    let head = adr_replacing("0003", "Accepted", &["0003"]);

    let out = lineage_listing(&[head]);

    assert!(out.contains("(supersedes 0003)"), "self lineage:\n{out}");
}

#[test]
fn a_lone_ancestor_with_a_record_and_no_ancestors_reads_as_earliest() {
    let old = adr("0133", "Superseded", None, "old");
    let head = adr_replacing("0134", "Accepted", &["0133"]);

    let out = lineage_listing(&[old, head]);

    assert!(out.contains("(supersedes 0133)"), "lone ancestor:\n{out}");
}

#[test]
fn an_ancestor_number_with_no_record_reads_as_earliest() {
    let head = adr_replacing("0134", "Accepted", &["0007"]);

    let out = lineage_listing(&[head]);

    assert!(
        out.contains("(supersedes 0007)"),
        "missing ancestor:\n{out}"
    );
}

#[test]
fn a_record_that_supersedes_nothing_has_no_lineage() {
    let head = adr("0134", "Accepted", None, "plain");

    let out = lineage_listing(&[head]);

    assert!(!out.contains("supersedes"), "no lineage expected:\n{out}");
}

/// A record of `kind` filed under `dir` as `name` whose `supersedes` is the flow list of `ancestors`.
fn record_at(
    dir: &str,
    name: &str,
    kind: &str,
    status: &str,
    ancestors: &[&str],
) -> (String, String) {
    let list = ancestors.join(", ");
    let fm = format!(
        "---\ntype: {kind}\ntitle: {name}\ndescription: d\nstatus: {status}\nsupersedes: [{list}]\n---\n\nbody\n"
    );
    (format!("docs/{dir}/{name}.md"), fm)
}

#[test]
fn records_of_different_types_sharing_a_number_each_follow_only_their_own_type() {
    let adr_old = record_at("adr", "0001-a", "ADR", "Superseded", &[]);
    let issue_old = record_at("issues", "0001-a", "Issue", "Superseded", &[]);
    let adr_mid = record_at("adr", "0002-b", "ADR", "Superseded", &["0001"]);
    let issue_head = record_at("issues", "0002-b", "Issue", "Accepted", &["0001"]);
    let adr_head = record_at("adr", "0003-c", "ADR", "Accepted", &["0002"]);

    let out = lineage_listing(&[adr_old, issue_old, adr_mid, issue_head, adr_head]);

    assert!(out.contains("(supersedes 0001)"), "issue lineage:\n{out}");
    assert!(
        out.contains("(supersedes 0001 via 0002)"),
        "adr lineage:\n{out}"
    );
}

#[test]
fn an_ancestor_never_borrows_the_ancestors_of_a_record_of_another_type_with_the_same_number() {
    let adr_old = record_at("adr", "0001-a", "ADR", "Superseded", &[]);
    let issue_old = record_at("issues", "0001-a", "Issue", "Superseded", &["0009"]);
    let adr_head = record_at("adr", "0002-b", "ADR", "Accepted", &["0001"]);

    let out = lineage_listing(&[adr_old, issue_old, adr_head]);

    assert!(out.contains("(supersedes 0001)"), "adr lineage:\n{out}");
    assert!(!out.contains("0009"), "foreign ancestor leaked:\n{out}");
}

#[test]
fn two_records_sharing_a_type_and_number_resolve_to_the_later_one_in_listing_order() {
    let first = record_at("adr", "0002-a", "ADR", "Superseded", &["0001"]);
    let later = record_at("adr", "0002-b", "ADR", "Superseded", &[]);
    let head = record_at("adr", "0005-c", "ADR", "Accepted", &["0002"]);

    let out = lineage_listing(&[first, later, head]);

    assert!(out.contains("(supersedes 0002)"), "later wins:\n{out}");
    assert!(!out.contains("via"), "earlier duplicate ignored:\n{out}");
}

#[test]
fn a_branch_with_its_own_ancestor_is_intermediate_even_when_that_ancestor_was_already_visited() {
    let root = adr("0001", "Superseded", None, "origin");
    let left = adr_replacing("0002", "Superseded", &["0001"]);
    let right = adr_replacing("0003", "Superseded", &["0001"]);
    let head = adr_replacing("0004", "Accepted", &["0003", "0002"]);

    let out = lineage_listing(&[root, left, right, head]);

    assert!(
        out.contains("(supersedes 0001 via 0003, 0002)"),
        "branch order follows the list:\n{out}"
    );
}
