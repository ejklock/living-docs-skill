use crate::frontmatter;
use crate::paths;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

const SUPPORTED_TYPES: [&str; 4] = ["adr", "bdr", "prd", "issue"];

pub fn run(docs_dir: &Path, doc_type: Option<String>) -> ExitCode {
    let types: Vec<String> = match doc_type {
        Some(t) => vec![t],
        None => SUPPORTED_TYPES.iter().map(|t| t.to_string()).collect(),
    };

    for doc_type in &types {
        if let Err(message) = regenerate(docs_dir, doc_type) {
            eprintln!("living-docs index: {message}");
            return ExitCode::from(2);
        }
    }

    ExitCode::SUCCESS
}

fn regenerate(docs_dir: &Path, doc_type: &str) -> Result<(), String> {
    let dir_name = paths::dir_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    let type_dir = docs_dir.join(dir_name);
    let records = collect_records(&type_dir)?;

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

/// Every `NNNN-*.md` record directly under `type_dir`, sorted ascending by `NNNN`.
/// `title`/`status` come from each record's frontmatter (S1's reader); `NNNN` comes
/// from the filename, matching how `next`/`new` allocate it.
fn collect_records(type_dir: &Path) -> Result<Vec<Record>, String> {
    let entries = match fs::read_dir(type_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.to_string()),
    };

    let mut records: Vec<Record> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| record_from_path(&entry.path()))
        .collect();

    records.sort_by_key(|record| record.number);
    Ok(records)
}

fn record_from_path(path: &Path) -> Option<Record> {
    let filename = path.file_name()?.to_str()?.to_string();
    let number = numbered_prefix(&filename)?;
    let title = frontmatter::read_scalar(path, "title").unwrap_or_default();
    let status = frontmatter::read_scalar(path, "status").unwrap_or_default();
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
}
