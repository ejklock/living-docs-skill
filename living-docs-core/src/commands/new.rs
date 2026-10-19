use crate::commands::next::next_number_from_store;

mod fill;
use crate::commands::index::{refresh_for_records, Indexed};
use crate::doc_type::{self, Identity};
use crate::paths;
use crate::store::DocStore;
pub(crate) use fill::{
    fill_body, fill_frontmatter, fill_frontmatter_description, fill_frontmatter_kind,
    fill_frontmatter_owner, fill_frontmatter_title, fill_heading,
};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

/// The point-of-use teaching line (ADR 0019, decision 3): printed after
/// `new`'s created path, and repeated verbatim in the root `--help` about
/// text and the `living-docs` SKILL.md stub, so an agent meets the
/// CLI-owns-the-mechanics rule at the moment it authors a record.
/// Optional authoring inputs for `new` beyond the type and title:
/// `--description`, `--kind` (ADR 0036) and `--owner`.
#[derive(Default)]
pub struct NewOptions<'a> {
    pub description: Option<&'a str>,
    pub kind: Option<&'a str>,
    /// Seeds the frontmatter `owner:` field with this value, inserted in its
    /// canonical position immediately after `description:`. `None` leaves
    /// the record with no `owner:` field at all.
    pub owner: Option<&'a str>,
    /// Replaces everything below the title heading — the template's slots —
    /// with this text. `None` keeps the scaffolded slots.
    pub body: Option<&'a str>,
}

pub const BODY_ONLY_INSTRUCTION: &str = "Write ONLY the body below the closing ---. Frontmatter and indexes are CLI-owned: `living-docs set` / `supersede`.";

pub fn run(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    opts: &NewOptions,
) -> ExitCode {
    match write(store, docs_dir, doc_type, title, opts) {
        Ok(created) => {
            println!("{}", created.path.display());
            println!("{BODY_ONLY_INSTRUCTION}");
            report_index_failure(&created.indexed)
        }
        Err(message) => {
            eprintln!("living-docs new: {message}");
            ExitCode::from(2)
        }
    }
}

/// Names the first index that could not be refreshed and exits 1, so a
/// script sees the half-finished state a usage error (exit 2) never has.
fn report_index_failure(indexed: &Indexed) -> ExitCode {
    match indexed.failures.first() {
        None => ExitCode::SUCCESS,
        Some(failure) => {
            eprintln!("living-docs new: {failure}");
            ExitCode::from(1)
        }
    }
}

/// The record `new` wrote and what refreshing its type's index did.
#[derive(Debug)]
pub struct Created {
    pub path: PathBuf,
    pub indexed: Indexed,
}

/// Scaffolds and writes the new record, then refreshes its type's index,
/// returning both without printing anything — the CLI front renders this as
/// colored text or JSON; [`run`] is the plain-text-always
/// convenience wrapper over it.
pub fn write(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    opts: &NewOptions,
) -> Result<Created, String> {
    let path = scaffold(store, docs_dir, doc_type, title, opts, &now_iso8601()?)?;
    let indexed = refresh_for_records(store, docs_dir, &[path.as_path()]);
    Ok(Created { path, indexed })
}

/// Computes `new`'s target path and filled content without writing it —
/// the pure planning half of [`scaffold`], reused by [`plan`] (with
/// today's timestamp) and by any caller (e.g.
/// `db_store::DbDocStore::write_checked`) that needs to run its own
/// transactional write instead of [`crate::store::DocStore::write`].
fn plan_at(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    opts: &NewOptions,
    timestamp: &str,
) -> Result<(PathBuf, String), String> {
    let spec = doc_type::spec_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    let target_path = target_path_for(store, docs_dir, spec, title)?;

    if store.read(&target_path).is_ok() {
        return Err(format!("{} already exists", target_path.display()));
    }

    let filled = fill_frontmatter(spec.template, spec.frontmatter, timestamp);
    let filled = fill_frontmatter_title(&filled, title);
    let filled = fill_frontmatter_description(&filled, opts.description);
    let filled = fill_frontmatter_owner(&filled, opts.owner);
    let filled = fill_frontmatter_kind(&filled, spec, opts.kind)?;
    let filled = fill_heading(&filled, title, numbered_prefix_of(&target_path));
    let filled = fill_body(&filled, opts.body);
    Ok((target_path, filled))
}

/// The `NNNN` filename prefix of a numbered record's target path, `None`
/// for singleton and named identities.
fn numbered_prefix_of(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    let prefix = name.get(0..4)?;
    (name.as_bytes().get(4) == Some(&b'-') && prefix.chars().all(|c| c.is_ascii_digit()))
        .then(|| prefix.parse().ok())
        .flatten()
}

/// Resolves `new`'s target path for `spec`'s identity shape: a
/// [`Identity::Numbered`] type allocates the next number and slugifies
/// `title` into `<dir>/NNNN-<slug>.md`; a [`Identity::Singleton`] type
/// allocates nothing and resolves straight to `docs_dir.join(file)` — no
/// number, no slug. Everything after this call (the clobber guard, the
/// frontmatter fill) is identity-blind, so only the path itself differs.
fn target_path_for(
    store: &dyn DocStore,
    docs_dir: &Path,
    spec: &doc_type::DocTypeSpec,
    title: &str,
) -> Result<PathBuf, String> {
    match spec.identity {
        Identity::Numbered { dir: dir_name } => {
            let number =
                next_number_from_store(store, docs_dir, dir_name).map_err(|e| e.to_string())?;
            Ok(docs_dir
                .join(dir_name)
                .join(format!("{number:04}-{}.md", paths::slugify(title))))
        }
        Identity::Singleton { file } => Ok(docs_dir.join(file)),
        Identity::Named { dir: dir_name } => Ok(docs_dir
            .join(dir_name)
            .join(format!("{}.md", paths::slugify(title)))),
    }
}

/// Plans `new`'s target path and filled content, timestamped now, without
/// writing it — the counterpart a caller uses when it owns its own write
/// (e.g. a transactional write+check verb) instead of going through
/// [`scaffold`]'s call to [`crate::store::DocStore::write`].
pub fn plan(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    opts: &NewOptions,
) -> Result<(PathBuf, String), String> {
    plan_at(store, docs_dir, doc_type, title, opts, &now_iso8601()?)
}

fn scaffold(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    opts: &NewOptions,
    timestamp: &str,
) -> Result<PathBuf, String> {
    let (target_path, filled) = plan_at(store, docs_dir, doc_type, title, opts, timestamp)?;
    store
        .write(&target_path, &filled)
        .map_err(|e| e.to_string())?;
    Ok(target_path)
}

/// The single definition of `new`/`index`'s unsupported-type error, naming
/// the offending `doc_type` and every token the registry ([`doc_type::DOC_TYPES`])
/// currently supports, rather than a hand-maintained list (ADR 0026).
pub(crate) fn unsupported_type_message(doc_type: &str) -> String {
    let supported = doc_type::DOC_TYPES
        .iter()
        .map(|spec| spec.token)
        .collect::<Vec<_>>()
        .join(", ");
    format!("unsupported doc type '{doc_type}' (expected one of {supported})")
}

pub(crate) fn now_iso8601() -> Result<String, String> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before the unix epoch".to_string())?
        .as_secs();
    let (year, month, day) = civil_date_from_unix_days((secs / 86_400) as i64);
    let time_of_day = secs % 86_400;
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        time_of_day / 3_600,
        (time_of_day % 3_600) / 60,
        time_of_day % 60
    ))
}

/// Days-since-epoch to (year, month, day) via Howard Hinnant's
/// `civil_from_days` (proleptic Gregorian) — the only way to produce an
/// ISO-8601 date from `std` alone, since this slice adds no chrono
/// dependency.
fn civil_date_from_unix_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

#[cfg(test)]
mod scaffold_tests;
#[cfg(test)]
mod tests;
