use crate::commands::next::next_number;
use crate::paths;
use crate::templates;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn run(docs_dir: &Path, doc_type: &str, title: &str) -> ExitCode {
    match scaffold(docs_dir, doc_type, title, &now_iso8601()) {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs new: {message}");
            ExitCode::from(2)
        }
    }
}

fn scaffold(
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    timestamp: &str,
) -> Result<PathBuf, String> {
    let dir_name = paths::dir_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    let frontmatter_type = paths::frontmatter_type_for(doc_type)
        .expect("dir_for and frontmatter_type_for cover the same doc types");
    let template = templates::template_for(doc_type)
        .expect("dir_for and template_for cover the same doc types");

    let type_dir = docs_dir.join(dir_name);
    let number = next_number(docs_dir, dir_name).map_err(|e| e.to_string())?;
    let target_path = type_dir.join(format!("{number:04}-{}.md", paths::slugify(title)));

    if target_path.exists() {
        return Err(format!("{} already exists", target_path.display()));
    }

    fs::create_dir_all(&type_dir).map_err(|e| e.to_string())?;
    let filled = fill_frontmatter(template, frontmatter_type, timestamp);
    fs::write(&target_path, filled).map_err(|e| e.to_string())?;
    Ok(target_path)
}

fn unsupported_type_message(doc_type: &str) -> String {
    format!("unsupported doc type '{doc_type}' (expected one of adr, bdr, prd, issue)")
}

/// Targeted line-edit fill of `type`/`status`/`timestamp` inside the leading
/// frontmatter block only — never a serde round-trip, so body placeholders
/// and frontmatter guidance comments outside those three keys survive
/// byte-for-byte.
fn fill_frontmatter(template: &str, type_value: &str, timestamp: &str) -> String {
    let lines: Vec<&str> = template.lines().collect();
    let Some(close) = frontmatter_close_index(&lines) else {
        return template.to_string();
    };

    let filled: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, &line)| {
            if i == 0 || i >= close {
                line.to_string()
            } else {
                fill_frontmatter_line(line, type_value, timestamp)
            }
        })
        .collect();

    filled.join("\n") + "\n"
}

fn frontmatter_close_index(lines: &[&str]) -> Option<usize> {
    lines
        .iter()
        .skip(1)
        .position(|&l| l == "---")
        .map(|i| i + 1)
}

fn fill_frontmatter_line(line: &str, type_value: &str, timestamp: &str) -> String {
    replace_targeted_value(line, "type", type_value)
        .or_else(|| replace_targeted_value(line, "status", "Proposed"))
        .or_else(|| replace_targeted_value(line, "timestamp", timestamp))
        .unwrap_or_else(|| line.to_string())
}

/// Replaces the value of a `key: value` frontmatter line, preserving any
/// trailing `# guidance comment` verbatim.
fn replace_targeted_value(line: &str, key: &str, new_value: &str) -> Option<String> {
    let prefix = format!("{key}:");
    let rest = line.strip_prefix(&prefix)?;
    match rest.find('#') {
        Some(hash_idx) => Some(format!("{prefix} {new_value} {}", &rest[hash_idx..])),
        None => Some(format!("{prefix} {new_value}")),
    }
}

fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the unix epoch")
        .as_secs();
    let (year, month, day) = civil_date_from_unix_days((secs / 86_400) as i64);
    let time_of_day = secs % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        time_of_day / 3_600,
        (time_of_day % 3_600) / 60,
        time_of_day % 60
    )
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
mod tests {
    use super::*;

    #[test]
    fn fill_frontmatter_sets_type_status_and_timestamp() {
        let template = "---\ntype: ADR\nstatus: Proposed            # Proposed | Accepted\ntimestamp: <ISO 8601 datetime>\n---\n\n# Body\n<placeholder>\n";
        let filled = fill_frontmatter(template, "ADR", "2026-07-14T00:00:00Z");

        assert!(filled.contains("type: ADR"));
        assert!(filled.contains("status: Proposed"));
        assert!(filled.contains("timestamp: 2026-07-14T00:00:00Z"));
    }

    #[test]
    fn fill_frontmatter_preserves_the_guidance_comment_verbatim() {
        let template = "---\ntype: ADR\nstatus: Proposed            # Proposed | Accepted | Superseded | Deprecated\ntimestamp: <ISO 8601 datetime>\n---\n\n# Body\n";
        let filled = fill_frontmatter(template, "ADR", "2026-07-14T00:00:00Z");

        assert!(filled.contains("# Proposed | Accepted | Superseded | Deprecated"));
    }

    #[test]
    fn fill_frontmatter_leaves_the_body_untouched() {
        let template = "---\ntype: BDR\nstatus: Draft               # Draft | Accepted\ntimestamp: <ISO 8601 datetime>\n---\n\n<!-- Status lives in frontmatter (`status`), not a body line. -->\n<Replace the diagram above with a flowchart...>\n";
        let filled = fill_frontmatter(template, "BDR", "2026-07-14T00:00:00Z");

        assert!(
            filled.contains("<!-- Status lives in frontmatter (`status`), not a body line. -->")
        );
        assert!(filled.contains("<Replace the diagram above with a flowchart...>"));
    }

    #[test]
    fn fill_frontmatter_without_a_closing_fence_returns_the_template_unchanged() {
        let template = "no frontmatter here\n";
        assert_eq!(
            fill_frontmatter(template, "ADR", "2026-07-14T00:00:00Z"),
            template
        );
    }

    #[test]
    fn civil_date_from_unix_days_matches_known_calendar_dates() {
        assert_eq!(civil_date_from_unix_days(0), (1970, 1, 1));
        assert_eq!(civil_date_from_unix_days(1), (1970, 1, 2));
        assert_eq!(civil_date_from_unix_days(31), (1970, 2, 1));
    }

    #[test]
    fn now_iso8601_has_the_expected_shape() {
        let timestamp = now_iso8601();
        assert_eq!(timestamp.len(), 20);
        assert_eq!(&timestamp[4..5], "-");
        assert_eq!(&timestamp[7..8], "-");
        assert_eq!(&timestamp[10..11], "T");
        assert_eq!(&timestamp[13..14], ":");
        assert_eq!(&timestamp[16..17], ":");
        assert_eq!(&timestamp[19..20], "Z");
    }

    #[test]
    fn unsupported_type_message_names_the_offending_type() {
        assert!(unsupported_type_message("constitution").contains("constitution"));
    }
}
