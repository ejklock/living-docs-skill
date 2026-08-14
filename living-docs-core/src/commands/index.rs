use crate::commands::new::unsupported_type_message;
use crate::doc_type::{self, Identity, IndexPartition};
use crate::frontmatter;
use crate::store::DocStore;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod named;

/// Every registry token with a directory to index — Numbered and Named
/// identities alike (ADR 0026, ADR 0036) — in [`doc_type::DOC_TYPES`]
/// order: the set `index` regenerates when invoked with no explicit type.
/// A [`Identity::Singleton`] type has no directory to index, so the bare
/// sweep excludes it — regenerating it would need a directory that `new`
/// never creates for a singleton.
fn all_type_tokens() -> Vec<String> {
    doc_type::DOC_TYPES
        .iter()
        .filter(|spec| {
            matches!(
                spec.identity,
                Identity::Numbered { .. } | Identity::Named { .. }
            )
        })
        .map(|spec| spec.token.to_string())
        .collect()
}

pub fn run(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: Option<String>,
    visibility_filter: Option<Vec<String>>,
) -> ExitCode {
    let types: Vec<String> = match doc_type {
        Some(t) => vec![t],
        None => all_type_tokens(),
    };

    for doc_type in &types {
        if let Err(message) = regenerate(store, docs_dir, doc_type, visibility_filter.as_deref()) {
            eprintln!("living-docs index: {message}");
            return ExitCode::from(2);
        }
    }

    ExitCode::SUCCESS
}

/// `index.md` itself is a reserved fs presentation artifact outside every
/// `DocStore` domain (ADR 0007: never synced to `db-store`), so it is always
/// read/written through `std::fs` regardless of the active backend — only
/// the records feeding its body are read through `store`, meaning a db-mode
/// run regenerates the filesystem `index.md` from the records in the
/// database.
///
/// `doc_type`'s directory coming into existence is `new`'s job, never
/// `index`'s (ADR 0026): a type with no directory yet is a successful no-op
/// here, both for the bare `index` sweep and for an explicit `index
/// <type>` naming a type the bundle doesn't use — otherwise a bare sweep
/// would materialize an empty `index.md` per registry token regardless of
/// whether the bundle carries that type, breaking invariant 3 (an
/// unreachable directory index) for every type the bundle never populated.
fn regenerate(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    visibility_filter: Option<&[String]>,
) -> Result<(), String> {
    let (index_path, content) = compute(store, docs_dir, doc_type, visibility_filter)?;
    let type_dir = index_path.parent().unwrap_or(docs_dir);
    if !type_dir.is_dir() {
        return Ok(());
    }
    fs::write(&index_path, content).map_err(|e| e.to_string())
}

/// Computes `doc_type`'s regenerated `index.md` path and full content,
/// reading the current on-disk file (if any) to preserve its preamble and
/// reading the records feeding its body through `store`, without touching
/// the filesystem itself — the pure step both [`regenerate`] (CLI `index`)
/// and `db-store`'s `write_checked` build on, the latter needing to inspect
/// and control the write/rollback timing itself.
pub fn compute(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    visibility_filter: Option<&[String]>,
) -> Result<(PathBuf, String), String> {
    let dir_name = numbered_dir_for(doc_type)?;
    let type_dir = docs_dir.join(dir_name);
    let index_path = type_dir.join("index.md");
    let existing = fs::read_to_string(&index_path).unwrap_or_default();
    let preamble = preamble_for(&existing, doc_type);
    let body = body_for(store, docs_dir, doc_type, &type_dir, visibility_filter)?;

    Ok((index_path, format!("{preamble}{body}")))
}

/// Renders `doc_type`'s listing body along its identity shape: a Named type
/// delegates to [`named::render_body`] (kind-ranked view rows, ADR 0036),
/// a Numbered one collects `NNNN-*.md` records and renders along its
/// registry partition axis.
fn body_for(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    type_dir: &Path,
    visibility_filter: Option<&[String]>,
) -> Result<String, String> {
    let is_named = matches!(
        doc_type::spec_for(doc_type).map(|spec| spec.identity),
        Some(Identity::Named { .. })
    );
    if is_named {
        return named::render_body(store, docs_dir, type_dir, visibility_filter);
    }
    let records: Vec<Record> = collect_records(store, docs_dir, type_dir)?
        .into_iter()
        .filter(|record| record_visible(record, visibility_filter))
        .collect();
    Ok(render_body(doc_type, &records))
}

/// Resolves the numbered-series directory `index` regenerates for
/// `doc_type`: an unknown token gets [`unsupported_type_message`], but a
/// registered [`Identity::Singleton`] token gets its own message instead —
/// it IS supported, it simply has no directory index, and reusing the
/// unsupported-type message would list `doc_type` itself among the tokens
/// the caller is told to pick from.
fn numbered_dir_for(doc_type: &str) -> Result<&'static str, String> {
    let spec = doc_type::spec_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    match spec.identity {
        Identity::Numbered { dir } | Identity::Named { dir } => Ok(dir),
        Identity::Singleton { file } => {
            Err(singleton_has_no_directory_index_message(doc_type, file))
        }
    }
}

fn singleton_has_no_directory_index_message(doc_type: &str, file: &str) -> String {
    format!(
        "'{doc_type}' has no directory index — it writes a single {file} at the bundle root, not a numbered series"
    )
}

struct Record {
    number: u32,
    title: String,
    status: String,
    filename: String,
    visibility: String,
}

/// The default-deny fallback effective visibility for a record whose
/// frontmatter carries no `visibility` key at all.
const DEFAULT_VISIBILITY: &str = "private";

/// True when `record` belongs in the rendered index under `filter`: every
/// record passes when `filter` is `None` (today's unfiltered dev view, ADR
/// 0009), otherwise only a record whose effective visibility is a member of
/// `filter` passes — default-deny, so an absent-visibility record is only
/// included when `filter` explicitly names `"private"`.
fn record_visible(record: &Record, filter: Option<&[String]>) -> bool {
    match filter {
        None => true,
        Some(allowed) => allowed.contains(&record.visibility),
    }
}

/// Every `NNNN-*.md` record directly under `type_dir`, sorted ascending by
/// `NNNN`, read through `store` (backend-faithful: a db-mode run sees
/// exactly the records the database lists, not whatever happens to sit on
/// disk). `title`/`status` come from each record's frontmatter (S1's
/// reader); `NNNN` comes from the filename, matching how `next`/`new`
/// allocate it.
fn collect_records(
    store: &dyn DocStore,
    docs_dir: &Path,
    type_dir: &Path,
) -> Result<Vec<Record>, String> {
    let paths = store.list(docs_dir).map_err(|e| e.to_string())?;

    let mut records: Vec<Record> = paths
        .iter()
        .filter(|path| path.parent() == Some(type_dir))
        .filter_map(|path| record_from_path(store, path))
        .collect();

    records.sort_by_key(|record| record.number);
    Ok(records)
}

fn record_from_path(store: &dyn DocStore, path: &Path) -> Option<Record> {
    let filename = path.file_name()?.to_str()?.to_string();
    let number = numbered_prefix(&filename)?;
    let contents = store.read(path).ok()?;
    let title = title_for_record(&contents, path, number);
    let status = frontmatter::read_scalar_from_str(&contents, "status").unwrap_or_default();
    let visibility = frontmatter::read_scalar_from_str(&contents, "visibility")
        .unwrap_or_else(|| DEFAULT_VISIBILITY.to_string());
    Some(Record {
        number,
        title,
        status,
        filename,
        visibility,
    })
}

/// The record's rendered title: its frontmatter `title:` when present and
/// parseable, otherwise its first `# ` H1 heading with a leading numbering
/// prefix stripped (issue 0021 cause 2 — legacy records that only ever
/// carried the title in their heading). A stderr warning names `path`
/// whenever the fallback fires, since a blank/substituted title is otherwise
/// invisible in the rendered index; only when the H1 is also absent does the
/// title stay empty (still warned).
fn title_for_record(contents: &str, path: &Path, number: u32) -> String {
    if let Some(title) = frontmatter::read_scalar_from_str(contents, "title") {
        return title;
    }
    let fallback = first_heading(contents)
        .map(|heading| strip_heading_number_prefix(&heading, number))
        .unwrap_or_default();
    warn_missing_title(path, &fallback);
    fallback
}

fn first_heading(contents: &str) -> Option<String> {
    contents
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(|title| title.trim().to_owned()))
}

/// Strips a leading `ADR NNNN — `, `NNNN. `, or `NNNN — ` numbering prefix
/// (`NNNN` being `number` zero-padded to four digits) from `heading`, in
/// that order, leaving it untouched when none matches.
fn strip_heading_number_prefix(heading: &str, number: u32) -> String {
    let padded = format!("{number:04}");
    [
        format!("ADR {padded} — "),
        format!("{padded}. "),
        format!("{padded} — "),
    ]
    .into_iter()
    .find_map(|prefix| heading.strip_prefix(prefix.as_str()).map(str::to_owned))
    .unwrap_or_else(|| heading.to_owned())
}

fn warn_missing_title(path: &Path, fallback: &str) {
    if fallback.is_empty() {
        eprintln!(
            "living-docs index: {} has no parseable 'title' frontmatter and no H1 heading; rendering an empty title",
            path.display()
        );
    } else {
        eprintln!(
            "living-docs index: {} has no parseable 'title' frontmatter; using its H1 heading {fallback:?}",
            path.display()
        );
    }
}

fn numbered_prefix(filename: &str) -> Option<u32> {
    if !filename.ends_with(".md") || filename.as_bytes().get(4) != Some(&b'-') {
        return None;
    }
    let prefix = filename.get(0..4)?;
    if !prefix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    prefix.parse().ok()
}

/// Renders `records` along the partition axis `doc_type`'s registry spec
/// declares (ADR 0026): [`IndexPartition::OpenClosed`] for work-in-progress
/// types, [`IndexPartition::ActiveSuperseded`] for types that track what is
/// in force, and [`IndexPartition::Flat`] — also the fallback for an
/// unrecognized `doc_type`, unreachable in practice since every caller
/// already validated it — as a single flat listing (`render_flat_body`).
fn render_body(doc_type: &str, records: &[Record]) -> String {
    match doc_type::spec_for(doc_type).map(|spec| &spec.index_partition) {
        Some(IndexPartition::OpenClosed) => {
            render_partitioned(records, "Open", "Closed", is_open_status)
        }
        Some(IndexPartition::ActiveSuperseded) => {
            render_partitioned(records, "Active", "Superseded", is_active_status)
        }
        Some(IndexPartition::Flat) | None => render_flat_body(records),
    }
}

fn render_flat_body(records: &[Record]) -> String {
    if records.is_empty() {
        return String::new();
    }
    render_rows(records) + "\n"
}

/// Splits records into a `first_heading` section above a `second_heading`
/// section, keyed by `in_first`, so a reader sees what matters now without
/// scrolling through history — see
/// `skills/living-docs/rules/adr-conventions.md` rule 7 for the decision-type
/// case this generalizes from. The first heading is always emitted; either
/// section's rows are omitted (heading only) when that bucket is empty.
fn render_partitioned(
    records: &[Record],
    first_heading: &str,
    second_heading: &str,
    in_first: fn(&str) -> bool,
) -> String {
    let (first, second): (Vec<&Record>, Vec<&Record>) =
        records.iter().partition(|record| in_first(&record.status));

    let mut body = format!("## {first_heading}\n");
    if !first.is_empty() {
        body.push('\n');
        body.push_str(&render_rows_ref(&first));
        body.push('\n');
    }

    if !second.is_empty() {
        body.push_str(&format!("\n## {second_heading}\n\n"));
        body.push_str(&render_rows_ref(&second));
        body.push('\n');
    }

    body
}

/// The decision-type axis (adr/bdr/prd): everything not explicitly retired
/// is still in force, so new decision statuses (e.g. a future vocabulary
/// entry) default to Active without special-casing each type's own words.
fn is_active_status(status: &str) -> bool {
    !matches!(status, "Superseded" | "Deprecated")
}

/// The issue work axis: matched case-insensitively so `done` and `Done` both
/// land in Closed alongside `closed`/`superseded` — the repo's real tracker
/// uses `done` as its closed value. An unknown/empty status is presumed not
/// done yet, so it defaults to Open.
fn is_open_status(status: &str) -> bool {
    !matches!(
        status.to_ascii_lowercase().as_str(),
        "closed" | "done" | "superseded"
    )
}

fn render_rows(records: &[Record]) -> String {
    render_rows_ref(&records.iter().collect::<Vec<_>>())
}

fn render_rows_ref(records: &[&Record]) -> String {
    records
        .iter()
        .map(|record| render_row(record))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_row(record: &Record) -> String {
    let Record {
        number,
        title,
        filename,
        status,
        visibility: _,
    } = record;
    format!("* [{number:04} — {title}]({filename}) - {status}")
}

/// Everything above the first generator-managed heading survives byte-for-byte —
/// this is what makes `index` idempotent on the second run, since the boundary is
/// found at the same offset both times. A fresh (or marker-less) file falls back to
/// a minimal `# <Title>` preamble.
fn preamble_for(existing: &str, doc_type: &str) -> String {
    match find_boundary_offset(existing) {
        Some(offset) => existing[..offset].to_string(),
        None => fallback_preamble(existing, doc_type),
    }
}

fn find_boundary_offset(existing: &str) -> Option<usize> {
    let mut offset = 0;
    for line in existing.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if is_boundary_line(trimmed) {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

/// Any generator-managed heading (`## `, whatever its text), bullet listing
/// row, or hand-maintained Markdown table listing row is a boundary,
/// whichever comes first. A single prefix check — rather than pinning the
/// exact heading text per type — is what lets a legacy issues index still
/// carrying `## Done`/`## Open` sections migrate cleanly: the first `## `
/// line is found and replaced, regardless of its old wording. Recognizing a
/// table listing row too (issue 0021 cause 1) is what turns a hand-maintained
/// table-format index into a single migration pass instead of a silent
/// append below it.
fn is_boundary_line(line: &str) -> bool {
    line.starts_with("## ") || line.starts_with("* [") || is_table_listing_row(line)
}

/// True for a Markdown table row (`| cell | cell | ... |`) whose first cell
/// is either a numbered-listing header (`| # |`) or a record link
/// (`| [NNNN](...)` / `| [NNNN-...`) — the two shapes a hand-maintained
/// record table uses in place of the generator's bullet format.
fn is_table_listing_row(line: &str) -> bool {
    let Some(first_cell) = first_table_cell(line) else {
        return false;
    };
    first_cell == "#" || is_record_link_cell(first_cell)
}

fn first_table_cell(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix('|')?;
    let cell = rest.split('|').next()?;
    Some(cell.trim())
}

fn is_record_link_cell(cell: &str) -> bool {
    let Some(after_bracket) = cell.strip_prefix('[') else {
        return false;
    };
    after_bracket.len() >= 4
        && after_bracket
            .get(..4)
            .is_some_and(|digits| digits.chars().all(|c| c.is_ascii_digit()))
}

fn fallback_preamble(existing: &str, doc_type: &str) -> String {
    let trimmed = existing.trim();
    if trimmed.is_empty() {
        format!("# {}\n\n", heading_title_for(doc_type))
    } else {
        format!("{trimmed}\n\n")
    }
}

fn heading_title_for(doc_type: &str) -> &'static str {
    doc_type::spec_for(doc_type)
        .map(|spec| spec.index_heading)
        .unwrap_or("Index")
}

#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod tests;
