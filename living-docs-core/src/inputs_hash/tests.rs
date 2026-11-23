use super::*;
use crate::store::{DocStore, RepositoryFiles};
use crate::test_support::MapStore;
use std::io;
use std::path::{Path, PathBuf};

/// The SHA-256 of zero bytes, the documented value for an empty file set.
const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn hash(seed: &[(&str, &str)], globs: &[&str]) -> String {
    let globs: Vec<String> = globs.iter().map(|g| (*g).to_string()).collect();
    inputs_hash(&MapStore::seeded(seed).0, &globs).unwrap()
}

/// A store answering with the files it was given, in the order given, to
/// prove the hash does not depend on the order a store lists in. With no
/// listing it answers that it holds no repository.
struct Listing(Option<Vec<(PathBuf, Vec<u8>)>>);

impl DocStore for Listing {
    fn list(&self, _: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(vec![])
    }
    fn read(&self, _: &Path) -> io::Result<String> {
        Ok(String::new())
    }
    fn write(&self, _: &Path, _: &str) -> io::Result<()> {
        Ok(())
    }
    fn rename(&self, _: &Path, _: &Path) -> io::Result<()> {
        Ok(())
    }
    fn repository_files(&self, _: &[String]) -> Option<RepositoryFiles> {
        self.0.clone().map(Ok)
    }
}

fn listing(files: Vec<(PathBuf, Vec<u8>)>) -> Listing {
    Listing(Some(files))
}

fn pair(path: &str, body: &str) -> (PathBuf, Vec<u8>) {
    (PathBuf::from(path), body.as_bytes().to_vec())
}

fn refused(glob: &str) -> bool {
    let error = inputs_hash(&MapStore::seeded(&[]).0, &[glob.to_string()]).unwrap_err();
    matches!(error, InputsError::InvalidGlob { .. })
}

#[test]
fn the_same_files_in_any_listing_order_give_the_same_hash() {
    let forward = listing(vec![pair("a.toml", "1"), pair("b.toml", "2")]);
    let backward = listing(vec![pair("b.toml", "2"), pair("a.toml", "1")]);
    let globs = vec!["*.toml".to_string()];
    assert_eq!(
        inputs_hash(&forward, &globs).unwrap(),
        inputs_hash(&backward, &globs).unwrap()
    );
}

#[test]
fn a_crlf_file_hashes_like_the_same_file_with_lf() {
    let crlf = hash(&[("a.toml", "x\r\ny\r\n")], &["a.toml"]);
    let lf = hash(&[("a.toml", "x\ny\n")], &["a.toml"]);
    assert_eq!(crlf, lf);
}

#[test]
fn one_changed_byte_changes_the_hash() {
    let before = hash(&[("a.toml", "x1")], &["a.toml"]);
    let after = hash(&[("a.toml", "x2")], &["a.toml"]);
    assert_ne!(before, after);
}

#[test]
fn a_renamed_file_changes_the_hash() {
    let before = hash(&[("a.toml", "x")], &["*.toml"]);
    let after = hash(&[("b.toml", "x")], &["*.toml"]);
    assert_ne!(before, after);
}

#[test]
fn two_globs_matching_one_file_count_it_once() {
    let once = hash(&[("a.toml", "x")], &["a.toml"]);
    let twice = hash(&[("a.toml", "x")], &["a.toml", "*.toml"]);
    assert_eq!(once, twice);
}

/// The expected value is `printf 'a.toml\0x\0' | sha256sum`: path, NUL,
/// content, NUL.
#[test]
fn a_known_file_hashes_to_the_documented_framing() {
    let expected = "37d6494d95a68ccc574e7858e725061c65864920c02c2401f0baf870a968e457";
    assert_eq!(hash(&[("a.toml", "x")], &["a.toml"]), expected);
}

#[test]
fn double_star_crosses_segments_and_single_star_does_not() {
    let seed = [
        ("Cargo.toml", "r"),
        ("a/Cargo.toml", "a"),
        ("a/b/Cargo.toml", "b"),
    ];
    let single = hash(&seed, &["*/Cargo.toml"]);
    let only_a = hash(&[("a/Cargo.toml", "a")], &["a/Cargo.toml"]);
    assert_eq!(single, only_a);
    let deep = hash(&seed, &["**/Cargo.toml"]);
    let all = hash(&seed, &["Cargo.toml", "a/Cargo.toml", "a/b/Cargo.toml"]);
    assert_eq!(deep, all);
}

#[test]
fn question_mark_matches_one_character() {
    let seed = [("a1.md", "1"), ("a22.md", "2")];
    assert_eq!(hash(&seed, &["a?.md"]), hash(&[("a1.md", "1")], &["a1.md"]));
}

#[test]
fn braces_and_classes_are_refused() {
    assert!(refused("{a,b}.toml"));
    assert!(refused("[ab].toml"));
}

#[test]
fn absolute_parent_and_empty_globs_are_refused() {
    assert!(refused("/etc/passwd"));
    assert!(refused("../x"));
    assert!(refused("a/../x"));
    assert!(refused(""));
}

#[test]
fn a_glob_matching_nothing_hashes_the_empty_set() {
    assert_eq!(hash(&[("a.toml", "x")], &["*.json"]), EMPTY);
}

#[test]
fn a_store_without_a_repository_is_reported_unavailable() {
    let error = inputs_hash(&Listing(None), &["a".to_string()]).unwrap_err();
    assert!(matches!(error, InputsError::Unavailable));
}

#[test]
fn a_store_listing_one_path_twice_hashes_it_once() {
    let once = listing(vec![pair("a.toml", "x")]);
    let twice = listing(vec![pair("a.toml", "x"), pair("a.toml", "x")]);
    let globs = vec!["*.toml".to_string()];
    assert_eq!(
        inputs_hash(&once, &globs).unwrap(),
        inputs_hash(&twice, &globs).unwrap()
    );
}

#[test]
fn a_double_star_inside_a_segment_does_not_cross_a_slash() {
    let seed = [("ab", "1"), ("axxb", "2"), ("a/x/b", "3")];
    let double = hash(&seed, &["a**b"]);
    let single = hash(&seed, &["a*b"]);
    assert_eq!(double, single);
    assert_eq!(double, hash(&[("ab", "1"), ("axxb", "2")], &["ab", "axxb"]));
}
