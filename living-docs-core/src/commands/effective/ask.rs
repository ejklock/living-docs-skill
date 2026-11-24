//! The question `write new adr` asks of every new decision, and the count of
//! records in force it reports: the same text for the CLI and the MCP front.

use super::Options;
use crate::doc_type;
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

/// The count of records in force that `write new` reports before it writes,
/// or `None` for every type but the ADR, which is the only one that asks.
pub fn adr_in_force(store: &dyn DocStore, bundle: &Path, doc_type: &str) -> Option<usize> {
    let is_adr = doc_type::spec_for(doc_type).is_some_and(|spec| spec.token == "adr");
    is_adr.then(|| in_force_count(store, bundle))
}

/// How many records are in force in the whole bundle, retired ones excluded.
fn in_force_count(store: &dyn DocStore, bundle: &Path) -> usize {
    let options = Options {
        topic: None,
        full: false,
        contract: false,
        all: false,
        dependents: None,
    };
    super::read(store, bundle, &options, false).in_force
}
