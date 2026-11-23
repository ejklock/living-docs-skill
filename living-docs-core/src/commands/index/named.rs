//! Listing body for a Named-identity directory: one row per
//! view file, sorted by the C4/arc42 zoom rank of its `kind` frontmatter
//! (`doc_type::VIEW_KIND_ORDER`), then filename; an absent or unknown kind
//! ranks after every listed one. A Nested directory lists its records at any
//! depth by filename alone, with no kind: root-level rows first, then one
//! `## <folder/path>` heading per subfolder, in path order; each row ends with
//! ` — <description>` when the record has a real one.

mod description;

use super::first_heading;
use super::rows::single_line;
use crate::doc_type::VIEW_KIND_ORDER;
use crate::frontmatter;
use crate::paths::link_destination;
use crate::store::DocStore;
use description::real_description;
use std::path::Path;

struct View {
    /// The `/`-separated folder below the type directory; empty at its root.
    folder: String,
    rank: usize,
    filename: String,
    title: String,
    kind: String,
    /// The frontmatter description, empty when absent or still a doc type's
    /// template sentence; only a Nested listing renders it.
    description: String,
}

pub(super) fn render_body(
    store: &dyn DocStore,
    docs_dir: &Path,
    type_dir: &Path,
    ranked: bool,
) -> Result<String, String> {
    let mut views = collect_views(store, docs_dir, type_dir, !ranked)?;
    if ranked {
        views.iter_mut().for_each(|view| view.description.clear());
        views.sort_by(|a, b| (a.rank, &a.filename).cmp(&(b.rank, &b.filename)));
        return Ok(join_rows(&views));
    }
    views.sort_by(|a, b| {
        (a.folder.split('/'))
            .cmp(b.folder.split('/'))
            .then_with(|| a.filename.cmp(&b.filename))
    });
    views.iter_mut().for_each(|view| view.kind.clear());
    Ok(render_folders(&views))
}

fn join_rows(views: &[View]) -> String {
    if views.is_empty() {
        return String::new();
    }
    let rows: Vec<String> = views.iter().map(render_row).collect();
    rows.join("\n") + "\n"
}

/// Rows of the views sorted by folder: the root's first, then a `## folder`
/// heading above each subfolder's rows.
fn render_folders(views: &[View]) -> String {
    let mut out = String::new();
    let mut current = "";
    for view in views {
        if view.folder != current {
            current = &view.folder;
            let gap = if out.is_empty() { "" } else { "\n" };
            out.push_str(&format!("{gap}## {current}\n\n"));
        }
        out.push_str(&render_row(view));
        out.push('\n');
    }
    out
}

fn collect_views(
    store: &dyn DocStore,
    docs_dir: &Path,
    type_dir: &Path,
    recursive: bool,
) -> Result<Vec<View>, String> {
    let paths = store.list(docs_dir).map_err(|e| e.to_string())?;
    Ok(paths
        .iter()
        .filter(|path| recursive || path.parent() == Some(type_dir))
        .filter_map(|path| view_from_path(store, path, type_dir))
        .collect())
}

/// The `/`-separated folder of `path` below `type_dir`, `None` when it lies
/// outside it.
fn folder_below(path: &Path, type_dir: &Path) -> Option<String> {
    let parent = path.parent()?.strip_prefix(type_dir).ok()?;
    let parts: Vec<&str> = parent.iter().filter_map(|part| part.to_str()).collect();
    Some(parts.join("/"))
}

fn view_from_path(store: &dyn DocStore, path: &Path, type_dir: &Path) -> Option<View> {
    let folder = folder_below(path, type_dir)?;
    let filename = path.file_name()?.to_str()?.to_string();
    if filename == "index.md" {
        return None;
    }
    let contents = store.read(path).ok()?;
    let kind = frontmatter::read_scalar_from_str(&contents, "kind").unwrap_or_default();
    Some(View {
        folder,
        rank: kind_rank(&kind),
        title: view_title(&contents, &filename),
        description: real_description(&contents),
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
    let relative = match view.folder.is_empty() {
        true => view.filename.clone(),
        false => format!("{}/{}", view.folder, view.filename),
    };
    let destination = link_destination(&relative);
    if !view.description.is_empty() {
        return format!("* [{title}]({destination}) — {}", view.description);
    }
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

        let body = render_body(&store, Path::new("/d"), Path::new("/d/architecture"), true)
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

        let body = render_body(&store, Path::new("/d"), Path::new("/d/architecture"), true)
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

        let body = render_body(&store, Path::new("/d"), Path::new("/d/architecture"), true)
            .expect("render must succeed");

        assert_eq!(body, "* [a b](context.md) - context\n");
    }

    #[test]
    fn the_directory_index_itself_is_never_a_row() {
        let store = store_with(vec![
            ("/d/architecture/index.md", "# Architecture\n".to_string()),
            ("/d/architecture/context.md", view_doc("Context", "context")),
        ]);

        let body = render_body(&store, Path::new("/d"), Path::new("/d/architecture"), true)
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

    #[test]
    fn an_unranked_listing_sorts_by_filename_and_shows_no_kind() {
        let store = store_with(vec![
            ("/d/context/b.md", view_doc("B", "context")),
            ("/d/context/a.md", view_doc("A", "component")),
        ]);

        let body = render_body(&store, Path::new("/d"), Path::new("/d/context"), false)
            .expect("render must succeed");

        assert_eq!(body, "* [A](a.md)\n* [B](b.md)\n");
    }
}
