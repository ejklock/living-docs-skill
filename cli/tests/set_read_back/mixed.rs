use super::crlf::{tree, Tree};
use super::*;

/// ADRs One, Two and Three in a fresh bundle, the second with mixed endings.
fn three_adrs(label: &str) -> Scratch {
    let docs = temp_bundle("set-read-back", label);
    for title in ["One", "Two", "Three"] {
        assert!(run_new(&docs, "adr", title).status.success());
    }
    let two = docs.join("adr/0002-two.md");
    let text = fs::read_to_string(&two).unwrap();
    fs::write(&two, text.replacen('\n', "\r\n", 1)).unwrap();
    docs
}

fn file(docs: &Path, rel: &str) -> String {
    fs::read_to_string(docs.join(rel)).unwrap()
}

#[test]
fn a_mixed_target_is_refused_naming_the_endings_with_every_file_untouched() {
    let scratch = Scratch::new("living-docs-mixed-section");
    let section = scratch.join("section.txt");
    fs::write(&section, "Added text.\n").unwrap();
    let from = section.to_str().unwrap();
    for args in [
        &["set", "0002", "section", "Notes", "--from", from][..],
        &["set", "0002", "description", "x"][..],
        &["set", "0002", "title", "Renamed"][..],
        &["supersede", "0002", "0001"][..],
        &["supersede", "0001", "0002"][..],
    ] {
        let docs = three_adrs("mixed-target");
        let before: Tree = tree(&docs);
        let out = run(&docs, args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("mixed line endings"), "got: {stderr}");
        assert_eq!(tree(&docs), before, "{args:?}");
    }
}

#[test]
fn a_mixed_sibling_keeps_its_index_row_and_its_bytes_through_set_and_supersede() {
    for args in [
        &["set", "0001", "status", "Accepted"][..],
        &["supersede", "0001", "0003"][..],
    ] {
        let docs = three_adrs("mixed-sibling");
        let two = file(&docs, "adr/0002-two.md");
        let out = run(&docs, args);
        assert_eq!(out.status.code(), Some(0), "{args:?}: {out:?}");
        assert_eq!(file(&docs, "adr/0002-two.md"), two);
        assert!(file(&docs, "adr/index.md").contains("(0002-two.md)"));
    }
}

#[test]
fn retitle_beside_mixed_files_renames_and_rewrites_references_leaving_them_as_they_were() {
    let docs = three_adrs("mixed-retitle");
    let two = file(&docs, "adr/0002-two.md");
    let notes = "see [three](adr/0003-three.md)\r\nsecond line\n";
    fs::write(docs.join("notes.md"), notes).unwrap();

    let out = run(&docs, &["set", "0003", "title", "Tri"]);

    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(docs.join("adr/0003-tri.md").is_file());
    assert!(!docs.join("adr/0003-three.md").exists());
    assert!(file(&docs, "adr/index.md").contains("(0003-tri.md)"));
    assert_eq!(file(&docs, "adr/0002-two.md"), two);
    let rewritten = notes.replace("0003-three.md", "0003-tri.md");
    assert_eq!(file(&docs, "notes.md"), rewritten);
}
