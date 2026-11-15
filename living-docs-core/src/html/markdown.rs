//! Markdown body to HTML for the single-file render. Raw HTML in a body is
//! shown as escaped text, never emitted as markup; images are reduced to their
//! alt text so the page loads nothing; a link keeps only a web, mail, fragment
//! or relative destination; a `mermaid` fence becomes a diagram container
//! holding its escaped source.

use pulldown_cmark::{
    html::push_html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use pulldown_cmark_escape::escape_html;

/// Escapes `text` for HTML element content and quoted attribute values.
pub(super) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    match escape_html(&mut out, text) {
        Ok(()) | Err(_) => out,
    }
}

/// Renders `body`; `resolve` maps a link destination to the fragment id of
/// the bundle record it names, if any.
pub(super) fn to_html(body: &str, resolve: &dyn Fn(&str) -> Option<String>) -> String {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let mut in_diagram = false;
    let mut events = Vec::new();
    for event in Parser::new_ext(body, options) {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) if is_mermaid(&info) => {
                in_diagram = true;
                events.push(Event::Html("<pre class=\"mermaid\">".into()));
            }
            Event::End(TagEnd::CodeBlock) if in_diagram => {
                in_diagram = false;
                events.push(Event::Html("</pre>\n".into()));
            }
            other => events.extend(neutralise(other, resolve)),
        }
    }
    let mut out = String::new();
    push_html(&mut out, events.into_iter());
    out
}

fn neutralise<'a>(event: Event<'a>, resolve: &dyn Fn(&str) -> Option<String>) -> Option<Event<'a>> {
    match event {
        Event::Html(text) | Event::InlineHtml(text) => Some(Event::Text(text)),
        Event::Start(Tag::Image { .. }) | Event::End(TagEnd::Image) => None,
        Event::Start(Tag::Heading {
            level,
            id,
            classes,
            attrs,
        }) => Some(Event::Start(Tag::Heading {
            level: below_title(level),
            id,
            classes,
            attrs,
        })),
        Event::End(TagEnd::Heading(level)) => Some(Event::End(TagEnd::Heading(below_title(level)))),
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Some(Event::Start(Tag::Link {
            link_type,
            dest_url: link_target(&dest_url, resolve).into(),
            title,
            id,
        })),
        other => Some(other),
    }
}

/// A body heading level shifted down by four so the body outline keeps its
/// relative levels below the card title, capped at H6.
fn below_title(level: HeadingLevel) -> HeadingLevel {
    match level {
        HeadingLevel::H1 => HeadingLevel::H5,
        HeadingLevel::H2
        | HeadingLevel::H3
        | HeadingLevel::H4
        | HeadingLevel::H5
        | HeadingLevel::H6 => HeadingLevel::H6,
    }
}

fn is_mermaid(info: &str) -> bool {
    info.split_whitespace().next() == Some("mermaid")
}

fn link_target(dest: &str, resolve: &dyn Fn(&str) -> Option<String>) -> String {
    if let Some(id) = resolve(dest) {
        return format!("#{id}");
    }
    let head = dest.split(['/', '?', '#']).next().unwrap_or("");
    let lower = dest.to_ascii_lowercase();
    let web = ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme));
    if web || !head.contains(':') {
        dest.to_string()
    } else {
        "#".to_string()
    }
}
