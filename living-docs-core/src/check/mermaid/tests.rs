use super::*;

fn write_temp(label: &str, contents: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("living-docs-mermaid-test-{label}-{nanos}.md"));
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

/// The conformance corpus must parse with the
/// exact same accept/reject verdict the prior parser gave — every valid
/// shape accepted, the broken arrow chain rejected. A parity regression
/// in `merman-core` fails this test, not silently degrades `check`.
#[test]
fn validate_all_matches_the_conformance_corpus() {
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

/// Verified `merman-core` 0.7.0 leniency, reported upstream:
/// https://github.com/Latias94/merman/issues/150. `ParseOptions::strict()`
/// accepts an unlabeled-arrow chain whose bare-word target text splits into
/// two nodes with no edge between them, where mermaid.js rejects the same
/// input ("got 'NODE_STRING'"). Pinned here so a future `merman-core`
/// upgrade that fixes the leniency fails this test, not the other way
/// around: when that happens, flip the assertion to rejected instead of
/// letting the verdict change silently.
#[test]
fn validate_all_accepts_a_known_merman_core_leniency_pending_upstream_fix() {
    let bare_word_target = corpus_diagram(
        "flowchart LR\n    AX -- \"~4.0:1 today\" --> per-story disable --> GAP[blind spot]\n",
    );
    let bracketed_target = corpus_diagram(
        "flowchart LR\n    AX -- \"~4.0:1 today\" --> PS[per-story disable] --> GAP[blind spot]\n",
    );

    let failures = validate_all(&[bare_word_target, bracketed_target]);

    assert!(
        failures.is_empty(),
        "expected merman-core 0.7.0 to still accept both diagrams, but {} failed",
        failures.len()
    );
}

#[test]
fn format_failure_reports_the_no_diagram_detail_verbatim() {
    let failure = Failure {
        file: PathBuf::from("doc.md"),
        start_line: 5,
        detail: "no mermaid diagram recognized in fence body".to_string(),
    };

    let line = format_failure(&failure);

    assert_eq!(
        line,
        "FAIL doc.md:5 — invalid mermaid diagram: no mermaid diagram recognized in fence body"
    );
}

#[test]
fn format_failure_collapses_a_multiline_parser_detail_to_one_line() {
    let failure = Failure {
        file: PathBuf::from("doc.md"),
        start_line: 7,
        detail: "UnrecognizedToken {\n  token: Arrow,\n  expected: [\"EdgeLabel\", \"Id\"]\n}"
            .to_string(),
    };

    let line = format_failure(&failure);

    assert_eq!(
        line,
        "FAIL doc.md:7 — invalid mermaid diagram: UnrecognizedToken { token: Arrow, expected: [\"EdgeLabel\", \"Id\"] }"
    );
    assert!(!line.contains('\n'), "expected one line, got:\n{line}");
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
