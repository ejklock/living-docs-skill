//! The description shown at the end of a Nested listing row.

use super::super::rows::single_line;
use crate::doc_type::is_template_description;
use crate::frontmatter;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

/// The record's description, or an empty string when it has none or it is a
/// registered template's placeholder sentence.
pub(super) fn real_description(contents: &str) -> String {
    let description = frontmatter::read_scalar_from_str(contents, "description")
        .map(|text| single_line(text.trim()))
        .unwrap_or_default();
    if is_template_description(&description) {
        return String::new();
    }
    link_text_only(&description)
}

/// `text` with every markdown link, image and autolink written as its text
/// alone and every inline HTML tag dropped, so a relative link of the record
/// never lands in an index in another folder, where it would resolve to a
/// different file.
fn link_text_only(text: &str) -> String {
    let mut out = String::new();
    let mut cursor = 0;
    let mut link_depth = 0;
    for (event, range) in Parser::new(text).into_offset_iter() {
        match event {
            Event::Start(Tag::Link { .. } | Tag::Image { .. }) => {
                if link_depth == 0 {
                    out.push_str(&text[cursor..range.start]);
                }
                link_depth += 1;
            }
            Event::End(TagEnd::Link | TagEnd::Image) => {
                link_depth -= 1;
                cursor = range.end;
            }
            Event::Text(inner) | Event::Code(inner) if link_depth > 0 => out.push_str(&inner),
            Event::InlineHtml(_) | Event::Html(_) if link_depth == 0 => {
                out.push_str(&text[cursor..range.start]);
                cursor = range.end;
            }
            _ => {}
        }
    }
    out.push_str(&text[cursor..]);
    out
}
