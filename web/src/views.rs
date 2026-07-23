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

pub(crate) fn record_href(path: &str) -> String {
    format!("/record/{path}")
}

/// A record page's supersede confirm form render state (ADR 0016, issue
/// 0012): `href` to post the confirmation to, `value` pre-filling the
/// superseding record's number input (the caller's last submission on a
/// rejected commit, empty for a fresh page load), and `error`, when `Some`,
/// rendered above the form.
pub struct SupersedeFormState<'a> {
    pub href: &'a str,
    pub value: &'a str,
    pub error: Option<&'a str>,
}

/// Renders a single record's center-pane content: the back link, a
/// discoverable "Edit" link when `edit_href` is `Some` (db-mode only — ADR
/// 0016, issue 0011), the supersede confirm form when `supersede` is `Some`
/// (db-mode only — ADR 0016, issue 0012), the delete confirm form when
/// `delete` is `Some` (db-mode only, and only for a not-yet-deleted record —
/// ADR 0018, issue 0013 slice B), and `body_html`, already rendered from
/// markdown by the caller and injected verbatim. The body's own leading
/// `# Title` markdown heading is the page's sole `h1` (issue 0008, ADR
/// 0015, S3) — this view renders no h1 of its own, so a record page never
/// carries two.
pub fn record_page(
    body_html: &str,
    edit_href: Option<&str>,
    supersede: Option<SupersedeFormState<'_>>,
    delete: Option<DeleteFormState<'_>>,
) -> Markup {
    html! {
        a href="/" { "← Back to search" }
        @if let Some(edit_href) = edit_href {
            a href=(edit_href) class="edit-link" { "Edit" }
        }
        @if let Some(supersede) = supersede {
            (supersede_form(&supersede))
        }
        @if let Some(delete) = delete {
            (delete_form(&delete))
        }
        (PreEscaped(body_html))
    }
}

/// A record page's delete confirm form render state (ADR 0018, issue 0013
/// slice B): `href` to post the confirmation to, and `error`, when `Some`,
/// rendered above the form — the delete form submits no fields of its own,
/// unlike the supersede form's `new` input.
pub struct DeleteFormState<'a> {
    pub href: &'a str,
    pub error: Option<&'a str>,
}

/// The delete confirm form itself: no input fields, a "Delete" submit
/// button, and an error slot above the button when `state.error` is `Some` —
/// [`record_page`]'s own extracted piece, mirroring [`supersede_form`]'s
/// shape.
fn delete_form(state: &DeleteFormState<'_>) -> Markup {
    html! {
        form action=(state.href) method="post" class="delete-form" {
            @if let Some(error) = state.error {
                p class="form-error" { (error) }
            }
            button type="submit" { "Delete" }
        }
    }
}

/// The supersede confirm form itself: a bare-number text input named `new`,
/// a "Supersede" submit button, and an error slot above the input when
/// `state.error` is `Some` — [`record_page`]'s own extracted piece, kept
/// separate so that function stays a flat sequence of the page's parts.
fn supersede_form(state: &SupersedeFormState<'_>) -> Markup {
    html! {
        form action=(state.href) method="post" class="supersede-form" {
            @if let Some(error) = state.error {
                p class="form-error" { (error) }
            }
            input
                type="text"
                name="new"
                value=(state.value)
                placeholder="Superseding record number";
            button type="submit" { "Supersede" }
        }
    }
}

/// Renders the right-pane metadata panel for a record (issue 0008, ADR
/// 0015, S3): its doc type, a badge — the "Deleted" badge when
/// `meta.deleted_at` is set (ADR 0018, issue 0013 slice B), otherwise the
/// status badge when `meta.status` is `Some`; only one of the two is ever
/// shown — its supersede chain in both directions as links to
/// `/record/<path>` (each section omitted when its list is empty), and its
/// tags as chips (omitted when empty).
pub fn metadata_panel(meta: &RecordMeta) -> Markup {
    html! {
        section class="meta-panel" {
            div class="meta-row" {
                span class="meta-label" { "Type" }
                span class="meta-value" { (meta.doc_type) }
            }
            @if meta.deleted_at.is_some() {
                (deleted_badge())
            } @else if let Some(status) = &meta.status {
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

/// The badge shown once a record's `deleted_at` is set (ADR 0018, issue
/// 0013 slice B) — mirrors [`status_badge`]'s shape but with a fixed
/// modifier and label, since a soft-deleted record's own former status is no
/// longer meaningful to show.
fn deleted_badge() -> Markup {
    html! {
        span class="status-badge status-deleted" { "Deleted" }
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

/// The doc types Atlas's create form offers — the same four
/// `living_docs_core::paths::dir_for` accepts (ADR 0016, issue 0010 slice
/// 3): `adr`, `bdr`, `prd`, `issue`.
const CREATABLE_DOC_TYPES: [&str; 4] = ["adr", "bdr", "prd", "issue"];

/// Renders `POST /new`'s create form: a doc-type select, a title input, and
/// a submit button, in the same plain (label-less) input style
/// `search_form`/`project_filter` already use. `doc_type`/`title` preserve
/// the caller's last submission across a failed create (re-render after a
/// rejected write); `error`, when `Some`, renders its message above the
/// form.
pub fn create_form(doc_type: Option<&str>, title: Option<&str>, error: Option<&str>) -> Markup {
    html! {
        h1 { "New record" }
        @if let Some(error) = error {
            p class="form-error" { (error) }
        }
        form action="/new" method="post" {
            (doc_type_select(doc_type))
            input type="text" name="title" value=(title.unwrap_or_default()) placeholder="Title";
            button type="submit" { "Create" }
        }
    }
}

fn doc_type_select(selected: Option<&str>) -> Markup {
    html! {
        select name="doc_type" {
            @for option in CREATABLE_DOC_TYPES {
                option value=(option) selected[selected == Some(option)] { (option) }
            }
        }
    }
}

/// Renders `POST /edit/{*path}`'s edit form (ADR 0016, issue 0011): a
/// content textarea pre-filled with `content`, a hidden `base_revision`
/// field carrying the optimistic-concurrency precondition, and a submit
/// button. `path`/`content`/`base_revision` preserve the caller's last
/// submission across a failed edit (a failing `check`) or a stale-revision
/// reload — for a stale rejection the caller passes the CURRENT server
/// content and revision, never the rejected submission (ADR 0016: reject,
/// never merge). `error`, when `Some`, renders its message above the form.
pub fn edit_form(path: &str, content: &str, base_revision: i64, error: Option<&str>) -> Markup {
    html! {
        h1 { "Edit record" }
        @if let Some(error) = error {
            p class="form-error" { (error) }
        }
        form action=(format!("/edit/{path}")) method="post" {
            textarea name="content" { (content) }
            input type="hidden" name="base_revision" value=(base_revision);
            button type="submit" { "Save" }
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
        let rendered = record_page(
            "<h1>Quokka Caching Strategy</h1>\n<p>Body.</p>\n",
            None,
            None,
            None,
        )
        .into_string();

        assert!(rendered.contains("<h1>Quokka Caching Strategy</h1>"));
        assert!(rendered.contains("<p>Body.</p>"));
        assert_eq!(rendered.matches("<h1").count(), 1);
        assert!(rendered.contains("← Back to search"));
    }

    #[test]
    fn record_page_omits_the_edit_link_when_edit_href_is_none() {
        let rendered = record_page("<p>Body.</p>\n", None, None, None).into_string();

        assert!(!rendered.contains("class=\"edit-link\""));
    }

    #[test]
    fn record_page_renders_the_edit_link_when_edit_href_is_some() {
        let rendered =
            record_page("<p>Body.</p>\n", Some("/edit/adr/0001-a.md"), None, None).into_string();

        assert!(rendered.contains("href=\"/edit/adr/0001-a.md\" class=\"edit-link\""));
        assert!(rendered.contains(">Edit<"));
    }

    #[test]
    fn record_page_omits_the_supersede_form_when_supersede_is_none() {
        let rendered = record_page("<p>Body.</p>\n", None, None, None).into_string();

        assert!(!rendered.contains("class=\"supersede-form\""));
    }

    #[test]
    fn record_page_renders_the_supersede_form_when_supersede_is_some() {
        let supersede = SupersedeFormState {
            href: "/supersede/adr/0001-a.md",
            value: "",
            error: None,
        };

        let rendered = record_page("<p>Body.</p>\n", None, Some(supersede), None).into_string();

        assert!(rendered.contains("<form action=\"/supersede/adr/0001-a.md\" method=\"post\""));
        assert!(rendered.contains("name=\"new\""));
        assert!(rendered.contains(">Supersede<"));
        assert!(!rendered.contains("form-error"));
    }

    #[test]
    fn record_page_supersede_form_preserves_the_submitted_value_and_shows_the_error() {
        let supersede = SupersedeFormState {
            href: "/supersede/adr/0001-a.md",
            value: "0099",
            error: Some("no record found for 0099"),
        };

        let rendered = record_page("<p>Body.</p>\n", None, Some(supersede), None).into_string();

        assert!(rendered.contains("class=\"form-error\""));
        assert!(rendered.contains("no record found for 0099"));
        assert!(rendered.contains("value=\"0099\""));
    }

    #[test]
    fn record_page_omits_the_delete_form_when_delete_is_none() {
        let rendered = record_page("<p>Body.</p>\n", None, None, None).into_string();

        assert!(!rendered.contains("class=\"delete-form\""));
    }

    #[test]
    fn record_page_renders_the_delete_form_when_delete_is_some() {
        let delete = DeleteFormState {
            href: "/delete/issue/0001-a.md",
            error: None,
        };

        let rendered = record_page("<p>Body.</p>\n", None, None, Some(delete)).into_string();

        assert!(rendered.contains("<form action=\"/delete/issue/0001-a.md\" method=\"post\""));
        assert!(rendered.contains(">Delete<"));
        assert!(!rendered.contains("form-error"));
    }

    #[test]
    fn record_page_delete_form_shows_the_error_when_present() {
        let delete = DeleteFormState {
            href: "/delete/adr/0001-a.md",
            error: Some("adr/0001-a.md: doc type 'ADR' is not eligible for delete"),
        };

        let rendered = record_page("<p>Body.</p>\n", None, None, Some(delete)).into_string();

        assert!(rendered.contains("class=\"form-error\""));
        assert!(rendered.contains("is not eligible for delete"));
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
            revision: 1,
            deleted_at: None,
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

    fn deleted(mut meta: RecordMeta) -> RecordMeta {
        meta.deleted_at = Some(1_700_000_000);
        meta
    }

    #[test]
    fn metadata_panel_renders_the_deleted_badge_when_deleted_at_is_set() {
        let meta = deleted(record_meta("Issue", None, vec![], vec![], vec![]));

        let rendered = metadata_panel(&meta).into_string();

        assert!(rendered.contains("class=\"status-badge status-deleted\""));
        assert!(rendered.contains(">Deleted<"));
    }

    #[test]
    fn metadata_panel_shows_the_deleted_badge_instead_of_the_status_badge_when_both_apply() {
        let meta = deleted(record_meta(
            "Issue",
            Some("Accepted"),
            vec![],
            vec![],
            vec![],
        ));

        let rendered = metadata_panel(&meta).into_string();

        assert!(rendered.contains(">Deleted<"));
        assert!(!rendered.contains("status-accepted"));
        assert!(!rendered.contains(">Accepted<"));
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
    fn create_form_without_prefill_renders_an_empty_form_and_no_error() {
        let rendered = create_form(None, None, None).into_string();

        assert!(rendered.contains("<form action=\"/new\" method=\"post\""));
        assert!(rendered.contains("name=\"doc_type\""));
        assert!(rendered.contains("name=\"title\""));
        assert!(!rendered.contains("form-error"));
    }

    #[test]
    fn create_form_lists_every_creatable_doc_type_option() {
        let rendered = create_form(None, None, None).into_string();

        for doc_type in ["adr", "bdr", "prd", "issue"] {
            assert!(
                rendered.contains(&format!("value=\"{doc_type}\"")),
                "missing option for {doc_type}: {rendered}"
            );
        }
    }

    #[test]
    fn create_form_preserves_the_submitted_doc_type_and_title_on_re_render() {
        let rendered = create_form(Some("bdr"), Some("Draft Behavior"), None).into_string();

        assert!(rendered.contains("<option value=\"bdr\" selected>bdr</option>"));
        assert!(rendered.contains("value=\"Draft Behavior\""));
    }

    #[test]
    fn create_form_renders_the_error_message_above_the_form() {
        let rendered =
            create_form(Some("adr"), Some("Broken"), Some("check failed: orphan")).into_string();

        assert!(rendered.contains("class=\"form-error\""));
        assert!(rendered.contains("check failed: orphan"));
    }

    #[test]
    fn create_form_escapes_a_title_containing_markup() {
        let rendered = create_form(None, Some("<script>alert(1)</script>"), None).into_string();

        assert!(!rendered.contains("<script>"));
        assert!(rendered.contains("&lt;script&gt;"));
    }

    #[test]
    fn edit_form_posts_to_the_record_path_and_prefills_content_and_base_revision() {
        let rendered = edit_form("adr/0001-a.md", "Body text.", 3, None).into_string();

        assert!(rendered.contains("<form action=\"/edit/adr/0001-a.md\" method=\"post\""));
        assert!(rendered.contains("<textarea name=\"content\">Body text.</textarea>"));
        assert!(rendered.contains("name=\"base_revision\" value=\"3\""));
        assert!(!rendered.contains("form-error"));
    }

    #[test]
    fn edit_form_renders_the_error_message_above_the_form() {
        let rendered = edit_form(
            "adr/0001-a.md",
            "Body text.",
            3,
            Some("check failed: orphan"),
        )
        .into_string();

        assert!(rendered.contains("class=\"form-error\""));
        assert!(rendered.contains("check failed: orphan"));
    }

    #[test]
    fn edit_form_escapes_content_containing_markup() {
        let rendered =
            edit_form("adr/0001-a.md", "<script>alert(1)</script>", 1, None).into_string();

        assert!(!rendered.contains("<script>"));
        assert!(rendered.contains("&lt;script&gt;"));
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
