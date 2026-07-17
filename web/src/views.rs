//! maud rendering for the read-only search page (ADR 0006, issue 0003 slice
//! S3a). Every value reflected from user input is rendered through maud's
//! auto-escaping `Markup`, never string-concatenated into HTML.

use db_store::SearchHit;
use maud::{html, Markup};

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

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(path: &str, title: &str, snippet: &str) -> SearchHit {
        SearchHit {
            path: path.to_owned(),
            title: title.to_owned(),
            snippet: snippet.to_owned(),
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
}
