use crate::doc_type;

/// Everything above the first generator-managed heading survives byte-for-byte —
/// this is what makes `index` idempotent on the second run, since the boundary is
/// found at the same offset both times. A fresh (or marker-less) file falls back to
/// a minimal `# <Title>` preamble.
pub(super) fn preamble_for(existing: &str, doc_type: &str) -> String {
    match find_boundary_offset(existing) {
        Some(offset) => existing[..offset].to_string(),
        None => fallback_preamble(existing, doc_type),
    }
}

pub(super) fn find_boundary_offset(existing: &str) -> Option<usize> {
    let raw: Vec<&str> = existing.split_inclusive('\n').collect();
    let lines: Vec<&str> = raw
        .iter()
        .map(|line| line.trim_end_matches(['\n', '\r']))
        .collect();
    let mut offset = 0;
    for (index, line) in raw.iter().enumerate() {
        if is_boundary_line(lines[index]) || starts_table_listing(&lines[index..]) {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

/// Any generator-managed heading (`## `, whatever its text), bullet listing
/// row (`* [`, or a `- [` whose link names a numbered record file), or
/// hand-maintained Markdown table listing row is a boundary,
/// whichever comes first — letting a legacy `## Done`/`## Open` index or a
/// hand-maintained table migrate in a single pass rather than
/// a silent append below it.
pub(super) fn is_boundary_line(line: &str) -> bool {
    line.starts_with("## ")
        || line.starts_with("* [")
        || is_record_dash_bullet(line)
        || is_table_listing_row(line)
}

/// True when `lines` opens a hand table: a header row with any first cell, a
/// separator row, then a record listing row. A legend table whose rows are
/// not record links never qualifies.
fn starts_table_listing(lines: &[&str]) -> bool {
    let [header, separator, row, ..] = lines else {
        return false;
    };
    first_table_cell(header).is_some() && is_separator_row(separator) && is_table_listing_row(row)
}

fn is_separator_row(line: &str) -> bool {
    let Some(rest) = line.trim().strip_prefix('|') else {
        return false;
    };
    rest.contains('-') && rest.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// `- [text](0001-foo.md)` or `- [text](./0001-foo.md)`: a dash bullet whose
/// link destination starts, bare or after `./`, with four digits and a dash.
/// A path such as `../adr/0001-x.md` stays preamble by design.
fn is_record_dash_bullet(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("- [") else {
        return false;
    };
    let Some((_, after)) = rest.split_once("](") else {
        return false;
    };
    let destination = after.strip_prefix("./").unwrap_or(after);
    let bytes = destination.as_bytes();
    bytes.len() > 4 && bytes[..4].iter().all(u8::is_ascii_digit) && bytes[4] == b'-'
}

/// True for a Markdown table row (`| cell | cell | ... |`) whose first cell
/// is either a numbered-listing header (`| # |`) or a record link
/// (`| [NNNN](...)` / `| [NNNN-...`) — the two shapes a hand-maintained
/// record table uses in place of the generator's bullet format.
pub(crate) fn is_table_listing_row(line: &str) -> bool {
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

pub(super) fn fallback_preamble(existing: &str, doc_type: &str) -> String {
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
