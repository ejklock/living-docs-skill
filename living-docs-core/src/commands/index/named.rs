//! Listing body for a Named-identity directory (ADR 0036): one row per
//! view file, sorted by the C4/arc42 zoom rank of its `kind` frontmatter
//! (`doc_type::VIEW_KIND_ORDER`), then filename; an absent or unknown kind
//! ranks after every listed one.

use super::first_heading;
use super::rows::single_line;
use crate::doc_type::VIEW_KIND_ORDER;
use crate::frontmatter;
use crate::paths::link_destination;
use crate::store::DocStore;
use std::path::Path;

struct View {
    rank: usize,
    filename: String,
    title: String,
    kind: String,
}

pub(super) fn render_body(
    store: &dyn DocStore,
    docs_dir: &Path,
    type_dir: &Path,
) -> Result<String, String> {
    let mut views = collect_views(store, docs_dir, type_dir)?;
    views.sort_by(|a, b| (a.rank, &a.filename).cmp(&(b.rank, &b.filename)));
    if views.is_empty() {
        return Ok(String::new());
    }
    let rows: Vec<String> = views.iter().map(render_row).collect();
    Ok(rows.join("\n") + "\n")
}

fn collect_views(
    store: &dyn DocStore,
    docs_dir: &Path,
    type_dir: &Path,
) -> Result<Vec<View>, String> {
    let paths = store.list(docs_dir).map_err(|e| e.to_string())?;
    Ok(paths
        .iter()
        .filter(|path| path.parent() == Some(type_dir))
        .filter_map(|path| view_from_path(store, path))
        .collect())
}

fn view_from_path(store: &dyn DocStore, path: &Path) -> Option<View> {
    let filename = path.file_name()?.to_str()?.to_string();
    if filename == "index.md" {
        return None;
    }
    let contents = store.read(path).ok()?;
    let kind = frontmatter::read_scalar_from_str(&contents, "kind").unwrap_or_default();
    Some(View {
        rank: kind_rank(&kind),
        title: view_title(&contents, &filename),
        kind,
        filename,
    })
}

fn kind_rank(kind: &str) -> usize {
    VIEW_KIND_ORDER
        .iter()
        .position(|listed| *listed == kind)
        .unwrap_or(VIEW_KIND_ORDER.len())
}

fn view_title(contents: &str, filename: &str) -> String {
    frontmatter::read_scalar_from_str(contents, "title")
        .or_else(|| first_heading(contents))
        .unwrap_or_else(|| filename.trim_end_matches(".md").to_string())
}

fn render_row(view: &View) -> String {
    let title = single_line(&view.title);
    let destination = link_destination(&view.filename);
    if view.kind.is_empty() {
        return format!("* [{title}]({destination})");
    }
    format!("* [{title}]({destination}) - {}", view.kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MapStore;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn view_doc(title: &str, kind: &str) -> String {
        format!("---\ntype: Architecture View\ntitle: {title}\nkind: {kind}\n---\n\n# {title}\n")
    }

    fn store_with(files: Vec<(&str, String)>) -> MapStore {
        MapStore {
            files: files
                .into_iter()
                .map(|(path, contents)| (PathBuf::from(path), contents))
                .collect::<BTreeMap<_, _>>(),
        }
    }

    #[test]
    fn rows_sort_by_c4_zoom_rank_not_alphabetically() {
        let store = store_with(vec![
            (
                "/d/architecture/modules.md",
                view_doc("Modules", "component"),
            ),
            ("/d/architecture/context.md", view_doc("Context", "context")),
            (
                "/d/architecture/backends.md",
                view_doc("Backends", "container"),
            ),
        ]);

        let body = render_body(&store, Path::new("/d"), Path::new("/d/architecture"))
            .expect("render must succeed");

        assert_eq!(
            body,
            "* [Context](context.md) - context\n\
             * [Backends](backends.md) - container\n\
             * [Modules](modules.md) - component\n"
        );
    }

    #[test]
    fn an_unknown_or_absent_kind_sorts_last_by_filename() {
        let store = store_with(vec![
            ("/d/architecture/zz.md", view_doc("Zz", "context")),
            ("/d/architecture/mystery.md", view_doc("Mystery", "exotic")),
            (
                "/d/architecture/bare.md",
                "---\ntype: Architecture View\ntitle: Bare\n---\n\n# Bare\n".to_string(),
            ),
        ]);

        let body = render_body(&store, Path::new("/d"), Path::new("/d/architecture"))
            .expect("render must succeed");

        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(lines[0], "* [Zz](zz.md) - context");
        assert_eq!(lines[1], "* [Bare](bare.md)");
        assert_eq!(lines[2], "* [Mystery](mystery.md) - exotic");
    }

    #[test]
    fn a_view_title_holding_a_line_break_renders_on_one_line() {
        let store = store_with(vec![(
            "/d/architecture/context.md",
            "---\ntype: Architecture View\ntitle: \"a\\nb\"\nkind: context\n---\n\n# A\n"
                .to_string(),
        )]);

        let body = render_body(&store, Path::new("/d"), Path::new("/d/architecture"))
            .expect("render must succeed");

        assert_eq!(body, "* [a b](context.md) - context\n");
    }

    #[test]
    fn the_directory_index_itself_is_never_a_row() {
        let store = store_with(vec![
            ("/d/architecture/index.md", "# Architecture\n".to_string()),
            ("/d/architecture/context.md", view_doc("Context", "context")),
        ]);

        let body = render_body(&store, Path::new("/d"), Path::new("/d/architecture"))
            .expect("render must succeed");

        assert_eq!(body, "* [Context](context.md) - context\n");
    }

    #[test]
    fn every_registry_kind_outranks_an_unlisted_one() {
        for kind in VIEW_KIND_ORDER {
            assert!(kind_rank(kind) < kind_rank("unlisted"));
        }
        assert_eq!(kind_rank(""), VIEW_KIND_ORDER.len());
    }
}
