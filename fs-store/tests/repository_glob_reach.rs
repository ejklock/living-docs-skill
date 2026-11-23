//! The filesystem store tells a glob that matches a file apart from one that
//! matches only what `.gitignore` excludes and one that matches nothing.

use fs_store::FsStore;
use living_docs_core::store::{is_refusal, DocStore, GlobReach};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct Scratch(PathBuf);

impl Scratch {
    fn put(&self, relative: &str, body: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A scratch git repository whose bundle is `docs/`.
fn repository(label: &str) -> (Scratch, FsStore) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let name = format!("fs-store-reach-{label}-{}-{nanos}", std::process::id());
    let root = std::env::temp_dir().join(name);
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    let root = root.canonicalize().unwrap();
    let store = FsStore::rooted(root.join("docs"));
    (Scratch(root), store)
}

fn reach(store: &FsStore, globs: &[&str]) -> Vec<GlobReach> {
    let globs: Vec<String> = globs.iter().map(|glob| (*glob).to_string()).collect();
    store.repository_glob_reach(&globs).unwrap().unwrap()
}

#[test]
fn a_glob_is_matched_ignored_only_or_nothing() {
    let (scratch, store) = repository("three");
    scratch.put(".gitignore", "gen/\n");
    scratch.put("Cargo.toml", "[workspace]\n");
    scratch.put("gen/schema.toml", "a = 1\n");
    let globs = ["Cargo.toml", "gen/schema.toml", "none/*.toml"];
    assert_eq!(
        reach(&store, &globs),
        [
            GlobReach::Matched,
            GlobReach::IgnoredOnly,
            GlobReach::Nothing
        ]
    );
}

#[test]
fn a_negated_re_include_makes_the_kept_file_matched_and_the_rest_ignored() {
    let (scratch, store) = repository("negated");
    scratch.put(".gitignore", "neg/*\n!neg/keep.toml\n");
    scratch.put("neg/keep.toml", "a = 1\n");
    scratch.put("neg/drop.toml", "b = 2\n");
    assert_eq!(
        reach(&store, &["neg/keep.toml", "neg/drop.toml"]),
        [GlobReach::Matched, GlobReach::IgnoredOnly]
    );
}

#[test]
fn a_nested_gitignore_excludes_only_below_its_own_folder() {
    let (scratch, store) = repository("nested");
    scratch.put("sub/.gitignore", "*.gen\n");
    scratch.put("sub/a.gen", "a\n");
    scratch.put("other/b.gen", "b\n");
    assert_eq!(
        reach(&store, &["sub/*.gen", "other/*.gen"]),
        [GlobReach::IgnoredOnly, GlobReach::Matched]
    );
}

#[test]
fn a_glob_matching_a_kept_and_an_ignored_file_is_matched() {
    let (scratch, store) = repository("mixed");
    scratch.put(".gitignore", "drop.toml\n");
    scratch.put("keep.toml", "a = 1\n");
    scratch.put("drop.toml", "b = 2\n");
    assert_eq!(reach(&store, &["*.toml"]), [GlobReach::Matched]);
}

#[test]
fn an_invalid_glob_is_refused() {
    let (_scratch, store) = repository("invalid");
    for glob in ["/abs", "../x", "a{b,c}"] {
        let globs = vec![glob.to_string()];
        let error = store.repository_glob_reach(&globs).unwrap().unwrap_err();
        assert!(is_refusal(&error), "{glob}: {error:?}");
    }
}

#[test]
fn a_store_with_no_git_ancestor_answers_none() {
    let scratch = std::env::temp_dir().join(format!("fs-store-reach-bare-{}", std::process::id()));
    fs::create_dir_all(&scratch).unwrap();
    let store = FsStore::rooted(&scratch);
    let answer = store.repository_glob_reach(&["*".to_string()]);
    fs::remove_dir_all(&scratch).unwrap();
    assert!(answer.is_none());
}
