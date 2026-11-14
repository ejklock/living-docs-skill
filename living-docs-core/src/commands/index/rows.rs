//! Row and partition rendering for a numbered-identity directory listing:
//! the decision-type (Active/Superseded) and issue-work (Open/Closed) axes,
//! plus the flat fallback, all built on the same per-row rendering that
//! names a retired record's successor when one is known.

use crate::doc_type::{self, IndexPartition};

use super::Record;

/// The reminder rendered above the `## Superseded` section's rows: a
/// retired record is history, not something to act on — `living-docs
/// read` is the read verb for what is currently in force.
pub(super) const RETIRED_SECTION_NOTE: &str = "_History only. Do not act on these records — run \
     `living-docs read records` for what is in force._";

/// Renders `records` along the partition axis `doc_type`'s registry spec
/// declares: [`IndexPartition::OpenClosed`] for work-in-progress types,
/// [`IndexPartition::ActiveSuperseded`] for types that track what is in
/// force, and [`IndexPartition::Flat`] — also the fallback for an
/// unrecognized `doc_type`, unreachable in practice since every caller
/// already validated it — as a single flat listing (`render_flat_body`).
pub(super) fn render_body(doc_type: &str, records: &[Record]) -> String {
    match doc_type::spec_for(doc_type).map(|spec| &spec.index_partition) {
        Some(IndexPartition::OpenClosed) => {
            render_partitioned(records, "Open", "Closed", is_open_status, None)
        }
        Some(IndexPartition::ActiveSuperseded) => render_partitioned(
            records,
            "Active",
            "Superseded",
            is_active_status,
            Some(RETIRED_SECTION_NOTE),
        ),
        Some(IndexPartition::Flat) | None => render_flat_body(records),
    }
}

pub(super) fn render_flat_body(records: &[Record]) -> String {
    if records.is_empty() {
        return String::new();
    }
    render_rows(records) + "\n"
}

/// Splits records into a `first_heading` section above a `second_heading`
/// section, keyed by `in_first`, so a reader sees what matters now without
/// scrolling through history. The first heading is always emitted; either
/// section's rows are omitted (heading only) when that bucket is empty.
/// `second_note`, when given, is rendered as one line under the second
/// heading, with a blank line on each side, before its rows.
pub(super) fn render_partitioned(
    records: &[Record],
    first_heading: &str,
    second_heading: &str,
    in_first: fn(&str) -> bool,
    second_note: Option<&str>,
) -> String {
    let (first, second): (Vec<&Record>, Vec<&Record>) =
        records.iter().partition(|record| in_first(&record.status));

    let mut body = format!("## {first_heading}\n");
    if !first.is_empty() {
        body.push('\n');
        body.push_str(&render_rows_ref(&first, records));
        body.push('\n');
    }

    if !second.is_empty() {
        body.push_str(&format!("\n## {second_heading}\n\n"));
        if let Some(note) = second_note {
            body.push_str(note);
            body.push_str("\n\n");
        }
        body.push_str(&render_rows_ref(&second, records));
        body.push('\n');
    }

    body
}

/// The decision-type axis (adr/prd): everything not explicitly retired
/// is still in force, so new decision statuses (e.g. a future vocabulary
/// entry) default to Active without special-casing each type's own words.
pub(super) fn is_active_status(status: &str) -> bool {
    !matches!(status, "Superseded" | "Deprecated")
}

/// The issue work axis: matched case-insensitively so `done` and `Done` both
/// land in Closed alongside `closed`/`superseded` — the repo's real tracker
/// uses `done` as its closed value. An unknown/empty status is presumed not
/// done yet, so it defaults to Open.
pub(super) fn is_open_status(status: &str) -> bool {
    !matches!(
        status.to_ascii_lowercase().as_str(),
        "closed" | "done" | "superseded"
    )
}

pub(super) fn render_rows(records: &[Record]) -> String {
    render_rows_ref(&records.iter().collect::<Vec<_>>(), records)
}

pub(super) fn render_rows_ref(rows: &[&Record], all: &[Record]) -> String {
    rows.iter()
        .map(|record| render_row(record, all))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn render_row(record: &Record, all: &[Record]) -> String {
    let Record {
        number,
        title,
        filename,
        status,
        superseded_by,
    } = record;
    let rendered_status =
        retirement_suffix(status, superseded_by.as_deref(), all).unwrap_or_else(|| status.clone());
    format!("* [{number:04} — {title}]({filename}) - {rendered_status}")
}

/// The rendered suffix for a Superseded or Deprecated row, naming the
/// successor record when one is known. `None` for every other status, so
/// the caller falls back to the plain status word unchanged.
fn retirement_suffix(status: &str, superseded_by: Option<&str>, all: &[Record]) -> Option<String> {
    match status {
        "Superseded" => Some(superseded_suffix(superseded_by, all)),
        "Deprecated" => Some("Deprecated (no successor)".to_string()),
        _ => None,
    }
}

fn superseded_suffix(superseded_by: Option<&str>, all: &[Record]) -> String {
    let Some(raw) = superseded_by else {
        return "Superseded".to_string();
    };
    let padded = padded_number(raw);
    match successor_filename(raw, all) {
        Some(filename) => format!("Superseded by [{padded}]({filename})"),
        None => format!("Superseded by {padded}"),
    }
}

fn padded_number(raw: &str) -> String {
    raw.trim()
        .parse::<u32>()
        .map(|number| format!("{number:04}"))
        .unwrap_or_else(|_| raw.to_string())
}

fn successor_filename(raw: &str, all: &[Record]) -> Option<String> {
    let number: u32 = raw.trim().parse().ok()?;
    all.iter()
        .find(|record| record.number == number)
        .map(|record| record.filename.clone())
}

#[cfg(test)]
mod tests;
