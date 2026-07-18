//! Mermaid fence validation (ADR 0013) — ports `skills/living-docs/scripts/lint-mermaid.sh`.
//!
//! Extracts every fenced ```mermaid``` block (optional indent, one open line +
//! one close line) and validates each one in-process through `merman-core`'s
//! `Engine::parse_diagram_sync`, the real Mermaid grammar parser — not a
//! hand-rolled check and no external process of any kind.

use super::{collect_md_files, Reporter};
use merman_core::{Engine, ParseOptions};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

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

/// `check --mermaid-only [paths...]` entry point: validates ONLY the mermaid
/// fences under `paths` (default: git-tracked `*.md`, fixtures dir excluded),
/// mirroring `lint-mermaid.sh`'s own default sweep and exit codes.
pub fn run_mermaid_only(paths: &[PathBuf]) -> ExitCode {
    let files = discover_files(paths);
    match check(&files) {
        Outcome::NoFences => {
            println!("\nOK: 0 diagram(s) across 0 file(s).");
            ExitCode::SUCCESS
        }
        Outcome::Checked {
            diagram_count,
            file_count,
            failures,
        } => {
            print_failures(&failures);
            println!();
            if failures.is_empty() {
                println!("OK: {diagram_count} diagram(s) across {file_count} file(s).");
                ExitCode::SUCCESS
            } else {
                println!(
                    "FAIL: {} of {diagram_count} diagram(s) failed to parse.",
                    failures.len()
                );
                ExitCode::from(1)
            }
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
        reporter.report(
            &f.file,
            format!(
                "FAIL {}:{} — invalid mermaid diagram",
                f.file.display(),
                f.start_line
            ),
        );
    }
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

fn print_failures(failures: &[Failure]) {
    for f in failures {
        println!("FAIL {}:{}", f.file.display(), f.start_line);
        for line in f.detail.lines().take(5) {
            println!("    {line}");
        }
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

/// Validates every diagram against one shared `Engine`, matching ADR 0013:
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
mod tests {
    use super::*;

    fn write_temp(label: &str, contents: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("living-docs-mermaid-test-{label}-{nanos}.md"));
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn extract_diagrams_captures_multiple_fences_with_body_and_start_line() {
        let path = write_temp(
            "multi",
            "# Doc\n\n```mermaid\nflowchart TD\n  A --> B\n```\n\nMore text.\n\n```mermaid\nerDiagram\n  A ||--o{ B : has\n```\n",
        );
        let diagrams = extract_diagrams(std::slice::from_ref(&path));

        assert_eq!(diagrams.len(), 2);
        assert_eq!(diagrams[0].start_line, 3);
        assert_eq!(diagrams[0].body, "flowchart TD\n  A --> B\n");
        assert_eq!(diagrams[1].start_line, 10);
        assert_eq!(diagrams[1].body, "erDiagram\n  A ||--o{ B : has\n");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn extract_diagrams_honors_indented_fence_lines() {
        let path = write_temp(
            "indented",
            "- item\n  ```mermaid\n  flowchart TD\n    A --> B\n  ```\n",
        );
        let diagrams = extract_diagrams(std::slice::from_ref(&path));

        assert_eq!(diagrams.len(), 1);
        assert_eq!(diagrams[0].start_line, 2);
        assert_eq!(diagrams[0].body, "  flowchart TD\n    A --> B\n");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn extract_diagrams_drops_an_unterminated_block_at_eof() {
        let path = write_temp("unterminated", "```mermaid\nflowchart TD\n  A --> B\n");
        let diagrams = extract_diagrams(std::slice::from_ref(&path));

        assert!(diagrams.is_empty());

        let _ = fs::remove_file(&path);
    }

    fn corpus_diagram(body: &str) -> Diagram {
        Diagram {
            file: PathBuf::from("corpus.md"),
            start_line: 1,
            body: body.to_string(),
        }
    }

    /// ADR 0013 fitness function: the conformance corpus must parse with the
    /// exact same accept/reject verdict the prior parser gave — every valid
    /// shape accepted, the broken arrow chain rejected. A parity regression
    /// in `merman-core` fails this test, not silently degrades `check`.
    #[test]
    fn validate_all_matches_the_adr_0013_conformance_corpus() {
        let valid = [
            "flowchart TD\n  A[Start] --> B{Decision}\n  B -->|Yes| C[Do the thing]\n  B -->|No| D[Skip it]\n",
            "flowchart LR\n  User -->|shortens| App\n  App -->|redirects| User\n",
            "erDiagram\n  CUSTOMER ||--o{ ORDER : places\n  ORDER ||--|{ LINE_ITEM : contains\n",
            "  flowchart TD\n    A --> B\n",
        ]
        .map(corpus_diagram);
        let failures = validate_all(&valid);
        assert!(
            failures.is_empty(),
            "expected every valid corpus diagram to parse, but {} failed",
            failures.len()
        );

        let invalid = [corpus_diagram("flowchart TD\nA --> --> B\n")];
        let failures = validate_all(&invalid);
        assert_eq!(
            failures.len(),
            1,
            "expected the broken arrow chain to be rejected"
        );
    }

    #[test]
    fn discover_files_with_an_explicit_directory_finds_its_markdown_files() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("living-docs-mermaid-discover-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("doc.md"), "# Doc\n").unwrap();
        fs::write(dir.join("not-md.txt"), "ignored\n").unwrap();

        let files = discover_files(std::slice::from_ref(&dir));

        assert_eq!(files, vec![dir.join("doc.md")]);

        let _ = fs::remove_dir_all(&dir);
    }
}
