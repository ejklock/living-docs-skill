//! The filesystem store's repository probe answers whether a
//! repository-relative path exists, inside the repository only.

use fs_store::FsStore;
use living_docs_core::store::DocStore;
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
            "fs-store-repo-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root.canonicalize().unwrap())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn repository(label: &str) -> (Scratch, FsStore) {
    let scratch = Scratch::new(label);
    fs::create_dir_all(scratch.0.join(".git")).unwrap();
    fs::create_dir_all(scratch.0.join("src")).unwrap();
    fs::write(scratch.0.join("src/lib.rs"), "x").unwrap();
    fs::create_dir_all(scratch.0.join("docs")).unwrap();
    let store = FsStore::rooted(scratch.0.join("docs"));
    (scratch, store)
}

fn exists(store: &FsStore, relative: &str) -> Option<bool> {
    store.repository_entry_exists(Path::new(relative))
}

#[test]
fn an_existing_file_and_directory_exist() {
    let (_scratch, store) = repository("exists");
    assert_eq!(exists(&store, "src/lib.rs"), Some(true));
    assert_eq!(exists(&store, "src"), Some(true));
    assert_eq!(exists(&store, "src/"), Some(true));
}

#[test]
fn a_missing_file_under_an_existing_directory_does_not_exist() {
    let (_scratch, store) = repository("missing");
    assert_eq!(exists(&store, "src/gone.rs"), Some(false));
}

#[test]
fn a_file_below_a_missing_subdirectory_does_not_exist() {
    let (_scratch, store) = repository("missing-dir");
    assert_eq!(exists(&store, "src/gone_dir/a.rs"), Some(false));
}

#[test]
fn an_absolute_path_is_refused() {
    let (_scratch, store) = repository("absolute");
    assert_eq!(exists(&store, "/etc/passwd"), None);
}

#[test]
fn a_path_with_a_parent_segment_is_refused() {
    let (_scratch, store) = repository("parent");
    assert_eq!(exists(&store, "src/../../etc/passwd"), None);
    assert_eq!(exists(&store, "src/../src/lib.rs"), None);
}

#[cfg(unix)]
#[test]
fn a_symlink_leaving_the_repository_is_refused() {
    let (scratch, store) = repository("symlink");
    let outside = Scratch::new("symlink-outside");
    fs::write(outside.0.join("secret.txt"), "x").unwrap();
    std::os::unix::fs::symlink(&outside.0, scratch.0.join("link")).unwrap();
    assert_eq!(exists(&store, "link/secret.txt"), None);
    assert_eq!(exists(&store, "link"), None);
    assert_eq!(exists(&store, "link/missing.txt"), None);
}

#[cfg(unix)]
#[test]
fn a_link_to_a_target_outside_answers_the_same_whether_or_not_the_target_exists() {
    use std::os::unix::fs::symlink;
    let (scratch, store) = repository("link-targets");
    let outside = Scratch::new("link-targets-outside");
    fs::write(outside.0.join("f.txt"), "x").unwrap();
    let src = scratch.0.join("src");
    symlink(outside.0.join("f.txt"), src.join("out_exists")).unwrap();
    symlink(outside.0.join("nofile.txt"), src.join("out_missing")).unwrap();
    symlink(outside.0.join("nodir"), src.join("dangling_out")).unwrap();
    symlink("../../outside_missing", src.join("rel_out")).unwrap();
    assert_eq!(exists(&store, "src/out_exists"), None);
    assert_eq!(exists(&store, "src/out_missing"), None);
    assert_eq!(exists(&store, "src/dangling_out"), None);
    assert_eq!(exists(&store, "src/dangling_out/x"), None);
    assert_eq!(exists(&store, "src/rel_out"), None);
}

#[cfg(unix)]
#[test]
fn a_dangling_link_or_a_link_loop_inside_the_repository_does_not_exist() {
    use std::os::unix::fs::symlink;
    let (scratch, store) = repository("link-inside");
    let src = scratch.0.join("src");
    symlink(src.join("nofile.rs"), src.join("dangling_in")).unwrap();
    symlink("loop_b", src.join("loop_a")).unwrap();
    symlink("loop_a", src.join("loop_b")).unwrap();
    assert_eq!(exists(&store, "src/dangling_in"), Some(false));
    assert_eq!(exists(&store, "src/loop_a"), Some(false));
}

#[test]
fn a_bundle_without_a_git_ancestor_has_no_repository() {
    let scratch = Scratch::new("none");
    fs::create_dir_all(scratch.0.join("docs")).unwrap();
    let store = FsStore::rooted(scratch.0.join("docs"));
    assert!(
        !scratch.0.ancestors().any(|dir| dir.join(".git").exists()),
        "precondition: the temporary directory must have no .git ancestor"
    );
    assert_eq!(exists(&store, "docs"), None);
}

#[test]
fn a_git_file_marks_the_repository_root() {
    let scratch = Scratch::new("gitfile");
    fs::write(scratch.0.join(".git"), "gitdir: elsewhere\n").unwrap();
    fs::create_dir_all(scratch.0.join("src")).unwrap();
    fs::create_dir_all(scratch.0.join("docs")).unwrap();
    let store = FsStore::rooted(scratch.0.join("docs"));
    assert_eq!(exists(&store, "src"), Some(true));
}

#[test]
fn a_bundle_nested_below_the_repository_root_finds_it() {
    let (scratch, _) = repository("nested");
    let nested = scratch.0.join("docs/deep/bundle");
    fs::create_dir_all(&nested).unwrap();
    let store = FsStore::rooted(nested);
    assert_eq!(exists(&store, "src/lib.rs"), Some(true));
    assert_eq!(exists(&store, "src/gone.rs"), Some(false));
}
