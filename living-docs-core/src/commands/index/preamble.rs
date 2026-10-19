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
/// whichever comes first — letting a legacy `## Done`/`## Open` index or a
/// hand-maintained table migrate in a single pass rather than
/// a silent append below it.
pub(super) fn is_boundary_line(line: &str) -> bool {
    line.starts_with("## ") || line.starts_with("* [") || is_table_listing_row(line)
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
