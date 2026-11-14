//! Native local-link validity (S5) — replaces lychee. Every markdown link AND
//! image destination is extracted via `pulldown-cmark` (inline / titled /
//! angle-bracket / reference-style, fence-aware by construction: fenced code
//! is parsed as a `CodeBlock` event, never as `Link`/`Image` tags).
//!
//! Resolution mirrors `resolve_link`/`normpath` in `lint-docs.sh` and
//! `check::graph`: strip the anchor, skip external/mailto/tel targets, join
//! bundle-relative (leading `/`) against the bundle root and everything else
//! against the linking file's directory, then normalize and check existence.
//!
//! Each linking file's own content is read through `DocStore::read`; the
//! resolved destination's existence check stays on the filesystem — it may
//! point at a non-record asset (an image, say) that the `DocStore` port
//! never models.

use super::graph::{dirname_str, normpath};
use super::Reporter;
use crate::store::DocStore;
use pulldown_cmark::{Event, Parser, Tag};
use std::path::{Path, PathBuf};

pub(crate) fn check_links(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    let bundle_str = bundle.to_string_lossy();
    for f in all_md {
        check_file_links(store, f, &bundle_str, reporter);
    }
}

fn check_file_links(store: &dyn DocStore, f: &Path, bundle: &str, reporter: &mut Reporter) {
    let Ok(content) = store.read(f) else {
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
pub(crate) fn link_destinations(content: &str) -> Vec<String> {
    link_starts(content)
        .into_iter()
        .map(|(_, _, dest)| dest)
        .collect()
}

/// Whether a destination came from a link or from an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkKind {
    Link,
    Image,
}

/// Every `Link`/`Image` outside code as the byte offset where it starts in
/// `content`, its kind and its destination, in document order.
pub(crate) fn link_starts(content: &str) -> Vec<(usize, LinkKind, String)> {
    Parser::new(content)
        .into_offset_iter()
        .filter_map(|(event, range)| match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                Some((range.start, LinkKind::Link, dest_url.into_string()))
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                Some((range.start, LinkKind::Image, dest_url.into_string()))
            }
            _ => None,
        })
        .collect()
}

/// Resolves a raw destination to a normalized local path, or `None` if it's
/// external / a pure anchor / unsupported.
pub(crate) fn resolve_destination(file: &str, raw_dest: &str, bundle: &str) -> Option<String> {
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
    use crate::test_support::MapStore;
    use std::process::ExitCode;

    fn exit_code_is_success(code: ExitCode) -> bool {
        format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
    }

    #[test]
    fn check_file_links_reads_content_the_store_serves_with_no_disk_backing() {
        let (store, _all_md) =
            MapStore::seeded(&[("/bundle/adr/0001.md", "[missing](./0099-missing.md)\n")]);
        let mut reporter = Reporter::new();

        check_file_links(
            &store,
            Path::new("/bundle/adr/0001.md"),
            "/bundle",
            &mut reporter,
        );

        assert!(!exit_code_is_success(reporter.finish()));
    }

    #[test]
    fn check_file_links_is_a_no_op_when_the_store_has_no_content_at_the_path() {
        let (store, _all_md) = MapStore::seeded(&[]);
        let mut reporter = Reporter::new();

        check_file_links(
            &store,
            Path::new("/bundle/adr/0001.md"),
            "/bundle",
            &mut reporter,
        );

        assert!(exit_code_is_success(reporter.finish()));
    }

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
    fn link_starts_report_each_link_offset_for_inline_reference_and_autolink_outside_code() {
        let content =
            "see [a](./a.md) and [b][r] <https://x.io>\n\n[r]: ./r.md\n\n```\n[f](./f.md)\n```\n";
        let got = link_starts(content);
        let expected = vec![
            (
                content.find("[a]").unwrap(),
                LinkKind::Link,
                "./a.md".to_string(),
            ),
            (
                content.find("[b]").unwrap(),
                LinkKind::Link,
                "./r.md".to_string(),
            ),
            (
                content.find("<https").unwrap(),
                LinkKind::Link,
                "https://x.io".to_string(),
            ),
        ];
        assert_eq!(got, expected);
    }

    #[test]
    fn link_starts_report_an_inline_image_as_an_image_at_its_bang() {
        let content = "see ![i](./i.png) and [a](./a.md)";
        let expected = vec![
            (
                content.find("![i]").unwrap(),
                LinkKind::Image,
                "./i.png".to_string(),
            ),
            (
                content.find("[a]").unwrap(),
                LinkKind::Link,
                "./a.md".to_string(),
            ),
        ];
        assert_eq!(link_starts(content), expected);
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
