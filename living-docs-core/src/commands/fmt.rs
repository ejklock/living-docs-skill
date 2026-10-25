//! `living-docs fmt` canonicalizes a concept record's frontmatter in
//! place, seeds a missing timestamp and a missing required owner,
//! aligns the title heading with the frontmatter title, and reconciles
//! the retired-record callout at the top of its body to match the status
//! that frontmatter carries — every other byte below the closing `---`
//! stays byte-for-byte unchanged. `target`
//! accepts either a bundle root (canonicalizes every record under it,
//! enumerated through the same `DocStore::list` call `check::compile` reads
//! from — no second directory walker) or a single record path
//! (canonicalizes only that record). With `check_only`, no record is
//! written: the command reports which records are pending instead.
//! File-mode only: db-mode is canonical by construction on export, so
//! this verb never runs against `--backend db`.

mod repair;

use crate::callout;
use crate::check::is_retired_record;
use crate::commands::set::retitle::number_prefix;
use crate::commands::supersede::guard::{refusal_message, unwritable_key};
use crate::frontmatter::read_scalar_from_str as frontmatter_scalar;
use crate::line_ending::{refuse_mixed, LineEndingStore};
use crate::record::{extract_record, to_canonical_markdown, ExtractedRecord};
use crate::store::DocStore;
pub use repair::Seeds;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Canonicalizes `target`'s frontmatter — a bundle root or a single record
/// path — printing each pending record and a summary count. With
/// `check_only`, prints the same pending records but writes nothing, and
/// returns a non-zero exit code when any record is pending.
pub fn run(store: &dyn DocStore, target: &Path, check_only: bool) -> ExitCode {
    match outcome(store, target, check_only, &Seeds::default()) {
        Ok(Outcome::Rewritten(paths)) => {
            for path in &paths {
                println!("{}", path.display());
            }
            println!("{} record(s) rewritten.", paths.len());
            ExitCode::SUCCESS
        }
        Ok(Outcome::Pending(paths)) => {
            for path in &paths {
                println!("{}", path.display());
            }
            println!("{} record(s) would change.", paths.len());
            if paths.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(message) => {
            eprintln!("living-docs fmt: {message}");
            ExitCode::from(2)
        }
    }
}

/// `fmt`'s outcome: which records were rewritten, or (`--check`) which are
/// pending. Kept as one type so a caller matches on the mode it asked for
/// rather than juggling two return values.
pub enum Outcome {
    Rewritten(Vec<PathBuf>),
    Pending(Vec<PathBuf>),
}

/// Canonicalizes `target`'s frontmatter (or, under `check_only`, reports
/// which records would change) without printing anything — the CLI front
/// renders this as colored text or JSON; [`run`] is the plain-text-always
/// convenience wrapper over it. `seeds` carries the git-derived values a
/// missing timestamp or owner is filled with.
pub fn outcome(
    store: &dyn DocStore,
    target: &Path,
    check_only: bool,
    seeds: &Seeds,
) -> Result<Outcome, String> {
    let Some(paths) = resolve_targets(store, target) else {
        return Err(format!("bundle root not found: {}", target.display()));
    };
    refuse_mixed_endings(store, &paths)?;
    let lf_view = LineEndingStore::new(store, false);
    let store: &dyn DocStore = &lf_view;
    let outcome = if check_only {
        Outcome::Pending(pending_records(store, &paths, seeds))
    } else {
        Outcome::Rewritten(canonicalize_bundle(store, &paths, seeds))
    };
    let refused = refusals(store, &paths, seeds);
    if refused.is_empty() {
        Ok(outcome)
    } else {
        Err(refused.join("\n"))
    }
}

/// One message per record whose canonical form would not read back as the
/// record does; such a record is never rewritten and is not listed as
/// pending, and the run as a whole fails after the other records are handled.
fn refusals(store: &dyn DocStore, paths: &[PathBuf], seeds: &Seeds) -> Vec<String> {
    paths
        .iter()
        .filter(|path| !is_reserved_file(path))
        .filter_map(|path| {
            let (contents, canonical) = drafted(store, path, seeds)?;
            let key = unwritable_key(&contents, &canonical)?;
            Some(refusal_message(path, &key))
        })
        .collect()
}

/// Refuses the whole run, before anything is written, when a record `fmt`
/// would canonicalize has mixed CRLF and LF line endings. Reserved files and
/// prose without frontmatter are never rewritten, so they are not judged.
fn refuse_mixed_endings(store: &dyn DocStore, paths: &[PathBuf]) -> Result<(), String> {
    let mixed: Vec<String> = paths
        .iter()
        .filter(|path| !is_reserved_file(path))
        .filter(|path| store.read(path).is_ok_and(|text| has_frontmatter(&text)))
        .filter_map(|path| refuse_mixed(store, path).err())
        .collect();
    if mixed.is_empty() {
        Ok(())
    } else {
        Err(mixed.join("\n"))
    }
}

/// Resolves `target` to the record paths `fmt` should consider: a single
/// path when `target` is a file, every listed `.md` file when it is a
/// directory, or `None` when it is neither.
fn resolve_targets(store: &dyn DocStore, target: &Path) -> Option<Vec<PathBuf>> {
    if target.is_file() {
        Some(vec![target.to_path_buf()])
    } else if target.is_dir() {
        Some(store.list(target).unwrap_or_default())
    } else {
        None
    }
}

/// Returns which of `paths` are non-canonical, without writing any of them.
fn pending_records(store: &dyn DocStore, paths: &[PathBuf], seeds: &Seeds) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|path| !is_reserved_file(path))
        .filter(|path| canonical_form_if_pending(store, path, seeds).is_some())
        .cloned()
        .collect()
}

/// Canonicalizes every non-reserved record in `all_md`, returning the paths
/// actually rewritten.
fn canonicalize_bundle(store: &dyn DocStore, all_md: &[PathBuf], seeds: &Seeds) -> Vec<PathBuf> {
    all_md
        .iter()
        .filter(|path| !is_reserved_file(path))
        .filter(|path| canonicalize_record(store, path, seeds))
        .cloned()
        .collect()
}

/// `index.md`/`log.md` carry no frontmatter and are never part of the
/// record domain (mirrors `check::records::is_reserved`).
fn is_reserved_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("index.md") | Some("log.md")
    )
}

/// Rewrites `path` to its canonical form when it already carries a
/// frontmatter block whose canonical re-serialization differs byte for
/// byte from its current contents. Returns whether a write happened.
fn canonicalize_record(store: &dyn DocStore, path: &Path, seeds: &Seeds) -> bool {
    let Some(canonical) = canonical_form_if_pending(store, path, seeds) else {
        return false;
    };
    store.write(path, &canonical).is_ok()
}

/// Returns the repaired canonical re-serialization of `path` when it differs
/// from the current contents and reads back as the record does. Returns `None`
/// when `path` cannot be read, carries no frontmatter, is already canonical,
/// or is refused by the read-back guard.
fn canonical_form_if_pending(store: &dyn DocStore, path: &Path, seeds: &Seeds) -> Option<String> {
    let (contents, canonical) = drafted(store, path, seeds)?;
    if canonical == contents || unwritable_key(&contents, &canonical).is_some() {
        None
    } else {
        Some(canonical)
    }
}

/// Reads `path` and returns its contents with the repaired canonical
/// re-serialization. `None` when `path` cannot be read or carries no
/// frontmatter.
fn drafted(store: &dyn DocStore, path: &Path, seeds: &Seeds) -> Option<(String, String)> {
    let contents = store.read(path).ok()?;
    if !has_frontmatter(&contents) {
        return None;
    }
    let normalized = callout::normalize_frontmatter_gap(&contents);
    let number = number_prefix(path).ok();
    let heading =
        (!is_retired_record(&normalized)).then_some(repair::HeadingKey(number.as_deref()));
    let keep_title = frontmatter_scalar(&normalized, "title").is_some_and(|t| !t.trim().is_empty());
    let mut record = extract_record(path, &normalized);
    record.body = reconciled_body(store, path, &record);
    let titled = repair::seed_fields(&mut record, path, heading, keep_title, seeds);
    let canonical = to_canonical_markdown(&record);
    let canonical = repair::align_heading(canonical, heading.filter(|_| titled), &record.title);
    Some((contents, canonical))
}

/// The body `record` should carry once its retired-record callout matches
/// its status: unchanged when `record` is active and carries none, gains
/// or loses one otherwise.
fn reconciled_body(store: &dyn DocStore, path: &Path, record: &ExtractedRecord) -> String {
    let successor = record
        .superseded_by
        .as_deref()
        .map(|superseded_by| callout::successor_filename(store, path, superseded_by));
    let expected = callout::expected(record.status.as_deref(), successor.as_deref());
    callout::reconcile_body(&record.body, expected.as_deref())
}

fn has_frontmatter(contents: &str) -> bool {
    contents.lines().next() == Some("---")
}

#[cfg(test)]
mod heading_tests;
#[cfg(test)]
mod tests;
