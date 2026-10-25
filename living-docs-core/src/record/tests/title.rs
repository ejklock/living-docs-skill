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

#[test]
fn frontmatter_title_is_kept_verbatim() {
    let contents = "---\ntype: ADR\ntitle: 0012. Kept\n---\n# 0012. Other\n";
    let extracted = extract_record(Path::new("adr/0012-x.md"), contents);

    assert_eq!(extracted.title, "0012. Kept");
}
