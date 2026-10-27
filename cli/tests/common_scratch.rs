mod common;

use common::{temp_bundle, Scratch};
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::Barrier;

#[test]
fn directory_exists_while_the_guard_is_alive() {
    let bundle = temp_bundle("scratch", "alive");
    assert!(bundle.is_dir(), "bundle dir missing: {}", bundle.display());
    assert!(bundle.parent().unwrap().is_dir());
}

#[test]
fn dropping_removes_the_root_and_everything_under_it() {
    let bundle = temp_bundle("scratch", "drop");
    let root = bundle.parent().unwrap().to_path_buf();
    fs::create_dir_all(bundle.join("adr/nested")).unwrap();
    fs::write(bundle.join("adr/nested/0001.md"), "body").unwrap();
    fs::write(root.join("sibling.txt"), "beside docs").unwrap();
    drop(bundle);
    assert!(!root.exists(), "root survived the drop: {}", root.display());
}

#[test]
fn a_panicking_test_still_removes_its_directory() {
    let mut root = PathBuf::new();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let bundle = temp_bundle("scratch", "panic");
        root = bundle.parent().unwrap().to_path_buf();
        fs::write(bundle.join("0001.md"), "body").unwrap();
        panic!("test failed on purpose");
    }));
    assert!(outcome.is_err());
    assert!(!root.as_os_str().is_empty());
    assert!(!root.exists(), "root survived a panic: {}", root.display());
}

#[test]
fn guards_created_at_the_same_instant_never_share_a_path() {
    const THREADS: usize = 16;
    const PER_THREAD: usize = 50;
    let barrier = Barrier::new(THREADS);
    let guards: Vec<Scratch> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    (0..PER_THREAD)
                        .map(|_| Scratch::new("living-docs-scratch"))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    let mut paths: Vec<PathBuf> = guards.iter().map(|g| g.root().to_path_buf()).collect();
    paths.sort();
    paths.dedup();
    assert_eq!(paths.len(), THREADS * PER_THREAD);
    assert!(paths.iter().all(|p| p.is_dir()));
}

#[test]
fn a_guard_derefs_to_its_root_or_to_its_subdirectory() {
    let plain = Scratch::new("living-docs-scratch");
    assert_eq!(&*plain, plain.root());
    let text = plain.to_str().unwrap();
    assert!(!text.ends_with('/') && !text.ends_with(std::path::MAIN_SEPARATOR));
    let nested = Scratch::with_subdir("living-docs-scratch", "docs");
    assert_eq!(&*nested, nested.root().join("docs"));
}
