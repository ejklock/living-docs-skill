//! maud rendering for the read-only search and record pages (ADR 0006, issue
//! 0003 slices S3a-S3b; project filter ADR 0005 issue 0005 slice 0005-C2),
//! wrapped in the three-pane doc-site shell (ADR 0015, issue 0008, S2) with
//! its right-pane metadata panel (ADR 0015, issue 0008, S3) and the Cmd+K
//! palette's hidden overlay container and result fragment (ADR 0015, issue
//! 0008, S4). Every value reflected from user input is rendered through
//! maud's auto-escaping `Markup`, never string-concatenated into HTML. The
//! record body is the sole exception: it is pre-rendered from the local
//! corpus's markdown source (not user input) via pulldown-cmark, then
//! injected as-is through `PreEscaped`.

use db_store::{NavEntry, ProjectView, RecordMeta, RelatedRef, SearchHit};
use maud::{html, Markup, PreEscaped};

/// Renders the full three-pane document: `<head>` with the served
/// stylesheet and the deferred palette script, the left nav tree grouped by
/// `doc_type` (built from `nav`), `main` as the center pane, `aside` as the
/// right pane — an empty placeholder when `None`, filled by slice S3 — and
/// the hidden Cmd+K palette overlay (slice S4). The nav entry whose `path`
/// equals `active_path` carries `aria-current="page"`.
pub fn shell(
    page_title: &str,
    nav: &[NavEntry],
    active_path: Option<&str>,
    main: Markup,
    aside: Option<Markup>,
) -> Markup {
    html! {
        html {
            head {
                meta charset="utf-8";
                title { (page_title) }
                link rel="stylesheet" href="/style.css";
                script src="/palette.js" defer {}
            }
            body {
                nav { (nav_tree(nav, active_path)) }
                main { (main) }
                aside { @if let Some(aside) = aside { (aside) } }
                (palette_overlay())
            }
        }
    }
}

fn palette_overlay() -> Markup {
    html! {
        div id="palette-overlay" hidden {
            div class="palette-dialog" {
                input
                    id="palette-input"
                    type="search"
                    placeholder="Search docs…"
                    aria-label="Search docs";
                div id="palette-results" {}
            }
        }
    }
}

fn nav_tree(nav: &[NavEntry], active_path: Option<&str>) -> Markup {
    html! {
        @for (doc_type, entries) in nav_groups(nav) {
            section class="nav-group" {
                h2 { (doc_type) }
                ul {
                    @for entry in entries {
                        li { (nav_link(entry, active_path)) }
                    }
                }
            }
        }
    }
}

fn nav_groups(nav: &[NavEntry]) -> Vec<(&str, Vec<&NavEntry>)> {
    let mut groups: Vec<(&str, Vec<&NavEntry>)> = Vec::new();
    for entry in nav {
        match groups.last_mut() {
            Some((doc_type, entries)) if *doc_type == entry.doc_type => entries.push(entry),
            _ => groups.push((entry.doc_type.as_str(), vec![entry])),
        }
    }
    groups
}

fn nav_link(entry: &NavEntry, active_path: Option<&str>) -> Markup {
    let is_active = active_path == Some(entry.path.as_str());
    html! {
        a href=(record_href(&entry.path)) aria-current=[is_active.then_some("page")] {
            (entry.title)
        }
    }
}

/// Renders the search page's center-pane content: the search form (with its
/// project filter) plus, when a query was submitted, either the ranked
/// results or an explicit empty-state message. `selected_project` is the
/// slug currently narrowing the search, preserved in the form on re-render.
pub fn search_page(
    query: Option<&str>,
    selected_project: Option<&str>,
    projects: &[ProjectView],
    hits: &[SearchHit],
) -> Markup {
    html! {
        h1 { "living-docs search" }
        (search_form(query, selected_project, projects))
        @if let Some(query) = query {
            (search_results(hits, query))
        }
    }
}

fn search_form(
    query: Option<&str>,
    selected_project: Option<&str>,
    projects: &[ProjectView],
) -> Markup {
    html! {
        form action="/" method="get" {
            input type="search" name="q" value=(query.unwrap_or_default()) placeholder="Search docs…";
            (project_filter(selected_project, projects))
            button type="submit" { "Search" }
        }
    }
}

fn project_filter(selected_project: Option<&str>, projects: &[ProjectView]) -> Markup {
    html! {
        select name="project" {
            option value="" selected[selected_project.is_none()] { "All projects" }
            @for project in projects {
                option
                    value=(project.slug)
                    selected[selected_project == Some(project.slug.as_str())] {
                    (project.name)
                }
            }
        }
    }
}

fn search_results(hits: &[SearchHit], query: &str) -> Markup {
    html! {
        @if hits.is_empty() {
            p class="empty-state" { "No results for \"" (query) "\"." }
        } @else {
            (result_list(hits))
        }
    }
}

/// Renders the Cmd+K palette's result fragment (issue 0008, ADR 0015, S4):
/// a bare `<ul class="results">` of the same result items the search page
/// renders — reused via `result_list` so the two views share one item
/// renderer — or an empty-state `<p>` when `hits` is empty. Returned as-is
/// by `GET /palette`, with no surrounding shell.
pub fn palette_fragment(hits: &[SearchHit]) -> Markup {
    html! {
        @if hits.is_empty() {
            p class="empty-state" { "No results." }
        } @else {
            (result_list(hits))
        }
    }
}

fn result_list(hits: &[SearchHit]) -> Markup {
    html! {
        ul class="results" {
            @for hit in hits {
                (result_item(hit))
            }
        }
    }
}

fn result_item(hit: &SearchHit) -> Markup {
    html! {
        li {
            a href=(record_href(&hit.path)) { (hit.title) }
            span class="project-label" { (hit.project) }
            p { (hit.snippet) }
        }
    }
}

fn record_href(path: &str) -> String {
    format!("/record/{path}")
}

/// Renders a single record's center-pane content: the back link plus
/// `body_html`, already rendered from markdown by the caller and injected
/// verbatim. The body's own leading `# Title` markdown heading is the
/// page's sole `h1` (issue 0008, ADR 0015, S3) — this view renders no h1 of
/// its own, so a record page never carries two.
pub fn record_page(body_html: &str) -> Markup {
    html! {
        a href="/" { "← Back to search" }
        (PreEscaped(body_html))
    }
}

/// Renders the right-pane metadata panel for a record (issue 0008, ADR
/// 0015, S3): its doc type, an optional status badge, its supersede chain
/// in both directions as links to `/record/<path>` (each section omitted
/// when its list is empty), and its tags as chips (omitted when empty).
pub fn metadata_panel(meta: &RecordMeta) -> Markup {
    html! {
        section class="meta-panel" {
            div class="meta-row" {
                span class="meta-label" { "Type" }
                span class="meta-value" { (meta.doc_type) }
            }
            @if let Some(status) = &meta.status {
                (status_badge(status))
            }
            @if !meta.supersedes.is_empty() {
                (related_section("Supersedes", &meta.supersedes))
            }
            @if !meta.superseded_by.is_empty() {
                (related_section("Superseded by", &meta.superseded_by))
            }
            @if !meta.tags.is_empty() {
                (tags_section(&meta.tags))
            }
        }
    }
}

fn status_badge(status: &str) -> Markup {
    let modifier = status_modifier(status);
    html! {
        span class=(format!("status-badge status-{modifier}")) { (status) }
    }
}

fn status_modifier(status: &str) -> String {
    status.to_lowercase().replace(' ', "-")
}

fn related_section(heading: &str, refs: &[RelatedRef]) -> Markup {
    html! {
        section class="meta-related" {
            h2 { (heading) }
            ul {
                @for related in refs {
                    li { a href=(record_href(&related.path)) { (related.title) } }
                }
            }
        }
    }
}

fn tags_section(tags: &[String]) -> Markup {
    html! {
        section class="meta-tags" {
            h2 { "Tags" }
            @for tag in tags {
                span class="tag" { (tag) }
            }
        }
    }
}

/// Renders the center-pane content shown when `GET /record/{*path}` finds
/// no matching record.
pub fn not_found() -> Markup {
    html! {
        h1 { "Record not found" }
        p { "No record exists at this path." }
        a href="/" { "← Back to search" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(path: &str, title: &str, snippet: &str, project: &str) -> SearchHit {
        SearchHit {
            path: path.to_owned(),
            title: title.to_owned(),
            snippet: snippet.to_owned(),
            project: project.to_owned(),
        }
    }

    fn project(slug: &str, name: &str) -> ProjectView {
        ProjectView {
            slug: slug.to_owned(),
            name: name.to_owned(),
        }
    }

    #[test]
    fn search_page_without_a_query_renders_only_the_form() {
        let markup = search_page(None, None, &[], &[]);

        let rendered = markup.into_string();
        assert!(rendered.contains("<form"));
        assert!(!rendered.contains("empty-state"));
        assert!(!rendered.contains("results"));
    }

    #[test]
    fn search_page_with_hits_links_to_the_record_path_and_escapes_the_query() {
        let hits = vec![hit(
            "adr/0001-quokka-caching.md",
            "Quokka Caching Strategy",
            "an aggressive [quokka] caching strategy",
            "docs",
        )];

        let rendered = search_page(Some("<script>"), None, &[], &hits).into_string();

        assert!(rendered.contains("href=\"/record/adr/0001-quokka-caching.md\""));
        assert!(rendered.contains("Quokka Caching Strategy"));
        assert!(!rendered.contains("<script>"));
        assert!(rendered.contains("&lt;script&gt;"));
    }

    #[test]
    fn search_page_with_no_hits_renders_the_empty_state_message() {
        let rendered = search_page(Some("zzzznomatch"), None, &[], &[]).into_string();

        assert!(rendered.contains("empty-state"));
        assert!(rendered.contains("No results for &quot;zzzznomatch&quot;."));
    }

    #[test]
    fn search_page_renders_a_project_filter_listing_every_project_plus_an_all_option() {
        let projects = vec![project("team-a", "Team A"), project("team-b", "Team B")];

        let rendered = search_page(None, None, &projects, &[]).into_string();

        assert!(rendered.contains("<select name=\"project\""));
        assert!(rendered.contains("value=\"\""));
        assert!(rendered.contains("All projects"));
        assert!(rendered.contains("value=\"team-a\""));
        assert!(rendered.contains("Team A"));
        assert!(rendered.contains("value=\"team-b\""));
        assert!(rendered.contains("Team B"));
    }

    #[test]
    fn search_page_preserves_the_selected_project_as_the_marked_option() {
        let projects = vec![project("team-a", "Team A"), project("team-b", "Team B")];

        let rendered = search_page(None, Some("team-b"), &projects, &[]).into_string();

        assert!(rendered.contains("<option value=\"team-b\" selected>Team B</option>"));
        assert!(rendered.contains("<option value=\"team-a\">Team A</option>"));
    }

    #[test]
    fn search_page_with_no_project_selected_marks_all_projects_selected() {
        let projects = vec![project("team-a", "Team A")];

        let rendered = search_page(None, None, &projects, &[]).into_string();

        assert!(rendered.contains("<option value=\"\" selected>All projects</option>"));
        assert!(rendered.contains("<option value=\"team-a\">Team A</option>"));
    }

    #[test]
    fn search_page_labels_each_hit_with_its_project() {
        let hits = vec![
            hit("adr/0001-a.md", "Title A", "snippet a", "team-a"),
            hit("adr/0002-b.md", "Title B", "snippet b", "team-b"),
        ];

        let rendered = search_page(Some("caching"), None, &[], &hits).into_string();

        assert!(rendered.contains("<span class=\"project-label\">team-a</span>"));
        assert!(rendered.contains("<span class=\"project-label\">team-b</span>"));
    }

    #[test]
    fn record_page_injects_the_preescaped_body_and_carries_no_view_level_h1() {
        let rendered =
            record_page("<h1>Quokka Caching Strategy</h1>\n<p>Body.</p>\n").into_string();

        assert!(rendered.contains("<h1>Quokka Caching Strategy</h1>"));
        assert!(rendered.contains("<p>Body.</p>"));
        assert_eq!(rendered.matches("<h1").count(), 1);
        assert!(rendered.contains("← Back to search"));
    }

    fn related_ref(path: &str, title: &str) -> RelatedRef {
        RelatedRef {
            path: path.to_owned(),
            title: title.to_owned(),
        }
    }

    fn record_meta(
        doc_type: &str,
        status: Option<&str>,
        supersedes: Vec<RelatedRef>,
        superseded_by: Vec<RelatedRef>,
        tags: Vec<&str>,
    ) -> RecordMeta {
        RecordMeta {
            doc_type: doc_type.to_owned(),
            status: status.map(str::to_owned),
            supersedes,
            superseded_by,
            tags: tags.into_iter().map(str::to_owned).collect(),
        }
    }

    #[test]
    fn metadata_panel_renders_the_doc_type_row() {
        let meta = record_meta("ADR", None, vec![], vec![], vec![]);

        let rendered = metadata_panel(&meta).into_string();

        assert!(rendered.contains("Type"));
        assert!(rendered.contains("ADR"));
    }

    #[test]
    fn metadata_panel_renders_a_lowercased_status_badge_modifier_class() {
        let meta = record_meta("ADR", Some("Accepted"), vec![], vec![], vec![]);

        let rendered = metadata_panel(&meta).into_string();

        assert!(rendered.contains("class=\"status-badge status-accepted\""));
        assert!(rendered.contains(">Accepted<"));
    }

    #[test]
    fn metadata_panel_omits_the_status_badge_when_status_is_none() {
        let meta = record_meta("ADR", None, vec![], vec![], vec![]);

        let rendered = metadata_panel(&meta).into_string();

        assert!(!rendered.contains("status-badge"));
    }

    #[test]
    fn metadata_panel_links_each_supersede_direction_to_its_record_path() {
        let meta = record_meta(
            "ADR",
            None,
            vec![related_ref("adr/0001-a.md", "Decision A")],
            vec![related_ref("adr/0003-c.md", "Decision C")],
            vec![],
        );

        let rendered = metadata_panel(&meta).into_string();

        assert!(rendered.contains("Supersedes"));
        assert!(rendered.contains("href=\"/record/adr/0001-a.md\""));
        assert!(rendered.contains("Decision A"));
        assert!(rendered.contains("Superseded by"));
        assert!(rendered.contains("href=\"/record/adr/0003-c.md\""));
        assert!(rendered.contains("Decision C"));
    }

    #[test]
    fn metadata_panel_omits_supersede_sections_when_their_lists_are_empty() {
        let meta = record_meta("ADR", None, vec![], vec![], vec![]);

        let rendered = metadata_panel(&meta).into_string();

        assert!(!rendered.contains("Supersedes"));
        assert!(!rendered.contains("Superseded by"));
    }

    #[test]
    fn metadata_panel_renders_a_tag_chip_per_tag() {
        let meta = record_meta("ADR", None, vec![], vec![], vec!["web", "ux"]);

        let rendered = metadata_panel(&meta).into_string();

        assert!(rendered.contains("<span class=\"tag\">web</span>"));
        assert!(rendered.contains("<span class=\"tag\">ux</span>"));
    }

    #[test]
    fn metadata_panel_omits_the_tags_section_when_tags_are_empty() {
        let meta = record_meta("ADR", None, vec![], vec![], vec![]);

        let rendered = metadata_panel(&meta).into_string();

        assert!(!rendered.contains("class=\"tag\""));
    }

    #[test]
    fn not_found_renders_the_not_found_copy() {
        let rendered = not_found().into_string();

        assert!(rendered.to_lowercase().contains("not found"));
    }

    fn nav_entry(doc_type: &str, path: &str, title: &str) -> NavEntry {
        NavEntry {
            doc_type: doc_type.to_owned(),
            number: None,
            title: title.to_owned(),
            path: path.to_owned(),
        }
    }

    #[test]
    fn shell_groups_nav_entries_by_doc_type_and_links_to_each_record() {
        let nav = vec![
            nav_entry("ADR", "adr/0001-a.md", "Decision A"),
            nav_entry("ADR", "adr/0002-b.md", "Decision B"),
            nav_entry("Runbook", "runbook/0001-r.md", "Runbook R"),
        ];

        let rendered =
            shell("living-docs", &nav, None, html! { p { "content" } }, None).into_string();

        assert!(rendered.contains("<nav"));
        assert_eq!(rendered.matches("<h2>ADR</h2>").count(), 1);
        assert!(rendered.contains("<h2>Runbook</h2>"));
        assert!(rendered.contains("href=\"/record/adr/0001-a.md\""));
        assert!(rendered.contains("Decision A"));
        assert!(rendered.contains("href=\"/record/runbook/0001-r.md\""));
    }

    #[test]
    fn shell_marks_the_active_path_entry_with_aria_current_page() {
        let nav = vec![
            nav_entry("ADR", "adr/0001-a.md", "Decision A"),
            nav_entry("ADR", "adr/0002-b.md", "Decision B"),
        ];

        let rendered = shell(
            "living-docs",
            &nav,
            Some("adr/0002-b.md"),
            html! { p { "content" } },
            None,
        )
        .into_string();

        assert!(rendered.contains("href=\"/record/adr/0002-b.md\" aria-current=\"page\""));
        assert!(!rendered.contains("href=\"/record/adr/0001-a.md\" aria-current=\"page\""));
    }

    #[test]
    fn shell_renders_an_empty_aside_placeholder_when_none() {
        let rendered =
            shell("living-docs", &[], None, html! { p { "content" } }, None).into_string();

        assert!(rendered.contains("<aside></aside>"));
    }

    #[test]
    fn shell_renders_the_provided_aside_content_when_present() {
        let rendered = shell(
            "living-docs",
            &[],
            None,
            html! { p { "content" } },
            Some(html! { p { "meta" } }),
        )
        .into_string();

        assert!(rendered.contains("<aside><p>meta</p></aside>"));
    }

    #[test]
    fn shell_links_the_stylesheet_and_sets_the_page_title() {
        let rendered = shell("living-docs search", &[], None, html! { p {} }, None).into_string();

        assert!(rendered.contains("<link rel=\"stylesheet\" href=\"/style.css\">"));
        assert!(rendered.contains("<title>living-docs search</title>"));
    }

    #[test]
    fn shell_loads_the_deferred_palette_script_and_renders_the_hidden_overlay() {
        let rendered = shell("living-docs search", &[], None, html! { p {} }, None).into_string();

        assert!(rendered.contains("<script src=\"/palette.js\" defer></script>"));
        assert!(rendered.contains("id=\"palette-overlay\" hidden"));
        assert!(rendered.contains("id=\"palette-input\""));
        assert!(rendered.contains("id=\"palette-results\""));
    }

    #[test]
    fn palette_fragment_with_hits_links_to_the_record_path_and_carries_no_shell_nav() {
        let hits = vec![hit(
            "adr/0001-quokka-caching.md",
            "Quokka Caching Strategy",
            "an aggressive [quokka] caching strategy",
            "docs",
        )];

        let rendered = palette_fragment(&hits).into_string();

        assert!(rendered.contains("href=\"/record/adr/0001-quokka-caching.md\""));
        assert!(rendered.contains("Quokka Caching Strategy"));
        assert!(!rendered.contains("<nav"));
        assert!(!rendered.contains("<html"));
    }

    #[test]
    fn palette_fragment_with_no_hits_renders_the_empty_state() {
        let rendered = palette_fragment(&[]).into_string();

        assert!(rendered.contains("empty-state"));
        assert!(rendered.contains("No results."));
    }
}
