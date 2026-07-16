//! Native local-link validity (S5) — replaces lychee. Every markdown link AND
//! image destination is extracted via `pulldown-cmark` (inline / titled /
//! angle-bracket / reference-style, fence-aware by construction: fenced code
//! is parsed as a `CodeBlock` event, never as `Link`/`Image` tags).
//!
//! Resolution mirrors `resolve_link`/`normpath` in `lint-docs.sh` and
//! `check::graph`: strip the anchor, skip external/mailto/tel targets, join
//! bundle-relative (leading `/`) against the bundle root and everything else
//! against the linking file's directory, then normalize and check existence.

use super::graph::{dirname_str, normpath};
use super::Reporter;
use pulldown_cmark::{Event, Parser, Tag};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn check_links(bundle: &Path, all_md: &[PathBuf], reporter: &mut Reporter) {
    let bundle_str = bundle.to_string_lossy();
    for f in all_md {
        check_file_links(f, &bundle_str, reporter);
    }
}

fn check_file_links(f: &Path, bundle: &str, reporter: &mut Reporter) {
    let Ok(content) = fs::read_to_string(f) else {
        return;
    };
    let file_str = f.to_string_lossy();
    for dest in link_destinations(&content) {
        let Some(target) = resolve_destination(&file_str, &dest, bundle) else {
            continue;
        };
        if !Path::new(&target).exists() {
            reporter.report(f, format!("broken link -> {target}"));
        }
    }
}

/// Every `Link`/`Image` destination `pulldown-cmark` finds, in document order.
/// Fenced code blocks are parsed as `CodeBlock` events, so example links shown
/// inside them never surface here.
fn link_destinations(content: &str) -> Vec<String> {
    Parser::new(content)
        .filter_map(|event| match event {
            Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => {
                Some(dest_url.into_string())
            }
            _ => None,
        })
        .collect()
}

/// Resolves a raw destination to a normalized local path, or `None` if it's
/// external / a pure anchor / unsupported.
fn resolve_destination(file: &str, raw_dest: &str, bundle: &str) -> Option<String> {
    let target = strip_anchor(raw_dest);
    if target.is_empty() || is_external(target) {
        return None;
    }
    let joined = if let Some(rest) = target.strip_prefix('/') {
        format!("{bundle}/{rest}")
    } else {
        format!("{}/{}", dirname_str(file), target)
    };
    Some(normpath(&joined))
}

fn strip_anchor(raw_dest: &str) -> &str {
    match raw_dest.find('#') {
        Some(pos) => &raw_dest[..pos],
        None => raw_dest,
    }
}

fn is_external(target: &str) -> bool {
    target.contains("://") || target.starts_with("mailto:") || target.starts_with("tel:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_destinations_ignores_fenced_code_and_extracts_every_link_form() {
        let content = "\
[a](./bar.md \"Title\")
[b](<./bar.md>)
[c](./bar.md)
[d][ref]
![img](./pic.png)

[ref]: ./bar.md

```md
[fenced](./should-not-appear.md)
```
";
        let dests = link_destinations(content);
        assert!(dests.iter().all(|d| d != "./should-not-appear.md"));
        assert!(dests.contains(&"./pic.png".to_string()));
        assert_eq!(dests.iter().filter(|d| *d == "./bar.md").count(), 4);
    }

    #[test]
    fn resolve_destination_skips_external_mailto_tel_and_pure_anchor_targets() {
        assert_eq!(
            resolve_destination("docs/index.md", "https://example.com/x", "docs"),
            None
        );
        assert_eq!(
            resolve_destination("docs/index.md", "mailto:a@b.com", "docs"),
            None
        );
        assert_eq!(
            resolve_destination("docs/index.md", "tel:+15551234567", "docs"),
            None
        );
        assert_eq!(
            resolve_destination("docs/index.md", "#section", "docs"),
            None
        );
    }

    #[test]
    fn resolve_destination_resolves_bundle_relative_file_relative_and_strips_anchor() {
        assert_eq!(
            resolve_destination("docs/a/index.md", "/b/index.md#top", "docs"),
            Some("docs/b/index.md".to_string())
        );
        assert_eq!(
            resolve_destination("docs/a/index.md", "./c.md", "docs"),
            Some("docs/a/c.md".to_string())
        );
    }
}
