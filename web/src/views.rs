//! maud rendering for the read-only search and record pages (ADR 0006, issue
//! 0003 slices S3a-S3b). Every value reflected from user input is rendered
//! through maud's auto-escaping `Markup`, never string-concatenated into
//! HTML. The record body is the sole exception: it is pre-rendered from the
//! local corpus's markdown source (not user input) via pulldown-cmark, then
//! injected as-is through `PreEscaped`.

use db_store::SearchHit;
use maud::{html, Markup, PreEscaped};

/// Renders the full search page: the search form plus, when a query was
/// submitted, either the ranked results or an explicit empty-state message.
pub fn search_page(query: Option<&str>, hits: &[SearchHit]) -> Markup {
    html! {
        html {
            head {
                title { "living-docs search" }
            }
            body {
                h1 { "living-docs search" }
                (search_form(query))
                @if let Some(query) = query {
                    (search_results(hits, query))
                }
            }
        }
    }
}

fn search_form(query: Option<&str>) -> Markup {
    html! {
        form action="/" method="get" {
            input type="search" name="q" value=(query.unwrap_or_default()) placeholder="Search docs…";
            button type="submit" { "Search" }
        }
    }
}

fn search_results(hits: &[SearchHit], query: &str) -> Markup {
    html! {
        @if hits.is_empty() {
            p class="empty-state" { "No results for \"" (query) "\"." }
        } @else {
            ul class="results" {
                @for hit in hits {
                    li {
                        a href=(record_href(&hit.path)) { (hit.title) }
                        p { (hit.snippet) }
                    }
                }
            }
        }
    }
}

fn record_href(path: &str) -> String {
    format!("/record/{path}")
}

/// Renders a single record's page: its escaped title plus its `body_html`,
/// already rendered from markdown by the caller and injected verbatim.
pub fn record_page(title: &str, body_html: &str) -> Markup {
    html! {
        html {
            head {
                title { (title) " — living-docs" }
            }
            body {
                a href="/" { "← Back to search" }
                h1 { (title) }
                (PreEscaped(body_html))
            }
        }
    }
}

/// Renders the page shown when `GET /record/{*path}` finds no matching
/// record.
pub fn not_found() -> Markup {
    html! {
        html {
            head {
                title { "Not found — living-docs" }
            }
            body {
                h1 { "Record not found" }
                p { "No record exists at this path." }
                a href="/" { "← Back to search" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(path: &str, title: &str, snippet: &str) -> SearchHit {
        SearchHit {
            path: path.to_owned(),
            title: title.to_owned(),
            snippet: snippet.to_owned(),
            project: "docs".to_owned(),
        }
    }

    #[test]
    fn search_page_without_a_query_renders_only_the_form() {
        let markup = search_page(None, &[]);

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
        )];

        let rendered = search_page(Some("<script>"), &hits).into_string();

        assert!(rendered.contains("href=\"/record/adr/0001-quokka-caching.md\""));
        assert!(rendered.contains("Quokka Caching Strategy"));
        assert!(!rendered.contains("<script>"));
        assert!(rendered.contains("&lt;script&gt;"));
    }

    #[test]
    fn search_page_with_no_hits_renders_the_empty_state_message() {
        let rendered = search_page(Some("zzzznomatch"), &[]).into_string();

        assert!(rendered.contains("empty-state"));
        assert!(rendered.contains("No results for &quot;zzzznomatch&quot;."));
    }

    #[test]
    fn record_page_renders_the_escaped_title_and_injects_the_preescaped_body() {
        let rendered = record_page(
            "Quokka Caching Strategy",
            "<h1>Quokka Caching Strategy</h1>\n<p>Body.</p>\n",
        )
        .into_string();

        assert!(rendered.contains("Quokka Caching Strategy"));
        assert!(rendered.contains("<h1>Quokka Caching Strategy</h1>"));
        assert!(rendered.contains("<p>Body.</p>"));
    }

    #[test]
    fn not_found_renders_the_not_found_copy() {
        let rendered = not_found().into_string();

        assert!(rendered.to_lowercase().contains("not found"));
    }
}
