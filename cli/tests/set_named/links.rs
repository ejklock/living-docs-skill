use super::*;

#[test]
fn a_view_and_the_constitution_take_a_section_by_name() {
    let docs = view_bundle("section");
    assert!(run_new(&docs, "constitution", "Product").status.success());
    let source = docs.parent().unwrap().join("section.txt");
    fs::write(&source, "New text.\n").unwrap();
    let from = ["section", "Purpose", "--from", source.to_str().unwrap()];
    for (reference, file) in [
        ("view/rings", "architecture/rings.md"),
        ("constitution", "constitution.md"),
    ] {
        let out = set(&docs, &[&[reference][..], &from[..]].concat());
        assert!(out.status.success(), "{reference}: {out:?}");
        let text = read(&docs, file);
        assert!(text.contains("## Purpose\n\nNew text."), "{reference}");
    }
    for reference in ["view/nope", "view/../adr/0001"] {
        assert_refused_unchanged(&docs, &[&[reference][..], &from[..]].concat());
    }
}

/// Writes `texts.0` to `file`, retitles the view `reference` to "Charter", and
/// asserts the file now reads `texts.1` and `check` finds no broken link.
fn assert_retitle_rewrites(docs: &Path, reference: &str, file: &str, texts: (&str, &str)) {
    write_doc(docs, file, texts.0);
    let out = set(docs, &[reference, "title", "Charter"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(read(docs, file), texts.1);
    let found = findings(docs);
    assert!(!found.contains("broken link"), "{found}");
}

fn shared_name_bundle(label: &str) -> PathBuf {
    let docs = temp_bundle("set-named", label);
    write_doc(&docs, "constitution.md", "# Root\n");
    new_view(&docs, "Constitution", "flow");
    docs
}

#[test]
fn a_shared_name_retitle_leaves_prose_and_link_titles_alone() {
    let docs = shared_name_bundle("prose-and-titles");
    let before = "[v](constitution.md) and [r](../constitution.md) prose the root constitution.md stays, title [t](constitution.md \"about constitution.md\")\n";
    let after = before
        .replace("[v](constitution", "[v](charter")
        .replace("[t](constitution", "[t](charter");
    let target = "view/constitution";
    assert_retitle_rewrites(&docs, target, "architecture/side.md", (before, &after));
}

#[test]
fn a_shared_name_retitle_rewrites_reference_definitions_and_angle_destinations() {
    let docs = shared_name_bundle("definitions");
    let before = "[x][r2] and [a](<architecture/constitution.md#top>) and [n](./architecture/constitution.md)\n\n[r2]:architecture/constitution.md\n";
    let after = before.replace("constitution.md", "charter.md");
    assert_retitle_rewrites(&docs, "view/constitution", "notes.md", (before, &after));
}

#[test]
fn a_shared_name_retitle_rewrites_spaced_definitions_and_images_inside_links() {
    let docs = shared_name_bundle("spaced-and-images");
    fs::write(docs.join("o.png"), "png").unwrap();
    let before = "[x][r] and [![i](o.png)](architecture/constitution.md)\n\n[r]: architecture/constitution.md\n";
    let after = before.replace("constitution.md", "charter.md");
    assert_retitle_rewrites(&docs, "view/constitution", "notes.md", (before, &after));
}
