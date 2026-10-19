//! Advisory body-size check (issue 0009) — decision/execution records aim for
//! ~100 body lines; past 120 the check prints a per-record `SIZE` note.
//! Advisory only: it never affects the exit code. Which doc types the line
//! target applies to is decided per-row by `doc_type::DocTypeSpec::body_size`
//! (ADR 0027), not by this module.

use super::{file_name_str, kinds, records, Reporter};
use crate::doc_type::{self, BodySize};
use crate::frontmatter;
use crate::store::DocStore;
use std::path::PathBuf;

const AIM_LINES: usize = 100;
const WARN_LINES: usize = 120;

pub(crate) fn check_body_size(store: &dyn DocStore, all_md: &[PathBuf], reporter: &mut Reporter) {
    for f in all_md {
        if records::is_reserved(&file_name_str(f)) {
            continue;
        }
        let Ok(content) = store.read(f) else {
            continue;
        };
        if let Some(lines) = over_target_body_lines(&content) {
            reporter.advise(
                f,
                kinds::SIZE,
                format!("SIZE body {lines} lines exceeds the {WARN_LINES}-line advisory target (aim ~{AIM_LINES})"),
            );
        }
    }
}

fn over_target_body_lines(content: &str) -> Option<usize> {
    let doc_type = frontmatter::read_scalar_from_str(content, "type")?;
    let spec = doc_type::spec_for_frontmatter(&doc_type)?;
    if is_retired(content, spec) {
        return None;
    }
    (spec.body_size == BodySize::Targeted)
        .then(|| body_line_count(content))
        .filter(|lines| *lines > WARN_LINES)
}

/// Whether `content`'s own `status` frontmatter, judged against `spec`,
/// marks the record retired — the shared [`doc_type::DocTypeSpec::is_retired`]
/// predicate (ADR 0063). Absent `status`, the record is judged live.
fn is_retired(content: &str, spec: &doc_type::DocTypeSpec) -> bool {
    frontmatter::read_scalar_from_str(content, "status")
        .is_some_and(|status| spec.is_retired(&status))
}

fn body_line_count(content: &str) -> usize {
    let lines: Vec<&str> = content.lines().collect();
    lines.len() - body_start_index(&lines)
}

fn body_start_index(lines: &[&str]) -> usize {
    if lines.first() != Some(&"---") {
        return 0;
    }
    lines
        .iter()
        .skip(1)
        .position(|&l| l == "---")
        .map_or(0, |close| close + 2)
}

#[cfg(test)]
mod tests;
