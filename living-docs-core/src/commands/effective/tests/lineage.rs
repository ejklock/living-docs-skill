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
