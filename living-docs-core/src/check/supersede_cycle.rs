//! The `supersede-cycle` advisory: records whose `superseded_by` links loop
//! back on themselves, which a hand edit can write and the supersede verb
//! refuses. A chain that ends in a retired record, or at a number that names
//! no record, is not a cycle. The finding is advice and never fails the check.

use super::records::for_each_readable_record;
use super::{kinds, Reporter};
use crate::lifecycle::{successor_number, supersede_cycles};
use crate::record::extract_record;
use crate::store::DocStore;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The records of one directory: where each numbered record lives and which
/// record number it names as its successor.
#[derive(Default)]
struct Directory {
    paths: BTreeMap<i32, PathBuf>,
    successor_of: BTreeMap<i32, i32>,
}

/// Advises once per cycle of `superseded_by` links among the records of a
/// directory, anchored on every member so a scope holding any one of them
/// keeps it, and naming each member.
pub(crate) fn check_supersede_cycle(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    for directory in read_directories(store, all_md).values() {
        for cycle in supersede_cycles(&directory.successor_of) {
            let members: Vec<PathBuf> = cycle
                .iter()
                .filter_map(|number| directory.paths.get(number).cloned())
                .collect();
            let names: Vec<String> = members.iter().map(|p| relative(bundle, p)).collect();
            let message = format!(
                "records name one another in a cycle through superseded_by: {}",
                names.join(", ")
            );
            reporter.advise_group(&members, kinds::SUPERSEDE_CYCLE, message);
        }
    }
}

fn relative(bundle: &Path, path: &Path) -> String {
    path.strip_prefix(bundle)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Every numbered record of `all_md` that the store can read, grouped by the
/// directory that holds it.
fn read_directories(store: &dyn DocStore, all_md: &[PathBuf]) -> BTreeMap<PathBuf, Directory> {
    let mut directories: BTreeMap<PathBuf, Directory> = BTreeMap::new();
    for_each_readable_record(store, all_md, |path, contents| {
        let record = extract_record(path, &contents);
        let Some(number) = record.number else {
            return;
        };
        let directory = directories
            .entry(path.parent().unwrap_or(path).to_path_buf())
            .or_default();
        directory.paths.insert(number, path.to_path_buf());
        if let Some(successor) = successor_number(&record) {
            directory.successor_of.insert(number, successor);
        }
    });
    directories
}
