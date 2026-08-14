use super::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;

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

fn briefed_with_title(title: &str) -> String {
    brief_content(
        TEMPLATE,
        "adr",
        "ADR",
        "2026-07-19T00:00:00Z",
        7,
        title,
        None,
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
    assert!(content.contains("title: Choose X\n"));
    assert!(content.contains("# 0007. Choose X\n"));
    assert!(!content.contains("<Short decision title>"));
}

#[test]
fn the_frontmatter_title_is_quoted_exactly_when_the_canonical_serializer_would_quote_it() {
    let content = briefed_with_title("Caching: A Deep Dive");
    assert!(content.contains(&format!(
        "title: {}\n",
        crate::record::format_scalar("Caching: A Deep Dive")
    )));
}

#[test]
fn the_trail_comment_sits_under_the_title_heading() {
    let content = briefed(None);
    assert!(
        content.contains("# 0007. Choose X\n\n<!-- trail: motivated-by /research/NNNN-<slug>.md")
    );
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

#[test]
fn constitution_judgment_sections_collapse_while_structural_sections_stay_intact() {
    let template = doc_type::spec_for("constitution")
        .expect("constitution must be registered")
        .template;
    let content = brief_content(
        template,
        "constitution",
        "Constitution",
        "2026-07-19T00:00:00Z",
        0,
        "Acme Constitution",
        None,
    );

    assert!(content.contains("## Product\n\n<!-- judgment: product -->\n"));
    assert!(content.contains("## Scope Boundaries\n\n<!-- judgment: scope-boundaries -->\n"));
    assert!(content.contains("## Non-negotiables\n\n<!-- judgment: non-negotiables -->\n"));
    assert!(content.contains("erDiagram"));
    assert!(content.contains("ENTITY_A ||--o{ ENTITY_B"));
    assert!(content.contains("<!-- Append amendments here"));
    assert!(!content.contains("<What the product is"));
    assert!(!content.contains("<Capability or domain"));
    assert!(!content.contains("<Non-negotiable 1>"));
}

#[test]
fn constitution_has_no_trail_comment_and_the_empty_trail_does_not_break_the_output() {
    let template = doc_type::spec_for("constitution")
        .expect("constitution must be registered")
        .template;
    let content = brief_content(
        template,
        "constitution",
        "Constitution",
        "2026-07-19T00:00:00Z",
        0,
        "Acme Constitution",
        None,
    );

    assert!(!content.contains("<!-- trail:"));
    assert!(content.contains("# Product Constitution\n"));
}

/// A minimal in-memory [`DocStore`] test double, mirroring the one in
/// `commands::new`'s tests, so `scaffold_brief`'s singleton branch needs
/// no filesystem.
struct MapStore {
    files: RefCell<BTreeMap<PathBuf, String>>,
}

impl MapStore {
    fn new() -> Self {
        Self {
            files: RefCell::new(BTreeMap::new()),
        }
    }

    fn seeded(seed: &[(&str, &str)]) -> Self {
        let files = seed
            .iter()
            .map(|(path, contents)| (PathBuf::from(path), (*contents).to_string()))
            .collect();
        Self {
            files: RefCell::new(files),
        }
    }
}

impl DocStore for MapStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(self
            .files
            .borrow()
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect())
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.files
            .borrow()
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), contents.to_string());
        Ok(())
    }
}

#[test]
fn scaffold_brief_writes_a_singleton_constitution_with_no_number_or_slug() {
    let store = MapStore::new();

    let target = scaffold_brief(
        &store,
        Path::new("/bundle"),
        "constitution",
        "Acme Constitution",
        "2026-07-19T00:00:00Z",
        None,
    )
    .expect("scaffold_brief should succeed");

    assert_eq!(target, PathBuf::from("/bundle/constitution.md"));
    let persisted = store
        .read(&target)
        .expect("scaffold_brief must persist through DocStore::write");
    assert!(persisted.contains("type: Constitution"));
    assert!(persisted.contains("<!-- judgment: product -->"));
}

#[test]
fn scaffold_brief_refuses_a_second_constitution() {
    let store = MapStore::seeded(&[("/bundle/constitution.md", "existing content")]);

    let err = scaffold_brief(
        &store,
        Path::new("/bundle"),
        "constitution",
        "Acme Constitution",
        "2026-07-19T00:00:00Z",
        None,
    )
    .expect_err("a second constitution must be refused");

    assert!(err.contains("already exists"), "got: {err}");
    assert_eq!(
        store.read(Path::new("/bundle/constitution.md")).unwrap(),
        "existing content"
    );
}
