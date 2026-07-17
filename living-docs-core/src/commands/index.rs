use crate::frontmatter;
use crate::paths;
use crate::store::DocStore;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

const SUPPORTED_TYPES: [&str; 4] = ["adr", "bdr", "prd", "issue"];

pub fn run(store: &dyn DocStore, docs_dir: &Path, doc_type: Option<String>) -> ExitCode {
    let types: Vec<String> = match doc_type {
        Some(t) => vec![t],
        None => SUPPORTED_TYPES.iter().map(|t| t.to_string()).collect(),
    };

    for doc_type in &types {
        if let Err(message) = regenerate(store, docs_dir, doc_type) {
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
fn regenerate(store: &dyn DocStore, docs_dir: &Path, doc_type: &str) -> Result<(), String> {
    let dir_name = paths::dir_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    let type_dir = docs_dir.join(dir_name);
    let records = collect_records(store, docs_dir, &type_dir)?;

    let index_path = type_dir.join("index.md");
    let existing = fs::read_to_string(&index_path).unwrap_or_default();
    let preamble = preamble_for(&existing, doc_type);
    let body = render_body(doc_type, &records);

    fs::create_dir_all(&type_dir).map_err(|e| e.to_string())?;
    fs::write(&index_path, format!("{preamble}{body}")).map_err(|e| e.to_string())
}

fn unsupported_type_message(doc_type: &str) -> String {
    format!("unsupported doc type '{doc_type}' (expected one of adr, bdr, prd, issue)")
}

struct Record {
    number: u32,
    title: String,
    status: String,
    filename: String,
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
    let title = frontmatter::read_scalar_from_str(&contents, "title").unwrap_or_default();
    let status = frontmatter::read_scalar_from_str(&contents, "status").unwrap_or_default();
    Some(Record {
        number,
        title,
        status,
        filename,
    })
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

fn render_body(doc_type: &str, records: &[Record]) -> String {
    if doc_type == "adr" {
        render_adr_body(records)
    } else {
        render_flat_body(records)
    }
}

fn render_flat_body(records: &[Record]) -> String {
    if records.is_empty() {
        return String::new();
    }
    render_rows(records) + "\n"
}

/// ADR listing splits `## Active` (Proposed|Accepted) above `## Superseded`
/// (Superseded|Deprecated) so a reader sees what is in force without scrolling
/// through history — see `skills/living-docs/rules/adr-conventions.md` rule 7.
fn render_adr_body(records: &[Record]) -> String {
    let (active, superseded): (Vec<&Record>, Vec<&Record>) = records
        .iter()
        .partition(|record| is_active_status(&record.status));

    let mut body = String::from("## Active\n");
    if !active.is_empty() {
        body.push('\n');
        body.push_str(&render_rows_ref(&active));
        body.push('\n');
    }

    if !superseded.is_empty() {
        body.push_str("\n## Superseded\n\n");
        body.push_str(&render_rows_ref(&superseded));
        body.push('\n');
    }

    body
}

fn is_active_status(status: &str) -> bool {
    matches!(status, "Proposed" | "Accepted")
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
    } = record;
    format!("* [{number:04} — {title}]({filename}) - {status}")
}

/// Everything above the first generator-managed heading survives byte-for-byte —
/// this is what makes `index` idempotent on the second run, since the boundary is
/// found at the same offset both times. A fresh (or marker-less) file falls back to
/// a minimal `# <Title>` preamble.
fn preamble_for(existing: &str, doc_type: &str) -> String {
    match find_boundary_offset(existing, doc_type) {
        Some(offset) => existing[..offset].to_string(),
        None => fallback_preamble(existing, doc_type),
    }
}

fn find_boundary_offset(existing: &str, doc_type: &str) -> Option<usize> {
    let mut offset = 0;
    for line in existing.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if is_boundary_line(trimmed, doc_type) {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

fn is_boundary_line(line: &str, doc_type: &str) -> bool {
    if doc_type == "adr" {
        line == "## Active"
    } else {
        line.starts_with("* [")
    }
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
    match doc_type {
        "adr" => "ADRs",
        "bdr" => "BDRs",
        "prd" => "PRDs",
        "issue" => "Issues",
        _ => "Index",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbered_prefix_accepts_four_digit_dash_form() {
        assert_eq!(numbered_prefix("0007-old.md"), Some(7));
    }

    #[test]
    fn numbered_prefix_rejects_index_and_malformed_names() {
        assert_eq!(numbered_prefix("index.md"), None);
        assert_eq!(numbered_prefix("12-old.md"), None);
        assert_eq!(numbered_prefix("abcd-old.md"), None);
    }

    #[test]
    fn render_row_matches_the_locked_row_format() {
        let record = Record {
            number: 7,
            title: "My Title".to_string(),
            status: "Proposed".to_string(),
            filename: "0007-my-title.md".to_string(),
        };
        assert_eq!(
            render_row(&record),
            "* [0007 — My Title](0007-my-title.md) - Proposed"
        );
    }

    #[test]
    fn fallback_preamble_is_minimal_for_a_fresh_file() {
        assert_eq!(fallback_preamble("", "adr"), "# ADRs\n\n");
    }

    #[test]
    fn fallback_preamble_wraps_unmarked_existing_content() {
        assert_eq!(
            fallback_preamble("Custom intro.\n", "prd"),
            "Custom intro.\n\n"
        );
    }

    #[test]
    fn find_boundary_offset_locates_the_adr_active_heading() {
        let existing = "# ADRs\n\nIntro.\n\n## Active\n\n* [0001 — X](0001-x.md) - Proposed\n";
        let offset = find_boundary_offset(existing, "adr").unwrap();
        assert_eq!(
            &existing[offset..],
            "## Active\n\n* [0001 — X](0001-x.md) - Proposed\n"
        );
    }

    #[test]
    fn find_boundary_offset_locates_the_first_row_for_non_adr_types() {
        let existing = "# PRDs\n\nIntro.\n\n* [0001 — X](0001-x.md) - Draft\n";
        let offset = find_boundary_offset(existing, "prd").unwrap();
        assert_eq!(&existing[offset..], "* [0001 — X](0001-x.md) - Draft\n");
    }

    use std::collections::BTreeMap;
    use std::io;
    use std::path::PathBuf;

    /// A minimal in-memory [`DocStore`] test double, proving `collect_records`
    /// reads a record's title/status through the port rather than the
    /// filesystem — the same double pattern used by `export.rs`/`new.rs`.
    struct MapStore {
        files: BTreeMap<PathBuf, String>,
    }

    impl DocStore for MapStore {
        fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(self
                .files
                .keys()
                .filter(|path| path.starts_with(root))
                .cloned()
                .collect())
        }

        fn read(&self, path: &Path) -> io::Result<String> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
        }

        fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn collect_records_reads_title_and_status_through_the_store() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-first.md"),
            "---\ntype: ADR\ntitle: First\nstatus: Accepted\n---\n# First\n".to_string(),
        );
        let store = MapStore { files };

        let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
            .expect("collect_records should succeed");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].title, "First");
        assert_eq!(records[0].status, "Accepted");
    }

    #[test]
    fn collect_records_ignores_paths_the_store_lists_outside_the_type_directory() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-in-scope.md"),
            "---\ntype: ADR\ntitle: In Scope\nstatus: Proposed\n---\n# In Scope\n".to_string(),
        );
        files.insert(
            PathBuf::from("/bundle/bdr/0001-other-type.md"),
            "---\ntype: BDR\ntitle: Other Type\nstatus: Draft\n---\n# Other Type\n".to_string(),
        );
        let store = MapStore { files };

        let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
            .expect("collect_records should succeed");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].filename, "0001-in-scope.md");
    }

    #[test]
    fn collect_records_on_an_empty_store_returns_no_records() {
        let store = MapStore {
            files: BTreeMap::new(),
        };

        let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
            .expect("collect_records should succeed on an empty store");

        assert!(records.is_empty());
    }
}
