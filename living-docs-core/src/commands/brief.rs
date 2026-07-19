//! `living-docs brief` (issue 0008) — `new` plus deterministic pre-fill: the
//! frontmatter title, the numbered title heading, a trail comment naming the
//! records this type conventionally links, and every judgment section
//! collapsed to a byte-identical `<!-- judgment: <name> -->` marker an agent
//! can locate without re-reading the file. The tool derives facts only — it
//! never writes rationale prose (ADR 0001 determinism boundary).

use crate::commands::new::{
    fill_frontmatter, frontmatter_close_index, now_iso8601, replace_targeted_value,
    unsupported_type_message,
};
use crate::commands::next::next_number_from_store;
use crate::paths;
use crate::store::DocStore;
use crate::templates;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The files a git range touched, resolved by the CLI front (`git diff
/// --name-only <range>`) so the core stays I/O-free.
pub struct DiffContext {
    pub range: String,
    pub files: Vec<String>,
}

pub fn run(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    diff: Option<&DiffContext>,
) -> ExitCode {
    match scaffold_brief(store, docs_dir, doc_type, title, &now_iso8601(), diff) {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs brief: {message}");
            ExitCode::from(2)
        }
    }
}

fn scaffold_brief(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    timestamp: &str,
    diff: Option<&DiffContext>,
) -> Result<PathBuf, String> {
    let dir_name = paths::dir_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    let frontmatter_type = paths::frontmatter_type_for(doc_type)
        .expect("dir_for and frontmatter_type_for cover the same doc types");
    let template = templates::template_for(doc_type)
        .expect("dir_for and template_for cover the same doc types");

    let number = next_number_from_store(store, docs_dir, dir_name).map_err(|e| e.to_string())?;
    let target_path = docs_dir
        .join(dir_name)
        .join(format!("{number:04}-{}.md", paths::slugify(title)));

    if store.read(&target_path).is_ok() {
        return Err(format!("{} already exists", target_path.display()));
    }

    let content = brief_content(
        template,
        doc_type,
        frontmatter_type,
        timestamp,
        number,
        title,
        diff,
    );
    store
        .write(&target_path, &content)
        .map_err(|e| e.to_string())?;
    Ok(target_path)
}

fn brief_content(
    template: &str,
    doc_type: &str,
    frontmatter_type: &str,
    timestamp: &str,
    number: u32,
    title: &str,
    diff: Option<&DiffContext>,
) -> String {
    let filled = fill_frontmatter(template, frontmatter_type, timestamp);
    let titled = fill_frontmatter_title(&filled, title);
    let slotted = replace_judgment_sections(&titled, slots_for(doc_type));
    let headed = fill_title_heading(&slotted, doc_type, number, title);
    match diff {
        Some(d) if !d.files.is_empty() => {
            insert_touched_files(&headed, context_marker_for(doc_type), d)
        }
        _ => headed,
    }
}

/// Judgment sections per doc type: heading line → marker name. Everything a
/// slot heading opens (until the next heading) is judgment the authoring
/// model owns; the structural sections (BDR Behavior/Contract/Test Design,
/// PRD NFR table, ADR Verification) keep their template scaffolding.
fn slots_for(doc_type: &str) -> &'static [(&'static str, &'static str)] {
    match doc_type {
        "adr" => &[
            ("## Context", "context"),
            ("## Decision", "decision"),
            ("## Consequences", "consequences"),
            ("# References", "references"),
        ],
        "bdr" => &[
            ("## Context", "context"),
            ("## Textual Description", "textual-description"),
            ("## Scenarios", "scenarios"),
            ("## Related", "related"),
        ],
        "prd" => &[
            ("## Problem / Motivation", "problem-motivation"),
            ("## Goals", "goals"),
            ("## Non-goals", "non-goals"),
            ("## Requirements", "requirements"),
            ("## Acceptance criteria", "acceptance-criteria"),
            ("## Success metrics", "success-metrics"),
            ("## Behavior (BDRs)", "behavior-bdrs"),
            ("## Open questions", "open-questions"),
            ("## Decision log", "decision-log"),
            ("## Related", "related"),
        ],
        "issue" => &[
            ("## <Issue title>", "context"),
            ("### Scope", "scope"),
            ("### Acceptance", "acceptance"),
            ("### Plan", "plan"),
        ],
        _ => &[],
    }
}

fn context_marker_for(doc_type: &str) -> &'static str {
    match doc_type {
        "prd" => "problem-motivation",
        _ => "context",
    }
}

/// Trail stubs live inside a comment so an unfilled scaffold carries no
/// dangling markdown links — `check` stays green on the raw `brief` output.
fn trail_comment_for(doc_type: &str) -> &'static str {
    match doc_type {
        "adr" => "<!-- trail: motivated-by /research/NNNN-<slug>.md · /prd/NNNN-<slug>.md · tracked-by /issues/NNNN-<slug>.md -->",
        "bdr" => "<!-- trail: spawned-by /prd/NNNN-<slug>.md · /adr/NNNN-<slug>.md · tracked-by /issues/NNNN-<slug>.md -->",
        "prd" => "<!-- trail: constitution /constitution.md · behavior /bdr/NNNN-<slug>.md · tracked-by /issues/NNNN-<slug>.md -->",
        "issue" => "<!-- trail: implements /adr/NNNN-<slug>.md · part-of /prd/NNNN-<slug>.md -->",
        _ => "",
    }
}

fn fill_frontmatter_title(content: &str, title: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let Some(close) = frontmatter_close_index(&lines) else {
        return content.to_string();
    };

    let filled: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, &line)| {
            if i == 0 || i >= close {
                line.to_string()
            } else {
                replace_targeted_value(line, "title", &yaml_quote(title))
                    .unwrap_or_else(|| line.to_string())
            }
        })
        .collect();

    filled.join("\n") + "\n"
}

fn yaml_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn replace_judgment_sections(content: &str, slots: &[(&str, &str)]) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        out.push(lines[i].to_string());
        let Some(marker) = marker_for_heading(lines[i], slots) else {
            i += 1;
            continue;
        };
        out.push(String::new());
        out.push(format!("<!-- judgment: {marker} -->"));
        i = next_heading_index(&lines, i + 1);
        if i < lines.len() {
            out.push(String::new());
        }
    }
    out.join("\n") + "\n"
}

fn marker_for_heading<'a>(line: &str, slots: &[(&str, &'a str)]) -> Option<&'a str> {
    slots
        .iter()
        .find(|(heading, _)| *heading == line)
        .map(|(_, marker)| *marker)
}

fn next_heading_index(lines: &[&str], from: usize) -> usize {
    (from..lines.len())
        .find(|&i| lines[i].starts_with('#'))
        .unwrap_or(lines.len())
}

fn fill_title_heading(content: &str, doc_type: &str, number: u32, title: &str) -> String {
    let filled: Vec<String> = content
        .lines()
        .map(|line| {
            if is_title_heading_placeholder(line, doc_type) {
                filled_heading_with_trail(doc_type, number, title)
            } else {
                line.to_string()
            }
        })
        .collect();
    filled.join("\n") + "\n"
}

fn is_title_heading_placeholder(line: &str, doc_type: &str) -> bool {
    match doc_type {
        "issue" => line == "## <Issue title>",
        _ => line.starts_with("# NNNN. <"),
    }
}

fn filled_heading_with_trail(doc_type: &str, number: u32, title: &str) -> String {
    let heading = match doc_type {
        "issue" => format!("## {title}"),
        _ => format!("# {number:04}. {title}"),
    };
    format!("{heading}\n\n{}", trail_comment_for(doc_type))
}

fn insert_touched_files(content: &str, context_marker: &str, diff: &DiffContext) -> String {
    let marker_line = format!("<!-- judgment: {context_marker} -->");
    let mut out: Vec<String> = Vec::new();
    for line in content.lines() {
        out.push(line.to_string());
        if line == marker_line {
            out.push(String::new());
            out.push(format!(
                "Touched files (`git diff --name-only {}`):",
                diff.range
            ));
            out.push(String::new());
            out.extend(diff.files.iter().map(|file| format!("- `{file}`")));
        }
    }
    out.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str = "---\ntype: ADR\ntitle: <Short decision title>\nstatus: Proposed\ntimestamp: <ISO 8601 datetime>\n---\n\n# NNNN. <Short decision title>\n\n## Context\n\n<guidance with a [link](/research/NNNN-<slug>.md)>\n\n## Decision\n\nWe will <the choice>.\n\n## Consequences\n\n- <what this unlocks>\n\n# References\n\n[1] [<source>](<url>)\n";

    fn briefed(diff: Option<&DiffContext>) -> String {
        brief_content(
            TEMPLATE,
            "adr",
            "ADR",
            "2026-07-19T00:00:00Z",
            7,
            "Choose X",
            diff,
        )
    }

    #[test]
    fn every_judgment_section_collapses_to_exactly_its_marker() {
        let content = briefed(None);
        assert!(content.contains("## Context\n\n<!-- judgment: context -->\n"));
        assert!(content.contains("## Decision\n\n<!-- judgment: decision -->\n"));
        assert!(content.contains("## Consequences\n\n<!-- judgment: consequences -->\n"));
        assert!(content.contains("# References\n\n<!-- judgment: references -->\n"));
        assert!(!content.contains("We will"));
        assert!(!content.contains("guidance with"));
    }

    #[test]
    fn the_frontmatter_title_and_the_numbered_heading_are_filled() {
        let content = briefed(None);
        assert!(content.contains("title: \"Choose X\""));
        assert!(content.contains("# 0007. Choose X\n"));
        assert!(!content.contains("<Short decision title>"));
    }

    #[test]
    fn the_trail_comment_sits_under_the_title_heading() {
        let content = briefed(None);
        assert!(content
            .contains("# 0007. Choose X\n\n<!-- trail: motivated-by /research/NNNN-<slug>.md"));
    }

    #[test]
    fn touched_files_land_verbatim_under_the_context_marker() {
        let diff = DiffContext {
            range: "HEAD~1..HEAD".to_string(),
            files: vec!["src/a.rs".to_string(), "docs/b.md".to_string()],
        };
        let content = briefed(Some(&diff));
        assert!(content.contains(
            "<!-- judgment: context -->\n\nTouched files (`git diff --name-only HEAD~1..HEAD`):\n\n- `src/a.rs`\n- `docs/b.md`"
        ));
    }

    #[test]
    fn an_empty_diff_inserts_nothing() {
        let diff = DiffContext {
            range: "HEAD~1..HEAD".to_string(),
            files: Vec::new(),
        };
        assert_eq!(briefed(Some(&diff)), briefed(None));
    }

    #[test]
    fn yaml_quote_escapes_backslashes_and_quotes() {
        assert_eq!(yaml_quote(r#"a "b" \c"#), r#""a \"b\" \\c""#);
    }

    #[test]
    fn the_issue_intro_heading_is_both_a_slot_and_the_filled_title() {
        let template = "---\ntype: Issue\ntitle: <Issue title>\nstatus: open\ntimestamp: <ISO 8601 datetime>\n---\n\n## <Issue title>\n\n<intro guidance>\n\n### Scope\n\n<scope guidance>\n";
        let content = brief_content(
            template,
            "issue",
            "Issue",
            "2026-07-19T00:00:00Z",
            3,
            "Fix It",
            None,
        );
        assert!(content.contains("## Fix It\n\n<!-- trail: implements"));
        assert!(content.contains("<!-- judgment: context -->"));
        assert!(content.contains("### Scope\n\n<!-- judgment: scope -->"));
        assert!(!content.contains("intro guidance"));
    }
}
