//! `living-docs brief` (issue 0008) — `new` plus deterministic pre-fill: the
//! frontmatter title, the numbered title heading, a trail comment naming the
//! records this type conventionally links, and every judgment section
//! collapsed to a byte-identical `<!-- judgment: <name> -->` marker an agent
//! can locate without re-reading the file. The tool derives facts only — it
//! never writes rationale prose (ADR 0001 determinism boundary).

use crate::commands::new::{
    fill_frontmatter, fill_frontmatter_title, now_iso8601, unsupported_type_message,
};
use crate::commands::next::next_number_from_store;
use crate::doc_type::{self, Identity};
use crate::paths;
use crate::store::DocStore;
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
    let spec = doc_type::spec_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    let (target_path, number) = brief_target_for(store, docs_dir, spec, title)?;

    if store.read(&target_path).is_ok() {
        return Err(format!("{} already exists", target_path.display()));
    }

    let content = brief_content(
        spec.template,
        doc_type,
        spec.frontmatter,
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

/// Resolves `brief`'s target path and heading number for `spec`'s identity
/// shape, mirroring [`crate::commands::new::target_path_for`]: a
/// [`Identity::Numbered`] type allocates the next number and slugifies
/// `title`; a [`Identity::Singleton`] type resolves straight to
/// `docs_dir.join(file)` with no number allocated. The returned `0` for a
/// singleton is never rendered — [`is_title_heading_placeholder`] only
/// recognizes a `# NNNN. <...>` heading, which a singleton template (e.g.
/// `constitution.md`'s plain `# Product Constitution`) does not carry.
fn brief_target_for(
    store: &dyn DocStore,
    docs_dir: &Path,
    spec: &doc_type::DocTypeSpec,
    title: &str,
) -> Result<(PathBuf, u32), String> {
    match spec.identity {
        Identity::Numbered { dir: dir_name } => {
            let number =
                next_number_from_store(store, docs_dir, dir_name).map_err(|e| e.to_string())?;
            let target_path = docs_dir
                .join(dir_name)
                .join(format!("{number:04}-{}.md", paths::slugify(title)));
            Ok((target_path, number))
        }
        Identity::Singleton { file } => Ok((docs_dir.join(file), 0)),
        Identity::Named { dir: dir_name } => Ok((
            docs_dir
                .join(dir_name)
                .join(format!("{}.md", paths::slugify(title))),
            0,
        )),
    }
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
#[allow(clippy::too_many_lines)]
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
        "research" => &[
            ("## Question", "question"),
            ("## Method", "method"),
            ("## Implications", "implications"),
            ("## Open Questions", "open-questions"),
            ("# References", "references"),
        ],
        "constitution" => &[
            ("## Product", "product"),
            ("## Scope Boundaries", "scope-boundaries"),
            ("## Non-negotiables", "non-negotiables"),
        ],
        _ => &[],
    }
}

fn context_marker_for(doc_type: &str) -> &'static str {
    match doc_type {
        "prd" => "problem-motivation",
        "research" => "question",
        "constitution" => "product",
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
        "research" => "<!-- trail: motivates /adr/NNNN-<slug>.md · /prd/NNNN-<slug>.md · tracked-by /issues/NNNN-<slug>.md -->",
        _ => "",
    }
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
mod tests;
