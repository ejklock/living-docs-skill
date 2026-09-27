//! Mermaid fence validation — ports `skills/living-docs/scripts/lint-mermaid.sh`.
//!
//! Extracts every fenced ```mermaid``` block (optional indent, one open line +
//! one close line) and validates each one in-process through `merman-core`'s
//! `Engine::parse_diagram_sync`, the real Mermaid grammar parser — not a
//! hand-rolled check and no external process of any kind. Parity with
//! mermaid.js is bounded by the conformance corpus exercised in tests, not
//! unbounded: a known parser leniency outside that corpus is pinned in its
//! own test instead of being silently accepted here.

use super::{collect_md_files, Reporter};
use merman_core::{Engine, ParseOptions};
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FIXTURES_PATHSPEC: &str = ":!skills/living-docs/tests/fixtures";

pub(crate) struct Diagram {
    file: PathBuf,
    start_line: usize,
    body: String,
}

struct Failure {
    file: PathBuf,
    start_line: usize,
    detail: String,
}

enum Outcome {
    NoFences,
    Checked {
        diagram_count: usize,
        file_count: usize,
        failures: Vec<Failure>,
    },
}

/// `check --mermaid-only`'s findings: one entry per diagram that
/// failed to parse, plus the totals a renderer needs for either its text or
/// JSON shape. Never printed here — the CLI front renders it (`compile`'s
/// own doc comment explains why).
#[derive(Debug, Serialize)]
pub struct MermaidReport {
    pub ok: bool,
    pub diagrams: usize,
    pub files: usize,
    pub errors: Vec<MermaidError>,
}

#[derive(Debug, Serialize)]
pub struct MermaidError {
    pub file: String,
    pub line: usize,
    pub message: String,
}

/// `check --mermaid-only [paths...]` entry point: validates ONLY the mermaid
/// fences under `paths` (default: git-tracked `*.md`, fixtures dir excluded),
/// mirroring `lint-mermaid.sh`'s own default sweep — without printing or
/// choosing an exit code, so the CLI front can render it as colored text or
/// JSON.
pub fn compile(paths: &[PathBuf]) -> MermaidReport {
    let files = discover_files(paths);
    match check(&files) {
        Outcome::NoFences => MermaidReport {
            ok: true,
            diagrams: 0,
            files: 0,
            errors: Vec::new(),
        },
        Outcome::Checked {
            diagram_count,
            file_count,
            failures,
        } => MermaidReport {
            ok: failures.is_empty(),
            diagrams: diagram_count,
            files: file_count,
            errors: failures.into_iter().map(MermaidError::from).collect(),
        },
    }
}

impl From<Failure> for MermaidError {
    fn from(failure: Failure) -> Self {
        Self {
            file: failure.file.display().to_string(),
            line: failure.start_line,
            message: failure.detail,
        }
    }
}

/// Wires mermaid validation into a full `check <bundle>` run: reports every
/// invalid diagram into `reporter`. A bundle with no fences reports nothing.
pub(crate) fn check_bundle(all_md: &[PathBuf], reporter: &mut Reporter) {
    let Outcome::Checked { failures, .. } = check(all_md) else {
        return;
    };
    for f in &failures {
        reporter.report(&f.file, format_failure(f));
    }
}

/// Renders one failed diagram as the single line `check` prints: the
/// file:line pointer plus the parser's own detail (or the no-diagram note),
/// with every whitespace run in the detail collapsed to one space so a
/// multi-line parser message still prints on one line.
fn format_failure(failure: &Failure) -> String {
    format!(
        "FAIL {}:{} — invalid mermaid diagram: {}",
        failure.file.display(),
        failure.start_line,
        collapse_whitespace(&failure.detail)
    )
}

fn collapse_whitespace(detail: &str) -> String {
    detail.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn check(files: &[PathBuf]) -> Outcome {
    let diagrams = extract_diagrams(files);
    if diagrams.is_empty() {
        return Outcome::NoFences;
    }
    let file_count = diagrams
        .iter()
        .map(|d| &d.file)
        .collect::<HashSet<_>>()
        .len();
    let diagram_count = diagrams.len();
    let failures = validate_all(&diagrams);
    Outcome::Checked {
        diagram_count,
        file_count,
        failures,
    }
}

fn discover_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    if paths.is_empty() {
        return default_sweep();
    }
    let mut out = Vec::new();
    for p in paths {
        collect_path(p, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

fn collect_path(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        out.extend(collect_md_files(path));
    } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
        out.push(path.to_path_buf());
    }
}

/// Default sweep: git-tracked `*.md` across the repo, excluding the hostile
/// fixtures dir (11-mermaid-invalid is intentionally broken; tests/run.sh
/// already covers it via an explicit path). Falls back to a plain directory
/// walk from `.` outside a git repo.
fn default_sweep() -> Vec<PathBuf> {
    git_tracked_markdown().unwrap_or_else(|| collect_md_files(Path::new(".")))
}

fn git_tracked_markdown() -> Option<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["ls-files", "-z", "--", "*.md", FIXTURES_PATHSPEC])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        output
            .stdout
            .split(|&b| b == 0)
            .filter(|chunk| !chunk.is_empty())
            .map(|chunk| PathBuf::from(String::from_utf8_lossy(chunk).into_owned()))
            .collect(),
    )
}

/// Awk-equivalent state machine: an optional-indent ```mermaid``` line opens a
/// block, the next optional-indent ``` line closes it. An unterminated block
/// at EOF yields no diagram, matching `lint-mermaid.sh`.
pub(crate) fn extract_diagrams(files: &[PathBuf]) -> Vec<Diagram> {
    files.iter().flat_map(|f| extract_from_file(f)).collect()
}

fn extract_from_file(file: &Path) -> Vec<Diagram> {
    let Ok(content) = fs::read_to_string(file) else {
        return Vec::new();
    };
    let mut diagrams = Vec::new();
    let mut block: Option<(usize, String)> = None;
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        match &mut block {
            None if trimmed == "```mermaid" => block = Some((i + 1, String::new())),
            Some((start_line, buffer)) if trimmed == "```" => {
                diagrams.push(Diagram {
                    file: file.to_path_buf(),
                    start_line: *start_line,
                    body: std::mem::take(buffer),
                });
                block = None;
            }
            Some((_, buffer)) => {
                buffer.push_str(line);
                buffer.push('\n');
            }
            None => {}
        }
    }
    diagrams
}

/// Validates every diagram against one shared `Engine`:
/// `Ok(Some(_))` passes, `Err(_)`/`Ok(None)` produce a `Failure` carrying the
/// parser's own error (or a no-diagram note) as `detail`.
fn validate_all(diagrams: &[Diagram]) -> Vec<Failure> {
    let engine = Engine::new();
    diagrams
        .iter()
        .filter_map(|d| validate_one(&engine, d))
        .collect()
}

fn validate_one(engine: &Engine, diagram: &Diagram) -> Option<Failure> {
    let detail = match engine.parse_diagram_sync(&diagram.body, ParseOptions::strict()) {
        Ok(Some(_)) => return None,
        Ok(None) => "no mermaid diagram recognized in fence body".to_string(),
        Err(err) => err.to_string(),
    };
    Some(Failure {
        file: diagram.file.clone(),
        start_line: diagram.start_line,
        detail,
    })
}

#[cfg(test)]
mod tests;
