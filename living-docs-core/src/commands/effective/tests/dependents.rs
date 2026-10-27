//! The dependents views: which live records link a record, directly or
//! through a chain of links.

use super::*;

const BUNDLE: &str = "docs";

fn path_of(number: &str) -> PathBuf {
    Path::new(BUNDLE).join("adr").join(format!("{number}-x.md"))
}

fn links(number: &str) -> String {
    format!("see [it](/adr/{number}-x.md)")
}

fn options_for(dependents: Dependents) -> Options {
    Options {
        dependents: Some(dependents),
        ..options(None, false)
    }
}

fn store_from(files: &[(String, String)]) -> MapStore {
    let pairs: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, c)| (p.as_str(), c.as_str()))
        .collect();
    store_of(&pairs)
}

/// `(number, distance)` of every record the query returns, in output order.
fn found(files: &[(String, String)], dependents: Dependents) -> Vec<(i32, Option<usize>)> {
    let store = store_from(files);
    let (_, views) = collect(&store, Path::new(BUNDLE), &options_for(dependents));
    views
        .iter()
        .map(|view| (view.number.unwrap(), view.distance))
        .collect()
}

fn direct(files: &[(String, String)], target: &str) -> Vec<i32> {
    let dependents = Dependents::LinksTo(path_of(target));
    found(files, dependents).iter().map(|(n, _)| *n).collect()
}

fn impact(files: &[(String, String)], target: &str) -> Vec<(i32, Option<usize>)> {
    found(files, Dependents::Impact(path_of(target)))
}

/// B and C link A, D links B.
fn chain() -> Vec<(String, String)> {
    vec![
        adr("0001", "Accepted", None, "root"),
        adr("0002", "Accepted", None, &links("0001")),
        adr("0003", "Accepted", None, &links("0001")),
        adr("0004", "Accepted", None, &links("0002")),
    ]
}

#[test]
fn links_to_returns_the_direct_linkers_in_reading_order() {
    assert_eq!(direct(&chain(), "0001"), vec![2, 3]);
}

#[test]
fn impact_returns_direct_linkers_at_one_then_chained_ones_at_two() {
    assert_eq!(
        impact(&chain(), "0001"),
        vec![(2, Some(1)), (3, Some(1)), (4, Some(2))]
    );
}

#[test]
fn impact_orders_by_distance_before_reading_order() {
    let files = vec![
        adr("0001", "Accepted", None, "root"),
        adr("0002", "Accepted", None, &links("0003")),
        adr("0003", "Accepted", None, &links("0001")),
    ];
    assert_eq!(impact(&files, "0001"), vec![(3, Some(1)), (2, Some(2))]);
}

#[test]
fn a_cycle_terminates_and_lists_the_other_record_once() {
    let files = vec![
        adr("0001", "Accepted", None, &links("0002")),
        adr("0002", "Accepted", None, &links("0001")),
    ];
    assert_eq!(impact(&files, "0001"), vec![(2, Some(1))]);
}

#[test]
fn a_superseded_or_deprecated_linker_is_absent() {
    let files = vec![
        adr("0001", "Accepted", None, "root"),
        adr("0002", "Superseded", None, &links("0001")),
        adr("0003", "Deprecated", None, &links("0001")),
        adr("0004", "Accepted", None, &links("0001")),
    ];
    assert_eq!(direct(&files, "0001"), vec![4]);
    assert_eq!(impact(&files, "0001"), vec![(4, Some(1))]);
}

#[test]
fn a_chain_passes_through_a_retired_record_but_never_lists_it() {
    let files = vec![
        adr("0001", "Accepted", None, "root"),
        adr("0003", "Superseded", None, &links("0001")),
        adr("0004", "Accepted", None, &links("0003")),
        adr("0005", "Accepted", None, &links("0001")),
    ];
    assert_eq!(impact(&files, "0001"), vec![(5, Some(1)), (4, Some(2))]);
    assert_eq!(direct(&files, "0001"), vec![5]);
}

#[test]
fn a_target_whose_only_linker_is_retired_gives_an_empty_result() {
    let files = vec![
        adr("0001", "Accepted", None, "root"),
        adr("0002", "Superseded", None, &links("0001")),
    ];
    assert!(direct(&files, "0001").is_empty());
    assert!(impact(&files, "0001").is_empty());
}

#[test]
fn a_retired_target_still_returns_its_live_linkers() {
    let files = vec![
        adr("0001", "Superseded", None, "old"),
        adr("0002", "Accepted", None, &links("0001")),
    ];
    assert_eq!(direct(&files, "0001"), vec![2]);
}

#[test]
fn a_link_inside_a_fenced_code_block_adds_nothing() {
    let fenced = "```\n[it](/adr/0001-x.md)\n```";
    let files = vec![
        adr("0001", "Accepted", None, "root"),
        adr("0002", "Accepted", None, fenced),
    ];
    assert!(direct(&files, "0001").is_empty());
}

#[test]
fn a_link_from_an_index_adds_nothing() {
    let files = vec![
        adr("0001", "Accepted", None, "root"),
        (
            path_of("index")
                .with_file_name("index.md")
                .display()
                .to_string(),
            "# Index\n\n[a](/adr/0001-x.md)\n".to_string(),
        ),
    ];
    assert!(direct(&files, "0001").is_empty());
}

#[test]
fn a_self_link_adds_nothing_and_the_target_is_never_in_its_result() {
    let files = vec![
        adr("0001", "Accepted", None, &links("0001")),
        adr("0002", "Accepted", None, &links("0001")),
    ];
    assert_eq!(direct(&files, "0001"), vec![2]);
    assert_eq!(impact(&files, "0001"), vec![(2, Some(1))]);
}

#[test]
fn bundle_relative_file_relative_and_anchored_links_all_count() {
    let files = vec![
        adr("0001", "Accepted", None, "root"),
        adr("0002", "Accepted", None, "[a](/adr/0001-x.md)"),
        adr("0003", "Accepted", None, "[a](0001-x.md)"),
        adr("0004", "Accepted", None, "[a](0001-x.md#context)"),
        adr("0005", "Accepted", None, "![a](../adr/0001-x.md)"),
    ];
    assert_eq!(direct(&files, "0001"), vec![2, 3, 4, 5]);
}

#[test]
fn an_external_link_adds_nothing() {
    let files = vec![
        adr("0001", "Accepted", None, "root"),
        adr(
            "0002",
            "Accepted",
            None,
            "[a](https://example.com/adr/0001-x.md)",
        ),
    ];
    assert!(direct(&files, "0001").is_empty());
}

#[test]
fn a_record_nothing_links_gives_an_empty_result() {
    assert!(direct(&chain(), "0004").is_empty());
    assert!(impact(&chain(), "0004").is_empty());
}

#[test]
fn a_record_reaching_the_target_two_ways_appears_once_at_the_shorter_distance() {
    let both = format!("{} {}", links("0001"), links("0002"));
    let files = vec![
        adr("0001", "Accepted", None, "root"),
        adr("0002", "Accepted", None, &links("0001")),
        adr("0004", "Accepted", None, &both),
    ];
    assert_eq!(impact(&files, "0001"), vec![(2, Some(1)), (4, Some(1))]);
}

fn json_records(
    dependents: Option<Dependents>,
    full: bool,
    contract: bool,
) -> Vec<serde_json::Value> {
    let store = store_from(&chain());
    let opts = Options {
        full,
        contract,
        dependents,
        ..options(None, false)
    };
    let json = compile_json(&store, Path::new("docs"), &opts);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value["records"].as_array().unwrap().clone()
}

#[test]
fn json_carries_distance_only_under_impact() {
    let impact = json_records(Some(Dependents::Impact(path_of("0001"))), false, false);
    let distances: Vec<_> = impact.iter().map(|r| r["distance"].as_u64()).collect();
    assert_eq!(distances, vec![Some(1), Some(1), Some(2)]);

    let direct = json_records(Some(Dependents::LinksTo(path_of("0001"))), false, false);
    assert_eq!(direct.len(), 2);
    assert!(direct.iter().all(|r| r.get("distance").is_none()));

    let plain = json_records(None, false, false);
    assert!(plain.iter().all(|r| r.get("distance").is_none()));
}

#[test]
fn full_and_contract_bodies_are_the_same_with_and_without_a_dependents_flag() {
    for (full, contract) in [(true, false), (false, true)] {
        let plain = json_records(None, full, contract);
        let narrowed = json_records(Some(Dependents::Impact(path_of("0001"))), full, contract);
        assert_eq!(narrowed.len(), 3);
        for record in &narrowed {
            let same = plain
                .iter()
                .find(|p| p["number"] == record["number"])
                .unwrap();
            assert!(record["body"].is_string());
            assert_eq!(record["body"], same["body"]);
        }
    }
}
