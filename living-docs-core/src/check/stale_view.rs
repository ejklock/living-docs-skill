//! `stale-view` advisory: an architecture view whose stamped generated region
//! records a hash that differs from the current hash of its input globs. The
//! region's command is never run. A store that knows no repository root keeps
//! the rule silent; a stamp that cannot be parsed or hashed is advised as
//! unverifiable. An unstamped region is never judged. It never fails the gate.
//! The advisory's anchors are the view followed by each input glob, so a
//! scope of changed inputs can keep it by matching a glob.

use super::records::{for_each_readable_record, frontmatter_scalar};
use super::{kinds, Reporter};
use crate::doc_type;
use crate::generated_region::{parse_regions, Region};
use crate::inputs_hash::{inputs_hash, InputsError};
use crate::record::strip_frontmatter;
use crate::store::DocStore;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The directory of the architecture view type.
const VIEW_DIR: &str = "architecture";

/// What hashing one glob set gave: the hash, no repository to hash in, or the
/// reason it could not be hashed.
enum Outcome {
    Hash(String),
    NoRepository,
    Failed(String),
}

pub(crate) fn check_stale_view(store: &dyn DocStore, all_md: &[PathBuf], reporter: &mut Reporter) {
    let mut outcomes: HashMap<Vec<String>, Outcome> = HashMap::new();
    for_each_readable_record(store, all_md, |f, contents| {
        if !is_view(f, &contents) {
            return;
        }
        let body = strip_frontmatter(&contents);
        let frontmatter_lines = contents[..contents.len() - body.len()]
            .matches('\n')
            .count();
        match parse_regions(body) {
            Ok(regions) => {
                for region in &regions {
                    judge(store, f, frontmatter_lines, region, &mut outcomes, reporter);
                }
            }
            Err(error) => reporter.advise(
                f,
                kinds::STALE_VIEW,
                format!(
                    "a generated region cannot be checked: line {}: {}",
                    error.line + frontmatter_lines,
                    error.message
                ),
            ),
        }
    });
}

/// Whether `path` is a record of the architecture view type, in its directory.
pub(super) fn is_view(path: &Path, contents: &str) -> bool {
    let in_dir = path
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|dir| dir == VIEW_DIR);
    let declared = frontmatter_scalar(contents, "type")
        .and_then(|declared| doc_type::spec_for_frontmatter(&declared));
    in_dir
        && declared.is_some_and(|spec| {
            doc_type::spec_for_dir(VIEW_DIR).is_some_and(|view| std::ptr::eq(view, spec))
        })
}

/// Advises on `region` when its stamped hash differs from the current one.
fn judge(
    store: &dyn DocStore,
    view: &Path,
    frontmatter_lines: usize,
    region: &Region,
    outcomes: &mut HashMap<Vec<String>, Outcome>,
    reporter: &mut Reporter,
) {
    let Some(stamped) = &region.hash else {
        return;
    };
    let line = region.span.start - 1 + frontmatter_lines;
    let globs = region.inputs.join(", ");
    let outcome = outcomes
        .entry(region.inputs.clone())
        .or_insert_with(|| hash_of(store, &region.inputs));
    let message = match outcome {
        Outcome::NoRepository => return,
        Outcome::Hash(current) if current == stamped => return,
        Outcome::Hash(_) => format!(
            "the generated region at line {line} is stale: its inputs `{globs}` changed since it was stamped"
        ),
        Outcome::Failed(reason) => {
            format!("the generated region at line {line} cannot be checked: {reason}")
        }
    };
    let mut members = vec![view.to_path_buf()];
    members.extend(region.inputs.iter().map(PathBuf::from));
    reporter.advise_group(&members, kinds::STALE_VIEW, message);
}

fn hash_of(store: &dyn DocStore, globs: &[String]) -> Outcome {
    match inputs_hash(store, globs) {
        Ok(hash) => Outcome::Hash(hash),
        Err(InputsError::Unavailable) => Outcome::NoRepository,
        Err(error) => Outcome::Failed(error.to_string()),
    }
}

#[cfg(test)]
mod tests;
