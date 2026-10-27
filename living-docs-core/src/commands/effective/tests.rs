use super::*;
use crate::test_support::MapStore;
use std::collections::BTreeMap;

mod lineage;

fn store_of(files: &[(&str, &str)]) -> MapStore {
    let mut map = BTreeMap::new();
    for (path, contents) in files {
        map.insert(PathBuf::from(path), (*contents).to_string());
    }
    MapStore { files: map }
}

fn options(topic: Option<&str>, full: bool) -> Options {
    Options {
        topic: topic.map(str::to_string),
        full,
        contract: false,
    }
}

fn adr(number: &str, status: &str, supersedes: Option<&str>, body: &str) -> (String, String) {
    let mut fm = format!("---\ntype: ADR\ntitle: ADR {number}\ndescription: d\nstatus: {status}\n");
    if let Some(s) = supersedes {
        fm.push_str(&format!("supersedes: \"{s}\"\n"));
    }
    fm.push_str("---\n\n");
    (format!("docs/adr/{number}-x.md"), format!("{fm}{body}"))
}

#[test]
fn a_supersede_chain_collapses_to_the_head_with_a_lineage_line() {
    let a = adr("0131", "Superseded", None, "old");
    let b = adr("0133", "Superseded", Some("0131"), "middle");
    let c = adr("0143", "Accepted", Some("0133"), "the rule in force");
    let store = store_of(&[(&a.0, &a.1), (&b.0, &b.1), (&c.0, &c.1)]);
    let out = compile(&store, Path::new("docs"), &options(None, false));

    assert!(out.contains("ADR 0143"), "head must appear:\n{out}");
    assert!(
        out.contains("supersedes 0131 via 0133"),
        "lineage line:\n{out}"
    );
    assert_eq!(
        out.matches("- [ADR").count(),
        1,
        "only the head is a row:\n{out}"
    );
}

#[test]
fn superseded_and_deprecated_records_are_excluded() {
    let a = adr("0001", "Superseded", None, "gone");
    let b = adr("0002", "Deprecated", None, "retired");
    let c = adr("0003", "Accepted", None, "live");
    let store = store_of(&[(&a.0, &a.1), (&b.0, &b.1), (&c.0, &c.1)]);
    let out = compile(&store, Path::new("docs"), &options(None, false));
    assert!(out.contains("ADR 0003"));
    assert!(!out.contains("ADR 0001"));
    assert!(!out.contains("ADR 0002"));
}

#[test]
fn a_proposed_record_stays_in_force() {
    let proposed = adr("0001", "Proposed", None, "a live proposal");
    let store = store_of(&[(&proposed.0, &proposed.1)]);
    let out = compile(&store, Path::new("docs"), &options(None, false));
    assert!(
        out.contains("ADR 0001"),
        "a proposal is still active:\n{out}"
    );
}

#[test]
fn topic_filters_by_case_insensitive_term_across_title_and_body() {
    let a = adr("0001", "Accepted", None, "concerns the STORAGE backend");
    let b = adr("0002", "Accepted", None, "about mermaid diagrams");
    let store = store_of(&[(&a.0, &a.1), (&b.0, &b.1)]);
    let out = compile(&store, Path::new("docs"), &options(Some("storage"), false));
    assert!(out.contains("ADR 0001"));
    assert!(!out.contains("ADR 0002"));
}

#[test]
fn constitution_and_prd_group_above_adrs() {
    let store = store_of(&[
        (
            "docs/adr/0001-x.md",
            "---\ntype: ADR\ntitle: A\ndescription: d\nstatus: Accepted\n---\n\nb",
        ),
        (
            "docs/prd/0001-x.md",
            "---\ntype: PRD\ntitle: P\ndescription: d\nstatus: Accepted\n---\n\nb",
        ),
        (
            "docs/constitution.md",
            "---\ntype: Constitution\ntitle: C\ndescription: d\n---\n\nb",
        ),
    ]);
    let out = compile(&store, Path::new("docs"), &options(None, false));
    let c = out.find("[Constitution]").unwrap();
    let p = out.find("[PRD 0001]").unwrap();
    let a = out.find("[ADR 0001]").unwrap();
    assert!(
        c < p && p < a,
        "order should be constitution, PRD, ADR:\n{out}"
    );
}

#[test]
fn withheld_retired_records_open_the_view_with_their_count() {
    let a = adr("0001", "Superseded", None, "gone");
    let b = adr("0002", "Deprecated", None, "retired");
    let c = adr("0003", "Accepted", None, "live");
    let store = store_of(&[(&a.0, &a.1), (&b.0, &b.1), (&c.0, &c.1)]);
    let out = compile(&store, Path::new("docs"), &options(None, false));
    assert!(
        out.starts_with(
            "_Withheld 2 retired record(s) (superseded or deprecated): history only, never act on them._\n\n"
        ),
        "must open with the withheld count and a blank line:\n{out}"
    );
}

#[test]
fn no_withheld_records_leaves_the_view_unchanged() {
    let a = adr("0001", "Accepted", None, "live");
    let store = store_of(&[(&a.0, &a.1)]);
    let out = compile(&store, Path::new("docs"), &options(None, false));
    assert!(
        !out.contains("Withheld"),
        "no retired records means no withheld line:\n{out}"
    );
}

#[test]
fn full_prints_bodies_while_the_default_prints_one_line_entries() {
    let a = adr("0001", "Accepted", None, "## Context\n\nthe whole story");
    let store = store_of(&[(&a.0, &a.1)]);

    let index = compile(&store, Path::new("docs"), &options(None, false));
    assert!(
        !index.contains("the whole story"),
        "index omits bodies:\n{index}"
    );

    let full = compile(&store, Path::new("docs"), &options(None, true));
    assert!(
        full.contains("the whole story"),
        "full includes bodies:\n{full}"
    );
}

fn titled(number: &str, title: &str, description: &str, body: &str) -> (String, String) {
    (
        format!("docs/{number}-x.md"),
        format!(
            "---\ntype: ADR\ntitle: {title}\ndescription: {description}\nstatus: Accepted\n---\n\n{body}"
        ),
    )
}

fn retired(number: &str, title: &str) -> (String, String) {
    (
        format!("docs/{number}-x.md"),
        format!("---\ntype: ADR\ntitle: {title}\ndescription: d\nstatus: Superseded\n---\n\nb"),
    )
}

fn listed(files: &[(String, String)], topic: Option<&str>) -> String {
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, c)| (p.as_str(), c.as_str()))
        .collect();
    compile(&store_of(&refs), Path::new("docs"), &options(topic, false))
}

#[test]
fn a_title_hit_ranks_above_a_body_only_mention() {
    let body_only = titled("0001", "Storage notes", "d", "we use a cache here");
    let in_title = titled("0002", "Cache strategy", "d", "plain");
    let out = listed(&[body_only, in_title], Some("cache"));
    let title_at = out.find("Cache strategy").expect("title hit listed");
    let body_at = out.find("Storage notes").expect("body hit listed");
    assert!(title_at < body_at, "title hit first:\n{out}");
}

#[test]
fn a_term_must_start_a_word_never_sit_inside_one() {
    let a = titled("0001", "Cache strategy", "d", "b");
    let out = listed(&[a], Some("ache"));
    assert!(
        !out.contains("Cache strategy"),
        "mid-word is no match:\n{out}"
    );
}

#[test]
fn every_term_must_match_a_word_in_some_field() {
    let both = titled("0001", "Owner rules", "the style guide", "b");
    let one = titled("0002", "Owner only", "d", "b");
    let out = listed(&[both, one], Some("owner style"));
    assert!(out.contains("Owner rules"), "both terms present:\n{out}");
    assert!(
        !out.contains("Owner only"),
        "one term is not enough:\n{out}"
    );
}

#[test]
fn accented_and_plain_topics_list_the_same_records() {
    let a = titled("0001", "Title", "d", "a decisão da ação foi tomada");
    let files = [a];
    let accented = listed(&files, Some("ação"));
    assert!(
        accented.contains("Title"),
        "accented topic matches:\n{accented}"
    );
    assert_eq!(accented, listed(&files, Some("acao")));
    assert_eq!(accented, listed(&files, Some("AÇÃO")));
}

#[test]
fn topic_case_does_not_change_the_result() {
    let files = [titled("0001", "Cache strategy", "d", "b")];
    assert_eq!(listed(&files, Some("CACHE")), listed(&files, Some("cache")));
}

#[test]
fn a_retired_record_never_appears_even_when_it_matches() {
    let out = listed(&[retired("0001", "Caching old")], Some("caching"));
    assert!(!out.contains("Caching old"), "retired withheld:\n{out}");
}

#[test]
fn equal_scores_keep_the_reading_order() {
    let later = titled("0002", "Alpha topic", "d", "b");
    let earlier = titled("0001", "Beta topic", "d", "b");
    let out = listed(&[later, earlier], Some("topic"));
    assert!(out.find("Beta topic") < out.find("Alpha topic"), "{out}");
}

#[test]
fn reading_reports_how_many_records_matched_and_were_withheld() {
    let live = titled("0001", "Caching", "d", "b");
    let gone = retired("0002", "Old");
    let store = store_of(&[(&live.0, &live.1), (&gone.0, &gone.1)]);
    let miss = options(Some("zzqq"), false);
    let none = read(&store, Path::new("docs"), &miss, false);
    assert_eq!((none.matched, none.withheld), (0, 1));
    let hit = options(Some("cach"), false);
    let some = read(&store, Path::new("docs"), &hit, true);
    assert_eq!(some.matched, 1);
    assert!(
        some.output.starts_with("{\"withheld\":1"),
        "{}",
        some.output
    );
}

#[test]
fn without_a_topic_records_list_in_base_rank_order_by_number() {
    let adr_late = titled("0009", "Late adr", "d", "b");
    let adr_early = titled("0002", "Early adr", "d", "b");
    let prd = (
        "docs/0005-p.md".to_string(),
        "---\ntype: PRD\ntitle: A prd\ndescription: d\nstatus: Accepted\n---\n\nb".to_string(),
    );
    let out = listed(&[adr_late, adr_early, prd], None);
    let at = |needle: &str| out.find(needle).expect(needle);
    assert!(at("A prd") < at("Early adr"), "{out}");
    assert!(at("Early adr") < at("Late adr"), "{out}");
}

#[test]
fn a_whitespace_only_topic_lists_the_same_as_no_topic() {
    let files = [
        titled("0002", "Alpha", "d", "b"),
        titled("0001", "Beta", "d", "b"),
        retired("0003", "Gone"),
    ];
    assert_eq!(listed(&files, Some("   ")), listed(&files, None));
}

#[test]
fn a_topic_keeps_only_matches_ranked_by_weighted_hits() {
    let title_hit = titled("0003", "Storage layout", "d", "b");
    let desc_hit = titled("0002", "Other", "storage plan", "b");
    let body_hit = titled("0001", "Misc", "d", "storage mention");
    let miss = titled("0004", "Unrelated", "d", "nothing");
    let out = listed(&[body_hit, desc_hit, title_hit, miss], Some("storage"));
    let at = |needle: &str| out.find(needle).expect(needle);
    assert!(at("Storage layout") < at("Other"), "{out}");
    assert!(at("Other") < at("Misc"), "{out}");
    assert!(!out.contains("Unrelated"), "{out}");
}
