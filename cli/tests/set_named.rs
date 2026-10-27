mod common;

use common::{living_docs, run_check, run_new, stdout_of, temp_bundle};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

fn set(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "set"])
        .args(args)
        .arg("--plain")
        .output()
        .unwrap()
}

fn new_view(docs: &Path, title: &str, kind: &str) {
    let out = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap()])
        .args(["new", "view", title, "--kind", kind])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
}

fn view_bundle(label: &str) -> common::Scratch {
    let docs = temp_bundle("set-named", label);
    new_view(&docs, "Rings", "component");
    docs
}

fn read(docs: &Path, rel: &str) -> String {
    fs::read_to_string(docs.join(rel)).unwrap()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn snapshot(docs: &Path) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    let mut pending = vec![docs.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push((path.clone(), fs::read_to_string(&path).unwrap()));
            }
        }
    }
    files.sort();
    files
}

fn assert_refused_unchanged(docs: &Path, args: &[&str]) -> String {
    let before = snapshot(docs);
    let out = set(docs, args);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert_eq!(snapshot(docs), before, "a refused set must write nothing");
    stderr_of(&out)
}

fn findings(docs: &Path) -> String {
    stdout_of(&run_check(docs)).replace(docs.to_str().unwrap(), "<docs>")
}

#[test]
fn a_view_description_is_set_by_slug_or_by_directory() {
    for reference in ["view/rings", "architecture/rings"] {
        let docs = view_bundle(&format!("describe-{}", reference.replace('/', "-")));
        let before = findings(&docs);
        let out = set(&docs, &[reference, "description", "Shows the rings."]);
        assert!(out.status.success(), "{reference}: {out:?}");
        assert!(read(&docs, "architecture/rings.md").contains("description: Shows the rings."));
        assert_eq!(findings(&docs), before, "{reference}: check regressed");
    }
}

#[test]
fn the_constitution_description_is_set_by_name() {
    let docs = temp_bundle("set-named", "constitution-description");
    assert!(run_new(&docs, "constitution", "Product").status.success());
    let out = set(&docs, &["constitution", "description", "Why we exist."]);
    assert!(out.status.success(), "{out:?}");
    assert!(read(&docs, "constitution.md").contains("description: Why we exist."));
}

#[test]
fn a_missing_slug_is_refused_by_name() {
    let docs = view_bundle("missing-slug");
    let message = assert_refused_unchanged(&docs, &["view/nope", "description", "x"]);
    assert!(message.contains("nope"), "got: {message}");
}

#[test]
fn a_hostile_slug_is_refused_and_writes_nothing() {
    let docs = view_bundle("hostile-slug");
    assert_refused_unchanged(&docs, &["view/", "description", "x"]);
    for reference in [
        "view/../adr/0001",
        "view//etc/passwd",
        "view/a/b",
        "view/Rings",
    ] {
        let message = assert_refused_unchanged(&docs, &[reference, "description", "x"]);
        assert!(
            message.contains("is not a view slug"),
            "{reference}: {message}"
        );
    }
}

#[path = "set_named/links.rs"]
mod links;

#[test]
fn a_view_title_renames_the_file_and_repoints_references() {
    let docs = view_bundle("retitle");
    fs::write(
        docs.join("notes.md"),
        "# Notes\n\nSee [rings](architecture/rings.md).\n",
    )
    .unwrap();
    let out = set(&docs, &["view/rings", "title", "Orbits"]);
    assert!(out.status.success(), "{out:?}");
    assert!(!docs.join("architecture/rings.md").exists());
    let renamed = read(&docs, "architecture/orbits.md");
    assert!(renamed.contains("title: Orbits"), "{renamed}");
    assert!(read(&docs, "notes.md").contains("architecture/orbits.md"));
    assert!(read(&docs, "architecture/index.md").contains("orbits.md"));
    assert!(!read(&docs, "architecture/index.md").contains("rings.md"));
    let before = snapshot(&docs);
    let indexed = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "index"])
        .output()
        .unwrap();
    assert!(indexed.status.success(), "{indexed:?}");
    assert_eq!(snapshot(&docs), before, "index must write nothing");
}

#[test]
fn a_view_title_leaves_a_reference_to_a_view_with_a_longer_name_alone() {
    let docs = view_bundle("longer-name");
    new_view(&docs, "Strings", "flow");
    let notes = "See [rings](architecture/rings.md) and [strings](architecture/strings.md).\n";
    fs::write(docs.join("notes.md"), notes).unwrap();
    let out = set(&docs, &["view/rings", "title", "Orbits"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        read(&docs, "notes.md"),
        "See [rings](architecture/orbits.md) and [strings](architecture/strings.md).\n"
    );
}

#[test]
fn a_view_title_colliding_with_another_view_changes_nothing() {
    let docs = view_bundle("collision");
    new_view(&docs, "Orbits", "flow");
    assert_refused_unchanged(&docs, &["view/rings", "title", "Orbits"]);
}

#[test]
fn a_view_refuses_the_keys_it_does_not_carry() {
    let docs = view_bundle("no-status-owner");
    let status = assert_refused_unchanged(&docs, &["view/rings", "status", "Accepted"]);
    assert!(status.contains("no status"), "got: {status}");
    let owner = assert_refused_unchanged(&docs, &["view/rings", "owner", "x"]);
    assert!(owner.contains("no owner"), "got: {owner}");
}

#[test]
fn the_constitution_refuses_the_keys_it_does_not_carry() {
    let docs = temp_bundle("set-named", "constitution-refusals");
    assert!(run_new(&docs, "constitution", "Product").status.success());
    let status = assert_refused_unchanged(&docs, &["constitution", "status", "Ratified"]);
    assert!(status.contains("no status"), "got: {status}");
    let owner = assert_refused_unchanged(&docs, &["constitution", "owner", "x"]);
    assert!(owner.contains("no owner"), "got: {owner}");
}

#[test]
fn the_invalid_type_error_names_the_named_reference_forms() {
    let docs = view_bundle("invalid-type");
    let message = assert_refused_unchanged(&docs, &["bogus/0001", "description", "x"]);
    for form in ["view/<slug>", "architecture/<slug>", "constitution"] {
        assert!(message.contains(form), "missing {form} in: {message}");
    }
    assert!(
        message.contains("adr, prd, issue, research"),
        "got: {message}"
    );
}

fn write_doc(docs: &Path, rel: &str, text: &str) {
    fs::create_dir_all(docs.join(rel).parent().unwrap()).unwrap();
    fs::write(docs.join(rel), text).unwrap();
}

#[test]
fn a_view_retitle_leaves_a_link_to_a_root_file_with_the_same_name() {
    for (root_file, title) in [
        ("constitution.md", "Constitution"),
        ("overview.md", "Overview"),
    ] {
        let docs = temp_bundle("set-named", &format!("shared-name-{title}"));
        write_doc(&docs, root_file, "# Root\n");
        new_view(&docs, title, "flow");
        let both = format!("[root](../{root_file}) and [view](../architecture/{root_file})\n");
        write_doc(&docs, "adr/note.md", &both);
        let reference = format!("view/{}", title.to_lowercase());
        let out = set(&docs, &[&reference, "title", "Charter"]);
        assert!(out.status.success(), "{out:?}");
        assert_eq!(
            read(&docs, "adr/note.md"),
            format!("[root](../{root_file}) and [view](../architecture/charter.md)\n")
        );
        assert!(
            !findings(&docs).contains("broken link"),
            "{}",
            findings(&docs)
        );
    }
}

#[test]
fn a_view_retitle_rewrites_links_from_the_view_directory_and_the_root() {
    let docs = view_bundle("link-origins");
    write_doc(
        &docs,
        "architecture/sibling.md",
        "---\ntype: Architecture View\ntitle: Sibling\ndescription: d\nkind: flow\n---\n\n# Sibling\n\n[a](./rings.md) [b](rings.md) [c](../architecture/rings.md#top)\n",
    );
    write_doc(&docs, "notes.md", "[v](architecture/rings.md)\n");
    write_doc(&docs, "constitution.md", "# Root\n");
    let out = set(&docs, &["view/rings", "title", "Orbits"]);
    assert!(out.status.success(), "{out:?}");
    let sibling = read(&docs, "architecture/sibling.md");
    assert!(
        sibling.contains("[a](./orbits.md) [b](orbits.md) [c](../architecture/orbits.md#top)"),
        "{sibling}"
    );
    assert_eq!(read(&docs, "notes.md"), "[v](architecture/orbits.md)\n");
}

#[test]
fn a_numbered_retitle_rewrites_emphasised_and_linked_mentions() {
    let docs = temp_bundle("set-named", "emphasis");
    assert!(run_new(&docs, "adr", "Base decision").status.success());
    write_doc(
        &docs,
        "notes.md",
        "_0001-base-decision.md_ and __0001-base-decision.md__ and [l](adr/0001-base-decision.md)\n",
    );
    let out = set(&docs, &["adr/0001", "title", "Base choice"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        read(&docs, "notes.md"),
        "_0001-base-choice.md_ and __0001-base-choice.md__ and [l](adr/0001-base-choice.md)\n"
    );
}
