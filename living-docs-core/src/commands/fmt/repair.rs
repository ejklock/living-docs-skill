//! The repairs `fmt` makes to a record's CLI-owned fields: a missing
//! timestamp, a missing owner on a type that requires one, and the title
//! heading. Git-derived inputs arrive through [`Seeds`], so this module does
//! no process or git I/O.

use crate::commands::new::now_iso8601;
use crate::commands::set::retitle::{replace_title_heading, title_heading_text};
use crate::doc_type::spec_for_frontmatter;
use crate::record::{ExtractedRecord, TailValue};
use std::path::Path;

const TIMESTAMP_KEY: &str = "timestamp";

/// The values `fmt` seeds into a record that lacks them, computed by the
/// caller because they come from git.
#[derive(Clone, Default)]
pub struct Seeds {
    /// The identity to seed a missing owner with, `None` when there is none.
    pub owner: Option<String>,
    /// Looks up when a record was created; `None` from the lookup (a file
    /// never committed, or outside a repository) falls back to the clock.
    /// A `None` field disables timestamp seeding altogether.
    pub added_at: Option<fn(&Path) -> Option<String>>,
}

/// Fills the absent CLI-owned fields of `record`; a present value is never
/// changed. `heading` says how to read the record's title heading, `None`
/// for a retired record whose heading is history; `keep_title` says whether
/// the source frontmatter carried a non-empty title, which is never replaced.
/// Returns whether the record now has a real title, kept or read from its
/// heading, rather than the filename or first-line fallback.
pub(super) fn seed_fields(
    record: &mut ExtractedRecord,
    path: &Path,
    heading: Option<HeadingKey<'_>>,
    keep_title: bool,
    seeds: &Seeds,
) -> bool {
    let mut titled = keep_title;
    if let (Some(HeadingKey(number)), false) = (heading, keep_title) {
        let text = title_heading_text(&record.body, number).filter(|text| !text.is_empty());
        if let Some(text) = text {
            record.title = text;
            titled = true;
        }
    }
    seed_timestamp(record, path, seeds);
    seed_owner(record, seeds);
    titled
}

fn seed_timestamp(record: &mut ExtractedRecord, path: &Path, seeds: &Seeds) {
    let Some(added_at) = seeds.added_at else {
        return;
    };
    let existing = record
        .frontmatter_tail
        .iter()
        .position(|(key, _)| key == TIMESTAMP_KEY);
    if existing.is_some_and(|at| !is_blank_scalar(&record.frontmatter_tail[at].1)) {
        return;
    }
    let Some(stamp) = added_at(path).or_else(|| now_iso8601().ok()) else {
        return;
    };
    let seeded = (TIMESTAMP_KEY.to_owned(), TailValue::Scalar(stamp));
    match existing {
        Some(at) => record.frontmatter_tail[at] = seeded,
        None => record.frontmatter_tail.push(seeded),
    }
}

fn is_blank_scalar(value: &TailValue) -> bool {
    matches!(value, TailValue::Scalar(text) if text.trim().is_empty())
}

fn seed_owner(record: &mut ExtractedRecord, seeds: &Seeds) {
    let requires_owner =
        spec_for_frontmatter(&record.doc_type).is_some_and(|spec| spec.requires_owner);
    if requires_owner && record.owner.is_none() {
        record.owner.clone_from(&seeds.owner);
    }
}

/// How to find a record's title heading: by its filename number when it has
/// one, else only by an unnumbered level-1 heading.
#[derive(Clone, Copy)]
pub(super) struct HeadingKey<'a>(pub(super) Option<&'a str>);

/// `canonical` with the text of its own title heading set to the trimmed
/// `title`; unchanged when `heading` is `None`, the title is blank, or the
/// body carries no title heading.
pub(super) fn align_heading(
    canonical: String,
    heading: Option<HeadingKey<'_>>,
    title: &str,
) -> String {
    let title = title.trim();
    match heading {
        Some(HeadingKey(number)) if !title.is_empty() => {
            replace_title_heading(&canonical, number, title).unwrap_or(canonical)
        }
        _ => canonical,
    }
}
