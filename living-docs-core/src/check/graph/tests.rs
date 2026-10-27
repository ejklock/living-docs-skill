use super::*;

#[test]
fn normpath_collapses_dot_and_dotdot_segments() {
    assert_eq!(normpath("docs/./a.md"), "docs/a.md");
    assert_eq!(
        normpath("docs/tables/../datasets/index.md"),
        "docs/datasets/index.md"
    );
    assert_eq!(normpath("/docs/../index.md"), "/index.md");
}

#[test]
fn resolve_link_skips_external_and_anchor_only_targets() {
    assert_eq!(
        resolve_link("docs/index.md", "https://example.com/x", "docs"),
        None
    );
    assert_eq!(resolve_link("docs/index.md", "#section", "docs"), None);
    assert_eq!(
        resolve_link("docs/index.md", "mailto:a@b.com", "docs"),
        None
    );
}

#[test]
fn resolve_link_resolves_bundle_relative_and_file_relative_targets() {
    assert_eq!(
        resolve_link("docs/a/index.md", "/b/index.md", "docs"),
        Some("docs/b/index.md".to_string())
    );
    assert_eq!(
        resolve_link("docs/a/index.md", "./c.md", "docs"),
        Some("docs/a/c.md".to_string())
    );
}

#[test]
fn links_in_excludes_targets_inside_fenced_code_blocks() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("living-docs-graph-test-{nanos}.md"));
    fs::write(&path, "[live](live.md)\n```\n[fenced](fenced.md)\n```\n").unwrap();

    assert_eq!(links_in(&path), vec!["live.md".to_string()]);

    let _ = fs::remove_file(&path);
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(files: &[(&str, &str)]) -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "living-docs-membership-{}-{serial}",
            std::process::id()
        ));
        for (rel, contents) in files {
            let path = root.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
        Self(root)
    }

    fn membership(&self, rels: &[&str]) -> Vec<(String, String)> {
        let all_md: Vec<PathBuf> = rels.iter().map(|rel| self.0.join(rel)).collect();
        let mut reporter = Reporter::new();
        check_directory_membership(&self.0, &all_md, &mut reporter);
        reporter
            .violations
            .into_iter()
            .map(|v| (v.file, v.message))
            .collect()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_record_listed_by_relative_dot_or_bundle_absolute_link_is_not_reported() {
    let scratch = Scratch::new(&[
        (
            "adr/index.md",
            "- [a](a.md)\n- [b](./b.md)\n- [c](/adr/c.md)\n",
        ),
        ("adr/a.md", "x"),
        ("adr/b.md", "x"),
        ("adr/c.md", "x"),
    ]);

    assert!(scratch
        .membership(&["adr/a.md", "adr/b.md", "adr/c.md"])
        .is_empty());
}

#[test]
fn a_record_named_only_inside_a_fenced_block_is_reported_not_listed() {
    let scratch = Scratch::new(&[("adr/index.md", "```\n[a](a.md)\n```\n"), ("adr/a.md", "x")]);

    let found = scratch.membership(&["adr/a.md"]);

    assert_eq!(found.len(), 1);
    assert!(found[0].0.ends_with("adr/a.md"));
    assert!(found[0].1.starts_with("not listed in "));
}

#[test]
fn only_the_unlisted_record_of_a_directory_is_reported() {
    let scratch = Scratch::new(&[
        ("adr/index.md", "- [a](a.md)\n"),
        ("adr/a.md", "x"),
        ("adr/b.md", "x"),
    ]);

    let found = scratch.membership(&["adr/a.md", "adr/b.md"]);

    assert_eq!(found.len(), 1);
    assert!(found[0].0.ends_with("adr/b.md"));
}

#[test]
fn every_record_of_a_directory_without_an_index_gets_its_own_report() {
    let scratch = Scratch::new(&[("adr/a.md", "x"), ("adr/b.md", "x")]);

    let found = scratch.membership(&["adr/a.md", "adr/b.md"]);

    assert_eq!(found.len(), 2);
    assert!(found
        .iter()
        .all(|(_, message)| message.starts_with("no index.md in its directory (")));
}

#[test]
fn reserved_files_are_exempt_from_membership() {
    let scratch = Scratch::new(&[("adr/index.md", "nothing"), ("adr/log.md", "x")]);

    assert!(scratch
        .membership(&["adr/index.md", "adr/log.md"])
        .is_empty());
}

#[test]
fn membership_is_judged_per_directory_in_both_directions() {
    let adr_listed = Scratch::new(&[
        ("adr/index.md", "- [a](0001-a.md)\n"),
        ("adr/0001-a.md", "x"),
        ("issues/index.md", "nothing"),
        ("issues/0001-b.md", "x"),
    ]);
    let issues_listed = Scratch::new(&[
        ("adr/index.md", "nothing"),
        ("adr/0001-a.md", "x"),
        ("issues/index.md", "- [b](0001-b.md)\n"),
        ("issues/0001-b.md", "x"),
    ]);
    let rels = ["adr/0001-a.md", "issues/0001-b.md"];

    let missing_issue = adr_listed.membership(&rels);
    let missing_adr = issues_listed.membership(&rels);

    assert_eq!(missing_issue.len(), 1, "adr listed: {missing_issue:?}");
    assert!(missing_issue[0].0.ends_with("issues/0001-b.md"));
    assert_eq!(missing_adr.len(), 1, "issues listed: {missing_adr:?}");
    assert!(missing_adr[0].0.ends_with("adr/0001-a.md"));
}
