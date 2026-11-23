use super::*;
use crate::test_support::MapStore;
use std::collections::HashSet;

fn slugs(markdown: &str) -> Vec<String> {
    heading_slugs(markdown)
}

fn slug_set(markdown: &str) -> HashSet<String> {
    heading_slugs(markdown).into_iter().collect()
}

#[test]
fn a_heading_slug_is_lowercase_with_spaces_as_hyphens() {
    assert_eq!(slugs("## Billing Rules\n"), ["billing-rules"]);
}

#[test]
fn punctuation_is_dropped_from_a_heading_slug() {
    assert_eq!(
        slugs("## Invoice: due date (net 30)\n"),
        ["invoice-due-date-net-30"]
    );
}

#[test]
fn letters_with_diacritics_are_kept() {
    assert_eq!(slugs("## Regras de cobrança\n"), ["regras-de-cobrança"]);
}

#[test]
fn each_space_becomes_a_hyphen_without_collapsing() {
    assert_eq!(slugs("## a  b\n"), ["a--b"]);
}

#[test]
fn inline_code_marks_do_not_reach_the_slug() {
    assert_eq!(slugs("## Use `check` now\n"), ["use-check-now"]);
}

#[test]
fn a_repeated_heading_gets_a_numbered_suffix_in_order() {
    assert_eq!(
        slugs("## Notes\n\n## Notes\n\n## Notes\n"),
        ["notes", "notes-1", "notes-2"]
    );
}

#[test]
fn a_setext_heading_is_a_heading() {
    assert_eq!(slugs("Title\n=====\n\ntext\n"), ["title"]);
}

#[test]
fn a_hash_line_inside_a_fence_is_not_a_heading() {
    assert_eq!(slugs("```\n# not a heading\n```\n\n## Real\n"), ["real"]);
}

#[test]
fn frontmatter_is_not_read_as_a_setext_heading() {
    assert_eq!(slugs("---\ntitle: X\n---\n\n## Real\n"), ["real"]);
}

#[test]
fn a_fragment_is_found_percent_encoded_or_plain() {
    let found = slug_set("## Regras de cobrança\n");
    assert!(has_fragment(&found, "regras-de-cobran%C3%A7a"));
    assert!(has_fragment(&found, "regras-de-cobrança"));
    assert!(!has_fragment(&found, "regras"));
}

#[test]
fn a_variation_selector_after_an_emoji_stays_in_the_slug() {
    assert_eq!(slugs("## ❤️ Love\n"), ["\u{FE0F}-love"]);
}

#[test]
fn combining_marks_of_a_script_stay_in_the_slug() {
    assert_eq!(slugs("## हिन्दी\n"), ["हिन्दी"]);
    assert_eq!(slugs("## x\u{0F84}y\n"), ["x\u{0F84}y"]);
}

#[test]
fn other_numbers_that_are_not_decimal_or_letter_numbers_are_dropped() {
    assert_eq!(slugs("## Area m²\n"), ["area-m"]);
    assert_eq!(slugs("## ½ off\n"), ["-off"]);
}

#[test]
fn a_circled_letter_symbol_that_is_alphabetic_stays_in_the_slug() {
    assert_eq!(slugs("## Ⓐ circled\n"), ["ⓐ-circled"]);
}

#[test]
fn an_images_alt_text_is_not_part_of_the_heading_text() {
    assert_eq!(slugs("## Logo ![alt text](x.png)\n"), ["logo-"]);
}

#[test]
fn a_link_text_in_a_heading_is_part_of_the_heading_text() {
    assert_eq!(
        slugs("## See [the rules](r.md) now\n"),
        ["see-the-rules-now"]
    );
}

fn advise_into(
    cache: &mut SlugCache,
    store: &MapStore,
    resolved: Option<(&str, &str)>,
    dest: &str,
) -> Reporter {
    let mut reporter = Reporter::new();
    cache.check_anchor(
        store,
        Path::new("/bundle/a.md"),
        "## Here\n",
        dest,
        resolved,
        &mut reporter,
    );
    reporter
}

#[test]
fn a_target_linked_many_times_is_parsed_once() {
    let (store, _) = MapStore::seeded(&[("/bundle/t.md", "## Rule\n")]);
    let mut cache = SlugCache::default();
    for _ in 0..5 {
        let r = advise_into(
            &mut cache,
            &store,
            Some(("/bundle/t.md", "/bundle")),
            "t.md#rule",
        );
        assert!(r.advisories.is_empty());
    }
    assert_eq!(cache.parses, 1);
}

#[test]
fn same_file_links_parse_the_linking_file_once() {
    let (store, _) = MapStore::seeded(&[]);
    let mut cache = SlugCache::default();
    for _ in 0..5 {
        advise_into(&mut cache, &store, None, "#here");
    }
    assert_eq!(cache.parses, 1);
}

#[test]
fn a_bundle_spelled_with_a_dot_segment_still_contains_its_normalized_targets() {
    let (store, _) = MapStore::seeded(&[("docs/t.md", "## Rule\n")]);
    for bundle in ["./docs", "docs/", "docs", "./docs/"] {
        let mut cache = SlugCache::default();
        let r = advise_into(&mut cache, &store, Some(("docs/t.md", bundle)), "t.md#gone");
        assert_eq!(r.advisories.len(), 1, "{bundle}");
    }
    let (store, _) = MapStore::seeded(&[("t.md", "## Rule\n")]);
    let mut cache = SlugCache::default();
    let r = advise_into(&mut cache, &store, Some(("t.md", ".")), "t.md#gone");
    assert_eq!(r.advisories.len(), 1);
}

#[test]
fn a_target_outside_the_bundle_is_not_judged() {
    let (store, _) = MapStore::seeded(&[("README.md", "## Rule\n")]);
    let mut cache = SlugCache::default();
    let r = advise_into(&mut cache, &store, Some(("README.md", "docs")), "x.md#gone");
    assert!(r.advisories.is_empty());
}
