//! The page shell and record cards of the single-file render: inline style
//! and scripts only, with the pinned mermaid script tag as the sole external
//! reference and then only when the caller asked for it.

use super::markdown::escape;
use super::MermaidSource;

const PINNED_MERMAID: &str = "<script src=\"https://cdn.jsdelivr.net/npm/mermaid@12.3.0/dist/mermaid.min.js\" integrity=\"sha384-HHW54r639fK42Ip0coGF3mz8oSiGn0eEoWNTLs75X7H63pLSDBa0Tqs0RwGmLlXE\" crossorigin=\"anonymous\"></script>";

const STYLE: &str =
    "body{font:16px/1.5 system-ui,sans-serif;max-width:60rem;margin:0 auto;padding:1rem}\
article{border:1px solid #ccc;border-radius:6px;padding:.5rem 1rem;margin:1rem 0}\
article[hidden],section[hidden]{display:none}\
.meta{color:#555;font-size:.85rem}\
pre{overflow:auto;background:#f5f5f5;padding:.5rem}\
pre.mermaid{background:none}\
table{border-collapse:collapse}td,th{border:1px solid #ccc;padding:.2rem .5rem}\
#filter{width:100%;padding:.5rem;font-size:1rem}";

const FILTER_SCRIPT: &str = "<script>document.getElementById('filter').addEventListener('input',function(e){var q=e.target.value.toLowerCase();document.querySelectorAll('article.record').forEach(function(a){a.hidden=q!==''&&a.textContent.toLowerCase().indexOf(q)<0;});document.querySelectorAll('section.group').forEach(function(g){g.hidden=!g.querySelector('article.record:not([hidden])');});});</script>";

/// Closes a section opened by `group_open`.
pub(super) const GROUP_CLOSE: &str = "</section>\n";

const MERMAID_START: &str =
    "<script>if(window.mermaid){mermaid.initialize({startOnLoad:true});}</script>";

/// One record as it appears on the page. `successor` and each backlink are a
/// fragment id with the title to show.
pub(super) struct Card {
    pub id: String,
    pub title: String,
    pub meta: String,
    pub description: String,
    pub successor: Option<(String, String)>,
    pub body_html: String,
    pub backlinks: Vec<(String, String)>,
}

/// The whole document around `main`, the already-built record sections.
pub(super) fn document(mermaid: &MermaidSource, main: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>Living Docs</title>\n<style>{STYLE}</style>\n</head>\n<body>\n<h1>Living Docs</h1>\n<input id=\"filter\" type=\"search\" placeholder=\"Filter records\" aria-label=\"Filter records\">\n<main>\n{main}</main>\n{FILTER_SCRIPT}\n{}\n{MERMAID_START}\n</body>\n</html>\n",
        mermaid_script(mermaid)
    )
}

fn mermaid_script(mermaid: &MermaidSource) -> String {
    match mermaid {
        MermaidSource::Pinned => PINNED_MERMAID.to_string(),
        MermaidSource::Inline(text) => {
            let safe = neutralise_script_ends(text);
            format!("<script>{safe}</script>")
        }
    }
}

/// Opens a group section holding a heading of the given level; the caller
/// closes it with `GROUP_CLOSE` after the group's records.
pub(super) fn group_open(level: u8, text: &str) -> String {
    format!(
        "<section class=\"group\">\n<h{level}>{}</h{level}>\n",
        escape(text)
    )
}

pub(super) fn card(card: &Card) -> String {
    let mut out = format!(
        "<article class=\"record\" id=\"{}\">\n<h4>{}</h4>\n<p class=\"meta\">{}</p>\n",
        escape(&card.id),
        escape(&card.title),
        escape(&card.meta)
    );
    if !card.description.is_empty() {
        out.push_str(&format!("<p>{}</p>\n", escape(&card.description)));
    }
    if let Some((id, title)) = &card.successor {
        out.push_str(&format!("<p>Superseded by {}</p>\n", anchor(id, title)));
    }
    out.push_str(&format!("<div class=\"body\">\n{}</div>\n", card.body_html));
    if !card.backlinks.is_empty() {
        let items: Vec<String> = card
            .backlinks
            .iter()
            .map(|(id, title)| format!("<li>{}</li>", anchor(id, title)))
            .collect();
        out.push_str(&format!(
            "<footer>Linked from<ul>{}</ul></footer>\n",
            items.join("")
        ));
    }
    out.push_str("</article>\n");
    out
}

fn anchor(id: &str, title: &str) -> String {
    format!("<a href=\"#{}\">{}</a>", escape(id), escape(title))
}

/// `text` with every `</script`, `<script` and `<!--` (any case) broken so
/// the inlined text can neither end its element nor put the HTML tokenizer
/// into its double-escaped state. Each rewrite keeps the same meaning inside
/// a JavaScript string, template literal or regular expression, including one
/// with the `u` or `v` flag: `\/`, `\u00xx` and `\x21` are valid escapes there,
/// where `\!` is not.
fn neutralise_script_ends(text: &str) -> String {
    let ends = replace_ignoring_case(text, "</script", |hit| format!("<\\/{}", &hit[2..]));
    let starts = replace_ignoring_case(&ends, "<script", |hit| {
        let first = hit.as_bytes()[1];
        format!("<\\u{:04x}{}", first, &hit[2..])
    });
    replace_ignoring_case(&starts, "<!--", |_| "<\\x21--".to_string())
}

fn replace_ignoring_case(text: &str, needle: &str, with: impl Fn(&str) -> String) -> String {
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    while let Some(found) = lower[from..].find(needle) {
        let at = from + found;
        out.push_str(&text[from..at]);
        out.push_str(&with(&text[at..at + needle.len()]));
        from = at + needle.len();
    }
    out.push_str(&text[from..]);
    out
}
