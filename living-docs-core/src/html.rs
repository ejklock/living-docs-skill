//! Single-file HTML render of a bundle for local reading: read-only, never
//! served or published, and byte-identical for the same bundle and the same
//! mermaid.js source.

use crate::check::links::{link_destinations, resolve_destination};
use crate::record::{self, ExtractedRecord};
use crate::store::DocStore;
use crate::{doc_type, frontmatter};
use page::Card;
use std::collections::HashMap;
use std::path::Path;

mod markdown;
mod page;

/// Where the rendered page takes mermaid.js from.
pub enum MermaidSource {
    /// The text of mermaid.js, inlined into the page.
    Inline(String),
    /// A script tag for the pinned CDN URL with its subresource-integrity hash.
    Pinned,
}

/// A record read from the bundle, with where it sits on the page.
struct Rec {
    id: String,
    rel: String,
    full: String,
    kind: String,
    record: ExtractedRecord,
}

/// Renders every record of the bundle at `bundle` into one self-contained,
/// deterministic HTML document: records grouped by type then status, each
/// with its successor and the records that link it. The page never names the
/// bundle's location.
pub fn render(store: &dyn DocStore, bundle: &Path, mermaid: &MermaidSource) -> String {
    let graph = Graph::new(load(store, bundle), &bundle.to_string_lossy());
    let mut main = String::new();
    let mut last: Option<(&str, &str)> = None;
    for (index, rec) in graph.recs.iter().enumerate() {
        let group = (
            rec.record.doc_type.as_str(),
            rec.record.status.as_deref().unwrap_or(""),
        );
        main.push_str(&headings(last, group));
        last = Some(group);
        main.push_str(&page::card(&graph.card(index)));
    }
    page::document(mermaid, &main)
}

/// The loaded records with the links between them.
struct Graph {
    recs: Vec<Rec>,
    by_key: HashMap<String, usize>,
    inbound: Vec<Vec<usize>>,
    bundle: String,
}

impl Graph {
    fn new(recs: Vec<Rec>, bundle: &str) -> Self {
        let by_key = recs
            .iter()
            .enumerate()
            .map(|(index, rec)| (key_of(&rec.full), index))
            .collect();
        let mut graph = Self {
            recs,
            by_key,
            inbound: Vec::new(),
            bundle: bundle.to_string(),
        };
        graph.inbound = inbound_links(&graph);
        graph
    }

    /// The record a link `dest` written in `recs[index]` names, if any.
    fn target(&self, index: usize, dest: &str) -> Option<usize> {
        let resolved = resolve_destination(&self.recs[index].full, dest, &self.bundle)?;
        self.by_key.get(&resolved).copied()
    }

    fn card(&self, index: usize) -> Card {
        let rec = &self.recs[index];
        let resolve = |dest: &str| Some(self.recs[self.target(index, dest)?].id.clone());
        Card {
            id: rec.id.clone(),
            title: rec.record.title.clone(),
            meta: meta_of(rec),
            description: rec.record.description.clone(),
            successor: successor(&self.recs, index).map(|next| reference(&self.recs[next])),
            body_html: markdown::to_html(&rec.record.body, &resolve),
            backlinks: self.inbound[index]
                .iter()
                .map(|&n| reference(&self.recs[n]))
                .collect(),
        }
    }
}

/// The type and status headings opening the group `now` after `last`.
fn headings(last: Option<(&str, &str)>, now: (&str, &str)) -> String {
    let mut out = String::new();
    if last.map(|(doc_type, _)| doc_type) != Some(now.0) {
        out.push_str(&page::heading(2, now.0));
    }
    if !same_group(last, now) && !now.1.is_empty() {
        out.push_str(&page::heading(3, now.1));
    }
    out
}

fn same_group(last: Option<(&str, &str)>, now: (&str, &str)) -> bool {
    last.is_some_and(|(doc_type, status)| doc_type == now.0 && status.eq_ignore_ascii_case(now.1))
}

fn reference(rec: &Rec) -> (String, String) {
    (rec.id.clone(), rec.record.title.clone())
}

fn meta_of(rec: &Rec) -> String {
    let mut parts = vec![rec.rel.clone()];
    parts.extend(rec.record.status.clone().filter(|s| !s.is_empty()));
    parts.extend(Some(rec.kind.clone()).filter(|k| !k.is_empty()));
    parts.join(" | ")
}

/// Every record of the bundle in page order: type by the registry's order,
/// then status by the type's vocabulary, then view kind, number and path.
fn load(store: &dyn DocStore, bundle: &Path) -> Vec<Rec> {
    let mut recs = Vec::new();
    for path in store.list(bundle).unwrap_or_default() {
        let Ok(contents) = store.read(&path) else {
            continue;
        };
        let record = record::extract_record(&path, &contents);
        if record.doc_type.is_empty() || is_reserved(&path) {
            continue;
        }
        let rel = relative(&path, bundle);
        recs.push(Rec {
            id: format!("rec-{}", slug(&rel)),
            full: path.to_string_lossy().into_owned(),
            kind: frontmatter::read_scalar_from_str(&contents, "kind").unwrap_or_default(),
            rel,
            record,
        });
    }
    recs.sort_by_cached_key(sort_key);
    recs
}

fn is_reserved(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("index.md" | "log.md")
    )
}

type SortKey = (usize, usize, String, usize, i32, String);

fn sort_key(rec: &Rec) -> SortKey {
    let doc_type = rec.record.doc_type.as_str();
    let types = doc_type::DOC_TYPES;
    let type_rank = types.iter().position(|spec| spec.frontmatter == doc_type);
    let status = rec.record.status.clone().unwrap_or_default();
    let folded = status.to_ascii_lowercase();
    let vocabulary = type_rank.map_or(&[][..], |at| types[at].status_vocabulary);
    let status_rank = vocabulary
        .iter()
        .position(|listed| listed.eq_ignore_ascii_case(&status));
    let kind_rank = doc_type::VIEW_KIND_ORDER
        .iter()
        .position(|listed| *listed == rec.kind);
    (
        type_rank.unwrap_or(types.len()),
        status_rank.unwrap_or(vocabulary.len()),
        folded,
        kind_rank.unwrap_or(doc_type::VIEW_KIND_ORDER.len()),
        rec.record.number.unwrap_or(i32::MAX),
        rec.rel.clone(),
    )
}

/// The path below the bundle with forward slashes; a path outside the bundle
/// keeps only its file name so no location leaks into the page.
fn relative(path: &Path, bundle: &Path) -> String {
    let below = path
        .strip_prefix(bundle)
        .unwrap_or_else(|_| path.file_name().map_or(path, |name| Path::new(name)));
    below
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn slug(rel: &str) -> String {
    rel.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// A record path in the normalized form `resolve_destination` returns, so a
/// record and a link resolved to it compare equal.
fn key_of(full: &str) -> String {
    let name = Path::new(full)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    resolve_destination(full, &name, "").unwrap_or_default()
}

/// For each record, the records whose body links it, each listed once in
/// page order.
fn inbound_links(graph: &Graph) -> Vec<Vec<usize>> {
    let mut inbound = vec![Vec::new(); graph.recs.len()];
    for (source, rec) in graph.recs.iter().enumerate() {
        for dest in link_destinations(&rec.record.body) {
            let target = graph.target(source, &dest);
            if let Some(target) = target.filter(|target| *target != source) {
                if inbound[target].last() != Some(&source) {
                    inbound[target].push(source);
                }
            }
        }
    }
    inbound
}

/// The record that replaces `recs[index]`: the one its `superseded_by` names,
/// else one of the same type whose `supersedes` lists its number.
fn successor(recs: &[Rec], index: usize) -> Option<usize> {
    let record = &recs[index].record;
    let same_type = |other: &Rec| other.record.doc_type == record.doc_type;
    let named = record
        .superseded_by
        .as_deref()
        .and_then(|text| text.trim().parse::<i32>().ok())
        .and_then(|number| {
            recs.iter()
                .position(|other| same_type(other) && other.record.number == Some(number))
        });
    named.or_else(|| {
        let number = record.number?;
        recs.iter().position(|other| {
            same_type(other)
                && other
                    .record
                    .supersedes
                    .iter()
                    .flatten()
                    .any(|listed| listed.trim().parse::<i32>().ok() == Some(number))
        })
    })
}

#[cfg(test)]
mod tests;
