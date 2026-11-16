//! The `link-style` advisory: a link or image written in the `/`-rooted form
//! resolves, but breaks when the bundle is read from another root, so each one
//! is advised with the relative form to write instead. The advisory never
//! depends on the target existing; a broken link stays its own violation.

use super::graph::normpath;
use super::links::{is_external, link_destinations};
use super::{kinds, Reporter};
use crate::store::DocStore;
use std::path::{Path, PathBuf};

pub(crate) fn check_link_style(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    for f in all_md {
        let Ok(content) = store.read(f) else {
            continue;
        };
        let from_dir = file_dir_in_bundle(f, bundle);
        for dest in link_destinations(&content) {
            if let Some(relative) = relative_form(&from_dir, &dest) {
                reporter.advise(
                    f,
                    kinds::LINK_STYLE,
                    format!("link `{dest}` is bundle-rooted; write `{relative}` instead"),
                );
            }
        }
    }
}

/// The linking file's directory as segments below the bundle root.
fn file_dir_in_bundle(file: &Path, bundle: &Path) -> Vec<String> {
    let inside = file.strip_prefix(bundle).unwrap_or(file);
    let mut segments: Vec<String> = inside
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    segments.pop();
    segments
}

/// The relative spelling of a single-`/` rooted, non-external destination, or
/// `None` for any other destination.
fn relative_form(from_dir: &[String], dest: &str) -> Option<String> {
    if !dest.starts_with('/') || dest.starts_with("//") || is_external(dest) {
        return None;
    }
    let (path, anchor) = match dest.find(['#', '?']) {
        Some(pos) => dest.split_at(pos),
        None => (dest, ""),
    };
    let target = normpath(path);
    let target: Vec<&str> = target.split('/').filter(|s| !s.is_empty()).collect();
    let shared = from_dir
        .iter()
        .zip(&target)
        .take_while(|(a, b)| a.as_str() == **b)
        .count();
    let ups = std::iter::repeat_n("..", from_dir.len() - shared);
    let down = target[shared..].iter().copied();
    let joined = ups.chain(down).collect::<Vec<_>>().join("/");
    let joined = if joined.is_empty() { "./" } else { &joined };
    Some(format!("{joined}{anchor}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(path: &str) -> Vec<String> {
        path.split('/')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn a_sibling_directory_target_climbs_one_level() {
        assert_eq!(
            relative_form(&dir("issues"), "/adr/0007-x.md").as_deref(),
            Some("../adr/0007-x.md")
        );
    }

    #[test]
    fn a_target_in_the_same_directory_has_no_climb() {
        assert_eq!(
            relative_form(&dir("adr"), "/adr/0007-x.md").as_deref(),
            Some("0007-x.md")
        );
    }

    #[test]
    fn a_target_below_the_linking_directory_descends() {
        assert_eq!(
            relative_form(&dir("a"), "/a/b/c.md#top").as_deref(),
            Some("b/c.md#top")
        );
    }

    #[test]
    fn a_target_that_is_the_linking_directory_is_advised_as_dot_slash() {
        assert_eq!(relative_form(&dir("adr"), "/adr/").as_deref(), Some("./"));
        assert_eq!(relative_form(&dir("adr"), "/adr").as_deref(), Some("./"));
        assert_eq!(
            relative_form(&dir("adr"), "/adr/#top").as_deref(),
            Some("./#top")
        );
        assert_eq!(relative_form(&dir(""), "/").as_deref(), Some("./"));
    }

    #[test]
    fn protocol_relative_and_relative_destinations_get_no_advice() {
        assert_eq!(relative_form(&dir("a"), "//cdn.example.com/x"), None);
        assert_eq!(relative_form(&dir("a"), "../b.md"), None);
    }
}
