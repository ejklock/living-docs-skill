use crate::frontmatter;
use crate::paths;
use crate::store::DocStore;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

const SUPPORTED_TYPES: [&str; 4] = ["adr", "bdr", "prd", "issue"];

pub fn run(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: Option<String>,
    visibility_filter: Option<Vec<String>>,
) -> ExitCode {
    let types: Vec<String> = match doc_type {
        Some(t) => vec![t],
        None => SUPPORTED_TYPES.iter().map(|t| t.to_string()).collect(),
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
fn regenerate(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    visibility_filter: Option<&[String]>,
) -> Result<(), String> {
    let dir_name = paths::dir_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    let type_dir = docs_dir.join(dir_name);
    let records: Vec<Record> = collect_records(store, docs_dir, &type_dir)?
        .into_iter()
        .filter(|record| record_visible(record, visibility_filter))
        .collect();

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
    let title = frontmatter::read_scalar_from_str(&contents, "title").unwrap_or_default();
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

/// Dispatches each supported type to the partition axis its lifecycle uses:
/// issues track work-in-progress (Open/Closed), decisions track what is in
/// force (Active/Superseded). A future/unknown type falls back to a flat
/// listing until its own axis is chosen — see `render_flat_body`.
fn render_body(doc_type: &str, records: &[Record]) -> String {
    match doc_type {
        "issue" => render_partitioned(records, "Open", "Closed", is_open_status),
        "adr" | "bdr" | "prd" => {
            render_partitioned(records, "Active", "Superseded", is_active_status)
        }
        _ => render_flat_body(records),
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

/// Any generator-managed heading (`## `, whatever its text) or listing row
/// is a boundary, whichever comes first. A single prefix check — rather than
/// pinning the exact heading text per type — is what lets a legacy issues
/// index still carrying `## Done`/`## Open` sections migrate cleanly: the
/// first `## ` line is found and replaced, regardless of its old wording.
fn is_boundary_line(line: &str) -> bool {
    line.starts_with("## ") || line.starts_with("* [")
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
            visibility: "private".to_string(),
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
        let offset = find_boundary_offset(existing).unwrap();
        assert_eq!(
            &existing[offset..],
            "## Active\n\n* [0001 — X](0001-x.md) - Proposed\n"
        );
    }

    #[test]
    fn find_boundary_offset_locates_the_first_row_for_non_adr_types() {
        let existing = "# PRDs\n\nIntro.\n\n* [0001 — X](0001-x.md) - Draft\n";
        let offset = find_boundary_offset(existing).unwrap();
        assert_eq!(&existing[offset..], "* [0001 — X](0001-x.md) - Draft\n");
    }

    #[test]
    fn find_boundary_offset_locates_a_legacy_heading_regardless_of_its_text() {
        let existing = "# Issues\n\nIntro.\n\n## Done\n\n* [0001 — X](0001-x.md) - closed\n";
        let offset = find_boundary_offset(existing).unwrap();
        assert_eq!(
            &existing[offset..],
            "## Done\n\n* [0001 — X](0001-x.md) - closed\n"
        );
    }

    #[test]
    fn is_open_status_treats_closed_done_and_superseded_case_insensitively_as_closed() {
        assert!(!is_open_status("closed"));
        assert!(!is_open_status("Closed"));
        assert!(!is_open_status("done"));
        assert!(!is_open_status("Done"));
        assert!(!is_open_status("Superseded"));
    }

    #[test]
    fn is_open_status_treats_open_in_progress_and_unknown_as_open() {
        assert!(is_open_status("open"));
        assert!(is_open_status("in-progress"));
        assert!(is_open_status("Mystery"));
        assert!(is_open_status(""));
    }

    #[test]
    fn is_active_status_treats_superseded_and_deprecated_as_not_active() {
        assert!(!is_active_status("Superseded"));
        assert!(!is_active_status("Deprecated"));
    }

    #[test]
    fn is_active_status_treats_draft_accepted_and_implemented_as_active() {
        assert!(is_active_status("Draft"));
        assert!(is_active_status("Accepted"));
        assert!(is_active_status("Implemented"));
        assert!(is_active_status("Proposed"));
    }

    #[test]
    fn render_partitioned_pins_the_adr_active_superseded_byte_shape() {
        let records = vec![
            Record {
                number: 1,
                title: "Old".to_string(),
                status: "Superseded".to_string(),
                filename: "0001-old.md".to_string(),
                visibility: "private".to_string(),
            },
            Record {
                number: 2,
                title: "Current".to_string(),
                status: "Accepted".to_string(),
                filename: "0002-current.md".to_string(),
                visibility: "private".to_string(),
            },
        ];

        let body = render_partitioned(&records, "Active", "Superseded", is_active_status);

        assert_eq!(
            body,
            "## Active\n\n* [0002 — Current](0002-current.md) - Accepted\n\n## Superseded\n\n* [0001 — Old](0001-old.md) - Superseded\n"
        );
    }

    #[test]
    fn render_partitioned_emits_only_the_first_heading_when_the_second_bucket_is_empty() {
        let records = vec![Record {
            number: 1,
            title: "Only".to_string(),
            status: "open".to_string(),
            filename: "0001-only.md".to_string(),
            visibility: "private".to_string(),
        }];

        let body = render_partitioned(&records, "Open", "Closed", is_open_status);

        assert_eq!(body, "## Open\n\n* [0001 — Only](0001-only.md) - open\n");
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

    #[test]
    fn collect_records_defaults_to_private_when_visibility_is_absent() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-first.md"),
            "---\ntype: ADR\ntitle: First\nstatus: Accepted\n---\n# First\n".to_string(),
        );
        let store = MapStore { files };

        let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
            .expect("collect_records should succeed");

        assert_eq!(records[0].visibility, "private");
    }

    #[test]
    fn collect_records_reads_an_explicit_visibility_value() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-first.md"),
            "---\ntype: ADR\ntitle: First\nstatus: Accepted\nvisibility: public\n---\n# First\n"
                .to_string(),
        );
        let store = MapStore { files };

        let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
            .expect("collect_records should succeed");

        assert_eq!(records[0].visibility, "public");
    }

    fn record_with_visibility(visibility: &str) -> Record {
        Record {
            number: 1,
            title: "Title".to_string(),
            status: "Accepted".to_string(),
            filename: "0001-title.md".to_string(),
            visibility: visibility.to_string(),
        }
    }

    #[test]
    fn record_visible_passes_every_record_when_the_filter_is_none() {
        assert!(record_visible(&record_with_visibility("private"), None));
        assert!(record_visible(&record_with_visibility("public"), None));
    }

    #[test]
    fn record_visible_excludes_a_record_outside_the_filter_set() {
        let filter = vec!["public".to_string(), "showcase".to_string()];
        assert!(!record_visible(
            &record_with_visibility("private"),
            Some(&filter)
        ));
    }

    #[test]
    fn record_visible_includes_a_record_inside_the_filter_set() {
        let filter = vec!["public".to_string(), "showcase".to_string()];
        assert!(record_visible(
            &record_with_visibility("public"),
            Some(&filter)
        ));
        assert!(record_visible(
            &record_with_visibility("showcase"),
            Some(&filter)
        ));
    }

    #[test]
    fn record_visible_default_deny_only_admits_private_when_explicitly_requested() {
        let private_filter = vec!["private".to_string()];
        let public_filter = vec!["public".to_string()];
        let absent_visibility = record_with_visibility(DEFAULT_VISIBILITY);

        assert!(record_visible(&absent_visibility, Some(&private_filter)));
        assert!(!record_visible(&absent_visibility, Some(&public_filter)));
    }
}
