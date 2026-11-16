use super::super::*;

fn title_of(path: &str, body: &str) -> String {
    let contents = format!("---\ntype: ADR\n---\n{body}");
    extract_record(Path::new(path), &contents).title
}

#[test]
fn stem_title_drops_the_leading_number_and_hyphen() {
    assert_eq!(title_of("adr/0012-some-thing.md", "Body.\n"), "some-thing");
}

#[test]
fn stem_title_without_a_number_prefix_is_unchanged() {
    assert_eq!(title_of("docs/notes.md", "Body.\n"), "notes");
}

#[test]
fn stem_title_keeps_a_prefix_of_fewer_than_four_digits() {
    assert_eq!(title_of("adr/012-x.md", "Body.\n"), "012-x");
}

#[test]
fn heading_inside_a_backtick_fence_is_not_the_title() {
    let body = "```\n# 0012. Fake\n```\n";
    assert_eq!(title_of("adr/0012-some-thing.md", body), "some-thing");
}

#[test]
fn heading_inside_a_tilde_fence_is_not_the_title() {
    let body = "~~~\n# 0012. Fake\n~~~\n";
    assert_eq!(title_of("adr/0012-some-thing.md", body), "some-thing");
}

#[test]
fn unfenced_heading_after_a_fenced_one_is_the_title_without_its_number() {
    let body = "```\n# 0012. Fake\n```\n\n# 0012. Real\n";
    assert_eq!(title_of("adr/0012-some-thing.md", body), "Real");
}

#[test]
fn heading_without_a_number_is_unchanged() {
    assert_eq!(title_of("adr/0012-x.md", "# Plain\n"), "Plain");
}

fn heading_title(heading: &str) -> String {
    title_of("adr/0012-stem.md", &format!("{heading}\n"))
}

#[test]
fn a_number_prefix_is_stripped_in_each_of_the_three_shapes() {
    let adr = format!("# ADR {} — Foo", "0001");
    assert_eq!(heading_title("# 0001. Foo"), "Foo");
    assert_eq!(heading_title("# 0001 — Foo"), "Foo");
    assert_eq!(heading_title(&adr), "Foo");
}

#[test]
fn a_five_digit_number_is_not_a_prefix() {
    assert_eq!(heading_title("# 00012. X"), "00012. X");
}

#[test]
fn an_adr_heading_without_a_number_is_unchanged() {
    assert_eq!(heading_title("# ADR Foo"), "ADR Foo");
}

#[test]
fn a_heading_that_is_only_a_number_prefix_gives_the_filename_stem() {
    assert_eq!(heading_title("# 0001. "), "stem");
    assert_eq!(heading_title("# 0001 — "), "stem");
}

#[test]
fn a_body_opening_with_a_rule_keeps_its_heading() {
    let body = "---\n# Real\n\ntext\n---\n";
    assert_eq!(title_of("adr/0012-stem.md", body), "Real");
}

#[test]
fn frontmatter_title_is_kept_verbatim() {
    let contents = "---\ntype: ADR\ntitle: 0012. Kept\n---\n# 0012. Other\n";
    let extracted = extract_record(Path::new("adr/0012-x.md"), contents);

    assert_eq!(extracted.title, "0012. Kept");
}
