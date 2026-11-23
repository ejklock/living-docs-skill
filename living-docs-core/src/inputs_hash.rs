//! The hash of a generated region's inputs: the files that input
//! globs match under the repository root, read through the store port. The
//! hash is pure and deterministic: no clock, environment or locale.
//!
//! Glob syntax: `*` matches within one path segment, `**` across segments,
//! `?` one character. Braces and character classes are refused, as are an
//! empty glob, an absolute glob and a glob with a `..` segment.

use crate::store::DocStore;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::fmt;
use std::io;
use std::path::Path;

/// Why the inputs of a region could not be hashed.
#[derive(Debug)]
pub enum InputsError {
    /// A glob outside the supported syntax or reaching outside the repository.
    InvalidGlob {
        /// The glob as written.
        glob: String,
        /// The rule it breaks.
        reason: &'static str,
    },
    /// The store holds no repository root, or does not serve repository files.
    Unavailable,
    /// The store refused a path or failed to read a file.
    Store(io::Error),
}

impl fmt::Display for InputsError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGlob { glob, reason } => write!(out, "glob `{glob}`: {reason}"),
            Self::Unavailable => write!(out, "no repository root is known for these inputs"),
            Self::Store(error) => write!(out, "{error}"),
        }
    }
}

impl std::error::Error for InputsError {}

/// A compiled input glob: `*` within a segment, `**` as a whole segment across
/// segments, `?` one character.
#[derive(Debug, Clone)]
pub struct Glob {
    whole: Regex,
    dirs: Vec<Option<Regex>>,
}

impl Glob {
    /// Compiles `glob`, refusing braces, character classes, an empty glob, an
    /// absolute glob and a glob with a `..` segment.
    pub fn new(glob: &str) -> Result<Self, InputsError> {
        let invalid = |reason| InputsError::InvalidGlob {
            glob: glob.to_string(),
            reason,
        };
        if glob.is_empty() {
            return Err(invalid("a glob must not be empty"));
        }
        if glob.contains(['{', '}', '[', ']']) {
            return Err(invalid("braces and character classes are not supported"));
        }
        let segments: Vec<&str> = glob.split('/').collect();
        if glob.starts_with('/') || segments.contains(&"..") {
            return Err(invalid("a glob must stay inside the repository"));
        }
        let compiled = |pattern: String| {
            Regex::new(&pattern).map_err(|_| invalid("the glob does not compile"))
        };
        let last = segments.len() - 1;
        let mut dirs: Vec<Option<Regex>> = Vec::new();
        for (index, segment) in segments.iter().enumerate() {
            if *segment == "**" {
                dirs.push(None);
            } else if index < last {
                dirs.push(Some(compiled(format!("^{}$", segment_pattern(segment)))?));
            }
        }
        Ok(Self {
            whole: compiled(regex_of(&segments))?,
            dirs,
        })
    }

    /// Whether the repository-relative path `relative`, `/`-separated, matches.
    pub fn is_match(&self, relative: &str) -> bool {
        self.whole.is_match(relative)
    }

    /// Whether the directory `dir`, repository-relative and `/`-separated, can
    /// hold a file this glob matches, at any depth below it.
    pub fn can_match_below(&self, dir: &str) -> bool {
        for (depth, name) in dir.split('/').enumerate() {
            match self.dirs.get(depth) {
                None => return false,
                Some(None) => return true,
                Some(Some(segment)) if !segment.is_match(name) => return false,
                Some(Some(_)) => {}
            }
        }
        true
    }
}

/// The regular-expression body of one glob segment that is not `**`.
fn segment_pattern(segment: &str) -> String {
    segment
        .chars()
        .map(|c| match c {
            '*' => "[^/]*".to_string(),
            '?' => "[^/]".to_string(),
            other => regex::escape(other.encode_utf8(&mut [0; 4])),
        })
        .collect()
}

/// The anchored regular expression equivalent to a glob's segments. A whole
/// `**` segment crosses `/`; `**` inside a longer segment acts as `*`.
fn regex_of(segments: &[&str]) -> String {
    let last = segments.len() - 1;
    let mut out = String::from("^");
    for (index, segment) in segments.iter().enumerate() {
        match (*segment == "**", index == last) {
            (true, true) => out.push_str(".*"),
            (true, false) => out.push_str("(?:.*/)?"),
            (false, true) => out.push_str(&segment_pattern(segment)),
            (false, false) => {
                out.push_str(&segment_pattern(segment));
                out.push('/');
            }
        }
    }
    out.push('$');
    out
}

/// Whether `relative` matches any of `globs`.
pub fn matches_any(globs: &[Glob], relative: &str) -> bool {
    globs.iter().any(|glob| glob.is_match(relative))
}

/// Whether the directory `dir` can hold a file any of `globs` matches.
pub fn can_match_below(globs: &[Glob], dir: &str) -> bool {
    globs.iter().any(|glob| glob.can_match_below(dir))
}

/// Compiles every glob of `globs`, failing on the first invalid one.
pub fn compile(globs: &[String]) -> Result<Vec<Glob>, InputsError> {
    globs.iter().map(|glob| Glob::new(glob)).collect()
}

/// The lowercase-hex SHA-256 over every entry `globs` match under the
/// repository root that `.gitignore` does not exclude: de-duplicated, in
/// sorted path order, each fed as its repository-relative path with `/`
/// separators, a NUL byte, its content with `\r\n` read as `\n`, and a NUL
/// byte. A regular file's content is its bytes; a symbolic link's content is
/// its target text, so a plain file holding that text hashes the same. A set
/// of no entries hashes to the SHA-256 of no bytes, `e3b0c442...b855`.
pub fn inputs_hash(store: &dyn DocStore, globs: &[String]) -> Result<String, InputsError> {
    compile(globs)?;
    let matched = store
        .repository_files(globs)
        .ok_or(InputsError::Unavailable)?
        .map_err(InputsError::Store)?;
    let mut sorted: Vec<(String, Vec<u8>)> = matched
        .into_iter()
        .map(|(path, content)| (slashed(&path), content))
        .collect();
    sorted.sort_by(|left, right| left.0.cmp(&right.0));
    sorted.dedup_by(|left, right| left.0 == right.0);
    let mut digest = Sha256::new();
    for (path, content) in &sorted {
        digest.update(path.as_bytes());
        digest.update([0]);
        digest.update(normalise_newlines(content));
        digest.update([0]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn slashed(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn normalise_newlines(content: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(content.len());
    for (index, byte) in content.iter().enumerate() {
        if *byte == b'\r' && content.get(index + 1) == Some(&b'\n') {
            continue;
        }
        out.push(*byte);
    }
    out
}

#[cfg(test)]
mod tests;
