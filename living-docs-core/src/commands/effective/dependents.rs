//! Who depends on a record: the live records whose body links it, directly
//! or through a chain of links, a retired record passing the chain on without
//! being listed. The link graph is built per run from the records `read`
//! already loaded and never persisted.

use super::Entry;
use crate::check::links::{link_destinations, resolve_destination};
use crate::store::DocStore;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Which dependents of a record the view is narrowed to.
pub enum Dependents {
    /// The live records whose body links the record directly.
    LinksTo(PathBuf),
    /// Every live record that reaches the record through a chain of links.
    Impact(PathBuf),
}

/// Resolves `reference` (`TYPE/NNNN`) to a record path the way `set` does.
pub fn resolve_target(
    store: &dyn DocStore,
    bundle: &Path,
    reference: &str,
) -> Result<PathBuf, String> {
    crate::commands::reference::find_record(store, bundle, reference)
}

/// Narrows `ranked` (record indexes in reading order) to the dependents the
/// query names. Under `Impact` each carries its distance and the list is
/// ordered by it, then by the incoming reading order.
pub(super) fn narrow(
    records: &[Entry],
    bundle: &Path,
    query: &Dependents,
    ranked: Vec<usize>,
) -> Vec<(usize, Option<usize>)> {
    let (target, depth, with_distance) = match query {
        Dependents::LinksTo(path) => (path, Some(1), false),
        Dependents::Impact(path) => (path, None, true),
    };
    let inbound = inbound_links(records, bundle);
    let Some(start) = index_of(records, target) else {
        return Vec::new();
    };
    let distances = distances_from(&inbound, start, depth);
    let mut found: Vec<(usize, usize)> = ranked
        .into_iter()
        .filter_map(|index| Some((index, *distances.get(&index)?)))
        .collect();
    if with_distance {
        found.sort_by_key(|(_, distance)| *distance);
    }
    found
        .into_iter()
        .map(|(index, distance)| (index, with_distance.then_some(distance)))
        .collect()
}

/// The path in the normalized form `resolve_destination` returns, so a
/// record path and a resolved link compare equal.
fn key_of(path: &Path) -> String {
    let name = path.file_name().map(|n| n.to_string_lossy());
    resolve_destination(&path.to_string_lossy(), name.as_deref().unwrap_or(""), "")
        .unwrap_or_default()
}

fn index_of(records: &[Entry], target: &Path) -> Option<usize> {
    let wanted = key_of(target);
    records.iter().position(|(path, _)| key_of(path) == wanted)
}

/// For each record, the records whose body links it, retired ones included so
/// a chain passes through them. A link counts when `check` resolves it to
/// another record of the bundle.
fn inbound_links(records: &[Entry], bundle: &Path) -> HashMap<usize, Vec<usize>> {
    let by_path: HashMap<String, usize> = records
        .iter()
        .enumerate()
        .map(|(index, (path, _))| (key_of(path), index))
        .collect();
    let bundle = bundle.to_string_lossy();
    let mut inbound: HashMap<usize, Vec<usize>> = HashMap::new();
    for (source, (path, record)) in records.iter().enumerate() {
        let file = path.to_string_lossy();
        for destination in link_destinations(&record.body) {
            let resolved = resolve_destination(&file, &destination, &bundle);
            let target = resolved.and_then(|resolved| by_path.get(&resolved).copied());
            if let Some(target) = target.filter(|target| *target != source) {
                inbound.entry(target).or_default().push(source);
            }
        }
    }
    inbound
}

/// Breadth-first walk over inbound links from `start`: each reached record
/// once, at its shortest distance, the start itself excluded.
fn distances_from(
    inbound: &HashMap<usize, Vec<usize>>,
    start: usize,
    max_depth: Option<usize>,
) -> HashMap<usize, usize> {
    let mut distances = HashMap::new();
    let mut frontier = vec![start];
    let mut depth = 0;
    while !frontier.is_empty() && max_depth.is_none_or(|max| depth < max) {
        depth += 1;
        let mut next = Vec::new();
        for node in frontier {
            for &source in inbound.get(&node).into_iter().flatten() {
                if source != start && !distances.contains_key(&source) {
                    distances.insert(source, depth);
                    next.push(source);
                }
            }
        }
        frontier = next;
    }
    distances
}
