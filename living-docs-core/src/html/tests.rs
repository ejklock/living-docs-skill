use super::*;
use crate::test_support::MapStore;

const BUNDLE: &str = "/private/secret-location/docs";
const PINNED_TAG: &str = "<script src=\"https://cdn.jsdelivr.net/npm/mermaid@12.3.0/dist/mermaid.min.js\" integrity=\"sha384-HHW54r639fK42Ip0coGF3mz8oSiGn0eEoWNTLs75X7H63pLSDBa0Tqs0RwGmLlXE\" crossorigin=\"anonymous\"></script>";

fn file(name: &str, front: &str, body: &str) -> (String, String) {
    (
        format!("{BUNDLE}/{name}"),
        format!("---\n{front}\n---\n\n{body}\n"),
    )
}

fn adr(number: &str, title: &str, status: &str, extra: &str, body: &str) -> (String, String) {
    let front = format!("type: ADR\ntitle: {title}\ndescription: d\nstatus: {status}\n{extra}");
    file(&format!("adr/{number}-x.md"), &front, body)
}

fn view(name: &str, title: &str, kind: &str) -> (String, String) {
    let front = format!("type: Architecture View\ntitle: {title}\ndescription: d\nkind: {kind}");
    file(&format!("architecture/{name}.md"), &front, "body")
}

fn page_of(files: &[(String, String)], mermaid: &MermaidSource) -> String {
    let store = MapStore {
        files: files
            .iter()
            .map(|(path, text)| (path.into(), text.clone()))
            .collect(),
    };
    render(&store, Path::new(BUNDLE), mermaid)
}

fn page(files: &[(String, String)]) -> String {
    page_of(files, &MermaidSource::Pinned)
}

fn rec_id(number: &str) -> String {
    format!("rec-adr-{number}-x-md")
}

fn at(html: &str, needle: &str) -> usize {
    html.find(needle)
        .unwrap_or_else(|| panic!("missing {needle:?} in:\n{html}"))
}

fn section<'a>(html: &'a str, id: &str) -> &'a str {
    let rest = &html[at(html, &format!("id=\"{id}\""))..];
    let end = rest[1..].find("<article").map_or(rest.len(), |at| at + 1);
    &rest[..end]
}

#[test]
fn rendering_twice_gives_the_same_page_without_time_or_the_bundle_path() {
    let files = [
        adr("0001", "Alpha accepted", "Accepted", "", "body"),
        adr("0002", "Bravo proposed", "Proposed", "", "body"),
    ];
    let first = page(&files);
    assert!(first.contains("Alpha accepted"), "{first}");
    assert_eq!(first, page(&files));
    assert!(!first.contains("secret-location"), "{first}");
    let has_date = regex::Regex::new(r"\d{4}-\d{2}-\d{2}|\d{2}:\d{2}:\d{2}").unwrap();
    assert!(!has_date.is_match(&first), "{first}");
}

#[test]
fn records_group_by_type_then_status_in_a_stable_order() {
    let issue = file(
        "issues/0001-x.md",
        "type: Issue\ntitle: Delta issue\ndescription: d\nstatus: open",
        "b",
    );
    let files = [
        issue,
        adr("0001", "Alpha accepted", "Accepted", "", "b"),
        adr("0002", "Bravo proposed", "Proposed", "", "b"),
    ];
    let html = page(&files);
    let order = [
        at(&html, "Bravo proposed"),
        at(&html, "Alpha accepted"),
        at(&html, "Delta issue"),
    ];
    assert!(order[0] < order[1] && order[1] < order[2], "{order:?}");
}

#[test]
fn a_superseded_record_links_its_successor_and_backlinks_list_both_linkers() {
    let linker = "see [t](/adr/0003-x.md)";
    let files = [
        adr(
            "0001",
            "Old rule",
            "Superseded",
            "superseded_by: \"0002\"",
            "b",
        ),
        adr("0002", "New rule", "Accepted", "supersedes: \"0001\"", "b"),
        adr("0003", "Linked target", "Accepted", "", "b"),
        adr("0004", "First linker", "Accepted", "", linker),
        adr("0005", "Second linker", "Accepted", "", linker),
    ];
    let html = page(&files);
    for title in ["Old rule", "New rule", "Linked target", "First linker"] {
        assert!(html.contains(title), "{title}");
    }
    let link_to = |n: &str| format!("href=\"#{}\"", rec_id(n));
    assert!(section(&html, &rec_id("0001")).contains(&link_to("0002")));
    let target = section(&html, &rec_id("0003"));
    assert!(target.contains(&link_to("0004")), "{target}");
    assert!(target.contains(&link_to("0005")), "{target}");
}

#[test]
fn architecture_views_follow_the_kind_order() {
    let files = [
        view("a", "View deployment", "deployment"),
        view("b", "View sequence", "sequence"),
        view("c", "View context", "context"),
        view("d", "View data", "data-model"),
        view("e", "View container", "container"),
    ];
    let html = page(&files);
    let titles = [
        "View context",
        "View container",
        "View sequence",
        "View data",
        "View deployment",
    ];
    let places: Vec<usize> = titles.iter().map(|t| at(&html, t)).collect();
    assert!(
        places.windows(2).all(|pair| pair[0] < pair[1]),
        "{places:?}"
    );
}

#[test]
fn raw_html_titles_and_diagrams_are_escaped() {
    let body = "<script>alert(1)</script>\n\n<img src=x onerror=alert(1)>\n\n```mermaid\ngraph TD; A-->B\n```";
    let files = [adr("0001", "A < B & C", "Accepted", "", body)];
    let html = page(&files);
    assert!(!html.contains("<script>alert(1)"), "{html}");
    assert!(!html.contains("<img src=x"), "{html}");
    assert!(html.contains("&lt;script&gt;alert(1)"), "{html}");
    assert!(html.contains("A &lt; B &amp; C"), "{html}");
    assert!(
        html.contains("<pre class=\"mermaid\">graph TD; A--&gt;B"),
        "{html}"
    );
}

#[test]
fn the_pinned_source_is_the_only_external_reference() {
    let html = page(&[adr("0001", "Alpha", "Accepted", "", "b")]);
    assert!(html.contains(PINNED_TAG), "{html}");
    assert_eq!(html.matches("://").count(), 1, "{html}");
    assert!(html.contains("<input"), "{html}");
    assert!(html.contains("addEventListener"), "{html}");
}

#[test]
fn an_inline_source_is_embedded_with_script_end_tags_neutralised() {
    let source = MermaidSource::Inline("var m=1;</script><b>x</b>".to_string());
    let html = page_of(&[adr("0001", "Alpha", "Accepted", "", "b")], &source);
    assert!(html.contains("var m=1;"), "{html}");
    assert!(!html.contains("</script><b>"), "{html}");
    assert_eq!(
        html.matches("<script").count(),
        html.matches("</script").count(),
        "{html}"
    );
    assert!(!html.contains("://"), "{html}");
}

#[test]
fn inline_raw_html_in_a_paragraph_is_shown_as_text() {
    let body = "text <img src=x onerror=alert(1)> text";
    let html = page(&[adr("0001", "Alpha", "Accepted", "", body)]);
    assert!(!html.contains("<img"), "{html}");
    assert!(
        html.contains("text &lt;img src=x onerror=alert(1)&gt; text"),
        "{html}"
    );
}

#[test]
fn only_web_mail_fragment_and_relative_links_keep_their_destination() {
    let body = "[a](javascript:alert(1)) [b](JaVaScRiPt:alert(1)) [c](data:text/html,x) [d](vbscript:x) [e](https://example.org/x)";
    let html = page(&[adr("0001", "Alpha", "Accepted", "", body)]);
    assert_eq!(html.matches("href=\"#\"").count(), 4, "{html}");
    assert!(html.contains("href=\"https://example.org/x\""), "{html}");
    assert!(!html.to_lowercase().contains("script:"), "{html}");
}

#[test]
fn an_image_is_reduced_to_its_alt_text() {
    let body = "![shown alt](https://example.org/a.png)";
    let html = page(&[adr("0001", "Alpha", "Accepted", "", body)]);
    assert!(!html.contains("<img"), "{html}");
    assert!(!html.contains("a.png"), "{html}");
    assert!(html.contains("shown alt"), "{html}");
}

#[test]
fn a_description_and_a_status_are_escaped_where_they_are_shown() {
    let front = "type: ADR\ntitle: T\ndescription: \"a < b & c\"\nstatus: \"x <i> &\"";
    let html = page(&[file("adr/0001-x.md", front, "b")]);
    assert!(html.contains("<p>a &lt; b &amp; c</p>"), "{html}");
    assert!(html.contains("<h3>x &lt;i&gt; &amp;</h3>"), "{html}");
    assert!(!html.contains("<i>"), "{html}");
}

#[test]
fn statuses_differing_only_in_case_share_one_heading() {
    let files = [
        adr("0001", "Alpha", "Accepted", "", "b"),
        adr("0002", "Bravo", "accepted", "", "b"),
    ];
    let html = page(&files);
    assert_eq!(html.matches("<h3>").count(), 1, "{html}");
}

#[test]
fn body_headings_never_outrank_the_card_title() {
    let html = page(&[adr("0001", "Alpha", "Accepted", "", "# Big\n\n## Mid")]);
    assert!(!html.contains("<h1>Big"), "{html}");
    assert!(!html.contains("<h2>Mid"), "{html}");
    assert!(
        !html.contains("<h3>Big") && !html.contains("<h4>Big"),
        "{html}"
    );
    assert!(html.contains("<h5>Big</h5>"), "{html}");
    assert!(html.contains("<h6>Mid</h6>"), "{html}");
    assert_eq!(html.matches("<h4>").count(), 1, "{html}");
}

#[test]
fn a_successor_declaring_only_supersedes_is_linked_by_its_predecessor() {
    let files = [
        adr("0001", "Old rule", "Superseded", "", "b"),
        adr("0002", "New rule", "Accepted", "supersedes: \"0001\"", "b"),
    ];
    let html = page(&files);
    let old = section(&html, &rec_id("0001"));
    assert!(
        old.contains(&format!("href=\"#{}\"", rec_id("0002"))),
        "{old}"
    );
}

#[test]
fn an_inline_source_cannot_end_or_double_escape_its_script_element() {
    let text = "x;<!--<script>y;</ScRiPt>z;<SCRIPT";
    let html = page_of(
        &[adr("0001", "Alpha", "Accepted", "", "b")],
        &MermaidSource::Inline(text.to_string()),
    );
    let start = at(&html, "<script>x;");
    let end = start + at(&html[start..], "</script>");
    let inlined = html[start + "<script>".len()..end].to_lowercase();
    for raw in ["</script", "<!--", "<script"] {
        assert!(!inlined.contains(raw), "{raw}: {inlined}");
    }
    assert!(inlined.starts_with("x;") && inlined.contains("y;") && inlined.contains("z;"));
    assert!(!html.contains("<!--"), "{html}");
    assert!(inlined.contains("<\\x21--"), "{inlined}");
    assert!(!inlined.contains("<\\!--"), "{inlined}");
    assert!(html[end..].ends_with("</body>\n</html>\n"), "{html}");
}
