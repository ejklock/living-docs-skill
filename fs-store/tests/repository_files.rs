//! The filesystem store's repository file read serves the files an input glob
//! matches, from the repository root, never leaving it.

use fs_store::FsStore;
use living_docs_core::inputs_hash::inputs_hash;
use living_docs_core::store::{is_refusal, DocStore};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "fs-store-files-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root.canonicalize().unwrap())
    }

    fn put(&self, relative: &str, body: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(
                self.0.join("docs/locked"),
                fs::Permissions::from_mode(0o755),
            );
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A scratch git repository whose bundle is `docs/`.
fn repository(label: &str) -> (Scratch, FsStore) {
    let scratch = Scratch::new(label);
    fs::create_dir_all(scratch.0.join(".git")).unwrap();
    fs::create_dir_all(scratch.0.join("docs")).unwrap();
    let store = FsStore::rooted(scratch.0.join("docs"));
    (scratch, store)
}

fn globs(list: &[&str]) -> Vec<String> {
    list.iter().map(|glob| (*glob).to_string()).collect()
}

fn names(store: &FsStore, list: &[&str]) -> Vec<String> {
    let found = store.repository_files(&globs(list)).unwrap().unwrap();
    let mut names: Vec<String> = found
        .iter()
        .map(|(path, _)| path.to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn content_of(store: &FsStore, list: &[&str], name: &str) -> Vec<u8> {
    let found = store.repository_files(&globs(list)).unwrap().unwrap();
    let (_, content) = found
        .into_iter()
        .find(|(path, _)| path == Path::new(name))
        .unwrap_or_else(|| panic!("{name} was not matched"));
    content
}

fn hash_of(store: &FsStore, list: &[&str]) -> String {
    inputs_hash(store, &globs(list)).unwrap()
}

#[test]
fn globs_resolve_from_the_repository_root_not_the_bundle() {
    let (scratch, store) = repository("root");
    scratch.put("Cargo.toml", "root");
    scratch.put("a/Cargo.toml", "a");
    scratch.put("docs/Cargo.toml", "bundle");
    assert_eq!(
        names(&store, &["Cargo.toml", "*/Cargo.toml"]),
        ["Cargo.toml", "a/Cargo.toml", "docs/Cargo.toml"]
    );
}

#[test]
fn a_matched_file_comes_back_with_its_content() {
    let (scratch, store) = repository("content");
    scratch.put("a.toml", "body");
    let found = store
        .repository_files(&globs(&["*.toml"]))
        .unwrap()
        .unwrap();
    assert_eq!(found, vec![(PathBuf::from("a.toml"), b"body".to_vec())]);
}

#[test]
fn the_git_directory_is_never_read() {
    let (scratch, store) = repository("git");
    scratch.put(".git/config", "secret");
    scratch.put("a/.git/config", "nested");
    assert!(names(&store, &["**"]).is_empty());
    assert!(names(&store, &[".git/*"]).is_empty());
}

#[test]
fn an_absolute_glob_is_refused() {
    let (_scratch, store) = repository("absolute");
    let error = store
        .repository_files(&globs(&["/etc/passwd"]))
        .unwrap()
        .unwrap_err();
    assert!(is_refusal(&error), "{error:?}");
}

#[test]
fn a_parent_segment_glob_is_refused() {
    let (_scratch, store) = repository("parent");
    let error = store
        .repository_files(&globs(&["../*"]))
        .unwrap()
        .unwrap_err();
    assert!(is_refusal(&error), "{error:?}");
}

#[cfg(unix)]
#[test]
fn a_symlink_leaving_the_repository_is_not_followed() {
    let (scratch, store) = repository("link");
    let outside = Scratch::new("link-outside");
    outside.put("secret.toml", "first");
    outside.put("dir/inner.toml", "inner");
    std::os::unix::fs::symlink(outside.0.join("secret.toml"), scratch.0.join("leak.toml")).unwrap();
    std::os::unix::fs::symlink(outside.0.join("dir"), scratch.0.join("leakdir")).unwrap();
    scratch.put("a.toml", "kept");

    assert_eq!(names(&store, &["**"]), ["a.toml", "leak.toml", "leakdir"]);
    let target = outside.0.join("secret.toml");
    assert_eq!(
        content_of(&store, &["**"], "leak.toml"),
        target.to_str().unwrap().as_bytes()
    );
    let before = hash_of(&store, &["**"]);
    outside.put("secret.toml", "second");
    outside.put("dir/inner.toml", "changed");
    assert_eq!(hash_of(&store, &["**"]), before);
}

#[cfg(unix)]
#[test]
fn a_symlink_hashes_as_its_target_text_and_is_never_followed() {
    let (scratch, store) = repository("inside");
    scratch.put(".git/config", "secret");
    scratch.put("a.toml", "kept");
    std::os::unix::fs::symlink(".git/config", scratch.0.join("cfg.toml")).unwrap();
    std::os::unix::fs::symlink("a.toml", scratch.0.join("twin.toml")).unwrap();

    assert_eq!(content_of(&store, &["*.toml"], "cfg.toml"), b".git/config");
    assert_eq!(content_of(&store, &["*.toml"], "twin.toml"), b"a.toml");
}

#[cfg(unix)]
#[test]
fn re_pointing_a_symlink_changes_the_hash() {
    let (scratch, store) = repository("repoint");
    scratch.put("a.toml", "x");
    scratch.put("b.toml", "x");
    let link = scratch.0.join("l.toml");
    std::os::unix::fs::symlink("a.toml", &link).unwrap();
    let before = hash_of(&store, &["l.toml"]);
    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink("b.toml", &link).unwrap();
    assert_ne!(hash_of(&store, &["l.toml"]), before);
}

#[cfg(unix)]
#[test]
fn a_plain_file_holding_a_links_target_text_hashes_like_the_link() {
    let (linked, linked_store) = repository("as-link");
    linked.put("a.toml", "x");
    std::os::unix::fs::symlink("a.toml", linked.0.join("l.toml")).unwrap();
    let (plain, plain_store) = repository("as-file");
    plain.put("l.toml", "a.toml");
    assert_eq!(
        hash_of(&linked_store, &["l.toml"]),
        hash_of(&plain_store, &["l.toml"])
    );
}

#[test]
fn a_file_listed_in_gitignore_is_not_hashed_unless_a_glob_matches_gitignore_itself() {
    let (scratch, store) = repository("ignored");
    scratch.put(".gitignore", "secret.toml\n");
    scratch.put("secret.toml", "s");
    scratch.put("a.toml", "x");
    assert_eq!(names(&store, &["*.toml"]), ["a.toml"]);
    assert_eq!(
        names(&store, &["*.toml", ".gitignore"]),
        [".gitignore", "a.toml"]
    );
}

#[test]
fn a_nested_gitignore_is_honored_and_a_gitignore_change_alone_is_no_input() {
    let (scratch, store) = repository("nested");
    scratch.put("a/.gitignore", "gen/\n*.log\n");
    scratch.put("a/gen/out.toml", "g");
    scratch.put("a/x.log", "l");
    scratch.put("a/keep.toml", "k");
    scratch.put("b/gen/out.toml", "elsewhere");
    assert_eq!(
        names(&store, &["**/*.toml"]),
        ["a/keep.toml", "b/gen/out.toml"]
    );
    let before = hash_of(&store, &["**/*.toml"]);
    scratch.put("a/.gitignore", "gen/\n*.log\n*.tmp\n");
    assert_eq!(hash_of(&store, &["**/*.toml"]), before);
}

#[cfg(unix)]
#[test]
fn an_unreadable_folder_no_glob_could_match_does_not_fail_a_hash() {
    use std::os::unix::fs::PermissionsExt;
    let (scratch, store) = repository("locked");
    scratch.put("a.toml", "x");
    scratch.put("a/Cargo.toml", "y");
    scratch.put("docs/locked/inner.toml", "z");
    fs::set_permissions(
        scratch.0.join("docs/locked"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    let result = inputs_hash(&store, &globs(&["a.toml", "*/Cargo.toml"]));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn a_store_with_no_git_ancestor_answers_none() {
    let scratch = Scratch::new("nogit");
    if scratch.0.ancestors().any(|dir| dir.join(".git").exists()) {
        eprintln!("skipped: the system temporary folder lies inside a git checkout");
        return;
    }
    fs::create_dir_all(scratch.0.join("docs")).unwrap();
    let store = FsStore::rooted(scratch.0.join("docs"));
    assert!(store.repository_files(&globs(&["*"])).is_none());
}
