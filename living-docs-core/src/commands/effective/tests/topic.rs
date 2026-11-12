//! Topic filtering and ranking of the in-force view: weighted title,
//! description and body hits.

use super::*;

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
