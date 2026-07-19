//! Advisory body-size check (issue 0009) — decision/execution records aim for
//! ~100 body lines; past 120 the check prints a `SIZE` note. Advisory only:
//! it never affects the exit code, and long-form evidence types (Research and
//! the non-record docs) are exempt.

use super::{file_name_str, records, Reporter};
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
                format!("SIZE body {lines} lines exceeds the {WARN_LINES}-line advisory target (aim ~{AIM_LINES})"),
            );
        }
    }
}

fn over_target_body_lines(content: &str) -> Option<usize> {
    let doc_type = frontmatter::read_scalar_from_str(content, "type")?;
    if !has_size_target(&doc_type) {
        return None;
    }
    let lines = body_line_count(content);
    (lines > WARN_LINES).then_some(lines)
}

fn has_size_target(doc_type: &str) -> bool {
    matches!(doc_type, "ADR" | "BDR" | "PRD" | "Issue")
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
mod tests {
    use super::*;

    fn doc_with_body_lines(doc_type: &str, body_lines: usize) -> String {
        let body = (0..body_lines)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("---\ntype: {doc_type}\n---\n{body}")
    }

    #[test]
    fn body_line_count_excludes_the_frontmatter_block() {
        assert_eq!(body_line_count("---\ntype: ADR\n---\none\ntwo\n"), 2);
    }

    #[test]
    fn body_line_count_without_frontmatter_counts_every_line() {
        assert_eq!(body_line_count("one\ntwo\nthree\n"), 3);
    }

    #[test]
    fn a_body_at_exactly_the_warn_threshold_is_not_flagged() {
        assert_eq!(
            over_target_body_lines(&doc_with_body_lines("ADR", 120)),
            None
        );
    }

    #[test]
    fn a_body_one_line_over_the_warn_threshold_is_flagged_with_its_count() {
        assert_eq!(
            over_target_body_lines(&doc_with_body_lines("ADR", 121)),
            Some(121)
        );
    }

    #[test]
    fn research_is_exempt_regardless_of_length() {
        assert_eq!(
            over_target_body_lines(&doc_with_body_lines("Research", 400)),
            None
        );
    }

    #[test]
    fn every_decision_and_execution_record_type_carries_the_target() {
        for doc_type in ["ADR", "BDR", "PRD", "Issue"] {
            assert_eq!(
                over_target_body_lines(&doc_with_body_lines(doc_type, 121)),
                Some(121),
                "{doc_type} should carry the size target"
            );
        }
    }
}
