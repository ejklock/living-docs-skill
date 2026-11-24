//! The question `write new adr` asks of every new decision, the count of
//! records in force it reports and the in-force ADRs that share rare words
//! with the new one: the same text for the CLI and the MCP front.

use super::candidates::{candidates, AdrText};
use super::{is_in_force, read_records, Candidate};
use crate::doc_type;
use crate::record::ExtractedRecord;
use crate::store::DocStore;
use std::path::Path;

/// The fixed question `write new adr` asks of every new decision: it names no
/// record and claims no conflict, so it is the same for every ADR.
pub fn adr_ask() -> &'static str {
    "Does this decision contradict or partly reverse a record in force? List the records in force with `living-docs read records`."
}

/// The one stderr line `write new adr` prints: the count of records in force
/// followed by [`adr_ask`].
pub fn adr_ask_line(in_force: usize) -> String {
    let s = if in_force == 1 { "" } else { "s" };
    format!("{in_force} record{s} in force. {}", adr_ask())
}

/// The text of the record about to be written, which the in-force ADRs are
/// compared with: its title, and its description and body when given.
pub struct Query<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub body: Option<&'a str>,
}

impl<'a> Query<'a> {
    /// The new record's title, with its description and body when given.
    pub fn new(title: &'a str, description: Option<&'a str>, body: Option<&'a str>) -> Self {
        Self {
            title,
            description,
            body,
        }
    }

    fn text(&self) -> String {
        let parts = [Some(self.title), self.description, self.body];
        parts.into_iter().flatten().collect::<Vec<_>>().join("\n")
    }
}

/// What `write new adr` reports before it writes: the count of records in
/// force and the in-force ADRs that share rare words with the new one.
pub struct Asked {
    pub in_force: usize,
    pub candidates: Vec<Candidate>,
}

/// The count and candidates `write new` reports before it writes, from one
/// read of the bundle, or `None` for every type but the ADR, which is the
/// only one that asks.
pub fn adr_asked(
    store: &dyn DocStore,
    bundle: &Path,
    doc_type: &str,
    query: &Query,
) -> Option<Asked> {
    let is_adr = doc_type::spec_for(doc_type).is_some_and(|spec| spec.token == "adr");
    is_adr.then(|| asked(store, bundle, query))
}

/// Reads the bundle once: the in-force count covers every type, the
/// candidates only the in-force ADRs.
fn asked(store: &dyn DocStore, bundle: &Path, query: &Query) -> Asked {
    let records = read_records(store, &store.list(bundle).unwrap_or_default());
    let live: Vec<&ExtractedRecord> = records
        .iter()
        .map(|(_, record)| record)
        .filter(|record| is_in_force(record))
        .collect();
    let adr_type = doc_type::spec_for("adr").map(|spec| spec.frontmatter);
    let adrs: Vec<AdrText> = live
        .iter()
        .filter(|record| Some(record.doc_type.as_str()) == adr_type)
        .filter_map(|record| {
            Some(AdrText {
                number: record.number?,
                title: &record.title,
                description: &record.description,
                body: &record.body,
            })
        })
        .collect();
    Asked {
        in_force: live.len(),
        candidates: candidates(&query.text(), &adrs),
    }
}

/// The stderr line for one candidate: it names the record and the words it
/// shares, and claims nothing about the decisions.
pub fn candidate_line(candidate: &Candidate) -> String {
    let Candidate {
        doc_type,
        number,
        title,
        shared,
    } = candidate;
    format!(
        "shares words with {doc_type} {number:04} {title}: {}",
        shared.join(", ")
    )
}
