//! Every byte string up to six bytes over `a`, `\r` and `\n` is rewritten
//! exactly as an independent oracle says: each `\r\n` becomes `\n` and every
//! other byte is kept. Text with no `\r` is left unchanged and normalises to
//! the same bytes as the same text with each `\n` written as `\r\n`.
//!
//! Directory pruning never drops a file a glob matches: over every file path of
//! up to three segments drawn from `a`, `b.md` and `c-d`, and every glob of up
//! to five tokens drawn from those names, `*`, `**`, `?` and `/`, the files a
//! walk that prunes directories reaches equal the files the glob matches when
//! tried against every path directly. The hash of the walk equals the hash of
//! the directly matched files, and the hash of any ordering of up to four
//! files is the same.

use super::{can_match_below, compile, inputs_hash, matches_any, normalise_newlines, Glob};
use crate::exhaustive::{for_each_permutation, for_each_sequence};
use crate::store::{DocStore, RepositoryFiles};
use std::io;
use std::path::{Path, PathBuf};

const ALPHABET: [u8; 3] = [b'a', b'\r', b'\n'];
const MAX_LEN: usize = 6;

/// Replaces each `\r\n` with `\n`, scanning left to right.
fn crlf_to_lf(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut rest = bytes;
    while let Some((first, tail)) = rest.split_first() {
        if rest.starts_with(b"\r\n") {
            out.push(b'\n');
            rest = &rest[2..];
        } else {
            out.push(*first);
            rest = tail;
        }
    }
    out
}

#[test]
fn normalisation_rewrites_exactly_each_crlf_for_every_short_text() {
    let cases = for_each_sequence(&ALPHABET, MAX_LEN, &mut |bytes| {
        assert_eq!(normalise_newlines(bytes), crlf_to_lf(bytes), "{bytes:?}");
    });
    assert_eq!(cases, 1093);
}

#[test]
fn lf_text_and_its_crlf_form_normalise_alike_for_every_short_text() {
    let mut lf_texts = 0;
    for_each_sequence(&ALPHABET, MAX_LEN, &mut |lf| {
        if lf.contains(&b'\r') {
            return;
        }
        lf_texts += 1;
        let crlf: Vec<u8> = lf
            .iter()
            .flat_map(|b| {
                if *b == b'\n' {
                    vec![b'\r', *b]
                } else {
                    vec![*b]
                }
            })
            .collect();
        assert_eq!(normalise_newlines(lf), lf, "{lf:?} changed");
        assert_eq!(normalise_newlines(lf), normalise_newlines(&crlf), "{lf:?}");
    });
    assert_eq!(lf_texts, 127);
}

const NAMES: [&str; 3] = ["a", "b.md", "c-d"];
const TOKENS: [&str; 6] = ["a", "b.md", "*", "**", "?", "/"];
const GLOB_LEN: usize = 5;
const HASHED_GLOB_LEN: usize = 4;

/// A store that lists exactly the files it holds, in the order held, or only
/// those a pruning walk would reach for the globs it is asked about.
struct Files {
    held: Vec<(PathBuf, Vec<u8>)>,
    prune: bool,
}

impl DocStore for Files {
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
    fn repository_files(&self, globs: &[String]) -> Option<RepositoryFiles> {
        let compiled = compile(globs).ok()?;
        let reached = |path: &Path| !self.prune || walk_reaches(&compiled, path);
        let kept = self.held.iter().filter(|(path, _)| reached(path));
        Some(Ok(kept
            .filter(|(p, _)| matches_any(&compiled, &text(p)))
            .cloned()
            .collect()))
    }
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Whether a walk that enters a directory only when a glob can match below it
/// reaches `path`: every directory above it is entered.
fn walk_reaches(globs: &[Glob], path: &Path) -> bool {
    let segments: Vec<String> = path
        .iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    (1..segments.len()).all(|depth| can_match_below(globs, &segments[..depth].join("/")))
}

/// Every file path of one to three segments over `NAMES`.
fn all_paths() -> Vec<String> {
    let mut paths = Vec::new();
    for_each_sequence(&NAMES, 3, &mut |names| {
        if !names.is_empty() {
            paths.push(names.join("/"));
        }
    });
    paths
}

/// Every glob up to `max_len` tokens that compiles, and how many sequences
/// were tried.
fn all_globs(max_len: usize) -> (Vec<String>, usize) {
    let mut globs = Vec::new();
    let tried = for_each_sequence(&TOKENS, max_len, &mut |tokens| {
        let glob = tokens.concat();
        if Glob::new(&glob).is_ok() {
            globs.push(glob);
        }
    });
    (globs, tried)
}

#[test]
fn pruning_reaches_exactly_the_files_a_glob_matches_for_every_short_glob() {
    let paths = all_paths();
    let (globs, tried) = all_globs(GLOB_LEN);
    let mut pairs = 0;
    for glob in &globs {
        let compiled = compile(std::slice::from_ref(glob)).unwrap();
        for path in &paths {
            let direct = matches_any(&compiled, path);
            let walked = walk_reaches(&compiled, Path::new(path)) && direct;
            assert_eq!(walked, direct, "glob `{glob}` path `{path}`");
            pairs += 1;
        }
    }
    assert_eq!(
        (paths.len(), tried, globs.len(), pairs),
        (39, 9331, 7775, 303_225)
    );
}

#[test]
fn the_hash_of_a_pruned_walk_equals_the_hash_of_the_directly_matched_files() {
    let held: Vec<(PathBuf, Vec<u8>)> = all_paths()
        .into_iter()
        .map(|p| (PathBuf::from(&p), p.into_bytes()))
        .collect();
    let (globs, _) = all_globs(HASHED_GLOB_LEN);
    let mut nonempty = 0;
    for glob in &globs {
        let one = vec![glob.clone()];
        let compiled = compile(&one).unwrap();
        let direct = held
            .iter()
            .filter(|(p, _)| matches_any(&compiled, &text(p)))
            .cloned();
        let oracle = Files {
            held: direct.collect(),
            prune: false,
        };
        let walk = Files {
            held: held.clone(),
            prune: true,
        };
        let expected = inputs_hash(&oracle, &one).unwrap();
        assert_eq!(inputs_hash(&walk, &one).unwrap(), expected, "glob `{glob}`");
        nonempty += usize::from(!oracle.held.is_empty());
    }
    assert_eq!(nonempty, 402);
}

#[test]
fn the_hash_is_the_same_for_every_ordering_of_up_to_four_files() {
    let files = ["a", "b.md", "c-d", "a/b.md"];
    let globs = vec!["**".to_string()];
    let (mut orderings, mut checked) = (0, 0);
    for size in 0..=files.len() {
        let held: Vec<(PathBuf, Vec<u8>)> = files[..size]
            .iter()
            .map(|f| (PathBuf::from(f), f.as_bytes().to_vec()))
            .collect();
        let expected = inputs_hash(
            &Files {
                held: held.clone(),
                prune: false,
            },
            &globs,
        )
        .unwrap();
        orderings += for_each_permutation(&held, &mut |order| {
            let store = Files {
                held: order.to_vec(),
                prune: false,
            };
            assert_eq!(inputs_hash(&store, &globs).unwrap(), expected, "{order:?}");
            checked += 1;
        });
    }
    assert_eq!((orderings, checked), (34, 34));
}
