//! `anchor-missing` advisory: a local markdown link whose `#fragment` matches
//! no heading of its markdown target. Heading anchors follow GitHub's
//! algorithm (the one `github-slugger` implements): lowercase the heading's
//! text, remove every character matching `[^\p{Alphabetic}\p{M}\p{Nd}\p{Pc} -]`
//! (letters and letter numbers are inside `Alphabetic`, as are the circled and
//! squared Latin letters; a variation selector or a script's combining mark
//! stays while `²` and `½` go), turn each space into a hyphen, and number a
//! repeated slug `-1`, `-2`, … in document order.
//!
//! Accepted skew: github-slugger's table is frozen at an older Unicode
//! release, so a character added later (for example U+0870) is kept here and
//! removed there.
//!
//! Headings are read with `pulldown-cmark`, so Setext headings, inline markup
//! and fenced code behave as the renderer sees them; an image contributes no
//! text, since its alt text is not part of the rendered heading.
//!
//! The slug set of each file is computed once per run by [`SlugCache`], so a
//! target linked many times is parsed once.

use crate::check::graph::normpath;
use crate::check::{kinds, Reporter};
use crate::record::strip_frontmatter;
use crate::store::DocStore;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

/// The constant pattern always compiles; `Option` only keeps the failure path
/// free of a panic (a `None` would leave punctuation in every slug).
static REMOVED_CHARACTERS: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"[^\p{Alphabetic}\p{M}\p{Nd}\p{Pc} -]").ok());

/// The heading slugs of every file a run has looked at, by path; `None` for a
/// file that could not be read.
#[derive(Default)]
pub(super) struct SlugCache {
    by_path: HashMap<String, Option<HashSet<String>>>,
    #[cfg(test)]
    pub(super) parses: usize,
}

impl SlugCache {
    /// Advises when `dest` carries a fragment that no heading of its markdown
    /// target answers. `resolved` is the destination's normalized local path
    /// with the bundle root as spelled, or `None` for a same-file
    /// `#fragment`, which `content` (the linking file's text) answers. An
    /// external link, a non-markdown target, a target outside the bundle and
    /// an unreadable target are not judged.
    pub(super) fn check_anchor(
        &mut self,
        store: &dyn DocStore,
        file: &Path,
        content: &str,
        dest: &str,
        resolved: Option<(&str, &str)>,
        reporter: &mut Reporter,
    ) {
        let Some((_, fragment)) = dest.split_once('#').filter(|(_, f)| !f.is_empty()) else {
            return;
        };
        let (target, slugs) = match resolved {
            None if dest.starts_with('#') => {
                let key = file.display().to_string();
                let slugs = self.slugs_of(&key, || Some(content.to_owned()));
                (key, slugs)
            }
            None => return,
            Some((target, bundle)) => {
                if !is_markdown_inside(target, bundle) {
                    return;
                }
                let slugs = self.slugs_of(target, || store.read(Path::new(target)).ok());
                (target.to_owned(), slugs)
            }
        };
        if slugs.is_some_and(|slugs| !has_fragment(slugs, fragment)) {
            reporter.advise(
                file,
                kinds::ANCHOR_MISSING,
                format!("link `{dest}` matches no heading of {target}"),
            );
        }
    }

    fn slugs_of(
        &mut self,
        key: &str,
        load: impl FnOnce() -> Option<String>,
    ) -> Option<&HashSet<String>> {
        if !self.by_path.contains_key(key) {
            #[cfg(test)]
            {
                self.parses += 1;
            }
            let slugs = load().map(|text| heading_slugs(&text).into_iter().collect());
            self.by_path.insert(key.to_owned(), slugs);
        }
        self.by_path.get(key)?.as_ref()
    }
}

/// Whether `target` (already normalized) is a markdown file under `bundle`,
/// however the bundle was spelled (`./docs`, `docs/`, `.`).
fn is_markdown_inside(target: &str, bundle: &str) -> bool {
    let is_markdown = Path::new(target)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
    is_markdown && Path::new(target).starts_with(normpath(bundle))
}

/// The anchor of every heading of `markdown`, in document order, with a
/// repeated anchor numbered the way GitHub numbers it.
pub(super) fn heading_slugs(markdown: &str) -> Vec<String> {
    let mut slugs: Vec<String> = Vec::new();
    let mut taken: HashSet<String> = HashSet::new();
    let mut text: Option<String> = None;
    let mut image_depth = 0;
    for event in Parser::new(strip_frontmatter(markdown)) {
        match event {
            Event::Start(Tag::Heading { .. }) => text = Some(String::new()),
            Event::Start(Tag::Image { .. }) => image_depth += 1,
            Event::End(TagEnd::Image) => image_depth -= 1,
            Event::Text(t) | Event::Code(t) if image_depth == 0 => {
                if let Some(buffer) = text.as_mut() {
                    buffer.push_str(&t);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(heading) = text.take() {
                    slugs.push(unique(&slug(&heading), &mut taken));
                }
            }
            _ => {}
        }
    }
    slugs
}

/// `base`, or `base-N` for the first free `N` when `base` is already taken.
fn unique(base: &str, taken: &mut HashSet<String>) -> String {
    let mut candidate = base.to_owned();
    let mut n = 0;
    while taken.contains(&candidate) {
        n += 1;
        candidate = format!("{base}-{n}");
    }
    taken.insert(candidate.clone());
    candidate
}

fn slug(heading: &str) -> String {
    let lowered = heading.to_lowercase();
    let kept = match REMOVED_CHARACTERS.as_ref() {
        Some(removed) => removed.replace_all(&lowered, "").into_owned(),
        None => lowered,
    };
    kept.replace(' ', "-")
}

/// Whether `fragment`, as written or percent-decoded, is one of `slugs`.
pub(super) fn has_fragment(slugs: &HashSet<String>, fragment: &str) -> bool {
    slugs.contains(fragment) || slugs.contains(&percent_decode(fragment))
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests;
