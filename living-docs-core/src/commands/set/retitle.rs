//! The `title` key of `set`: one transaction over the frontmatter
//! `title`, the record's heading line, its filename, and every in-bundle
//! reference to the old filename. A record whose status is terminal for its
//! type, or `Superseded`, is refused — its title is history.

use super::resolve_spec;
use crate::commands::supersede::set_frontmatter_fields;
use crate::doc_type::{DocTypeSpec, Identity};
use crate::paths::slugify;
use crate::record::{extract_record, format_scalar};
use crate::store::DocStore;
use references::rewrite_references;
use std::path::{Path, PathBuf};

/// What a retitle did: where the record now lives, and which other bundle
/// files had a reference to the old filename rewritten.
#[derive(Debug)]
pub(crate) struct Retitled {
    pub(crate) path: PathBuf,
    pub(crate) rewritten: Vec<PathBuf>,
}

pub(crate) fn apply(
    store: &dyn DocStore,
    docs_dir: &Path,
    path: &Path,
    title: &str,
) -> Result<Retitled, String> {
    let contents = store.read(path).map_err(|e| e.to_string())?;
    let spec = resolve_spec(path, &contents)?;
    guard_status(path, &contents, spec)?;
    let target = target_path(spec, path, title)?;
    if let Some(target) = target.as_ref() {
        refuse_collision(store, docs_dir, target)?;
    }

    set_frontmatter_fields(store, path, &[("title", format_scalar(title))])?;
    rewrite_heading(store, path, title)?;

    let Some(target) = target else {
        return Ok(Retitled {
            path: path.to_path_buf(),
            rewritten: Vec::new(),
        });
    };
    store.rename(path, &target).map_err(|e| e.to_string())?;
    let rewritten = rewrite_references(store, docs_dir, path, &target)?;
    Ok(Retitled {
        path: target,
        rewritten,
    })
}

/// A record closed for good keeps the title it was closed under: `supersede`
/// is the verb that revises a decision, and a `Superseded` record is already
/// cited by its successor's callout.
fn guard_status(path: &Path, contents: &str, spec: &DocTypeSpec) -> Result<(), String> {
    let Some(status) = extract_record(path, contents).status else {
        return Ok(());
    };
    if !spec.is_retired(&status) {
        return Ok(());
    }
    Err(format!(
        "{}: status '{status}' is terminal; a closed record keeps its title — revise it with `living-docs supersede <old> <new>`",
        path.display()
    ))
}

/// Where the retitled record belongs: a numbered record keeps its number and
/// takes the new slug, a named record is its slug, and a singleton has a
/// fixed filename that no title can move.
fn target_path(spec: &DocTypeSpec, path: &Path, title: &str) -> Result<Option<PathBuf>, String> {
    let slug = slugify(title);
    if slug.is_empty() {
        return Err(format!(
            "'{title}' has no slug; a title needs alphanumerics"
        ));
    }
    let dir = path.parent().unwrap_or(Path::new(""));
    let target = match spec.identity {
        Identity::Singleton { .. } => return Ok(None),
        Identity::Named { .. } => dir.join(format!("{slug}.md")),
        Identity::Numbered { .. } => dir.join(format!("{}-{slug}.md", number_prefix(path)?)),
    };
    Ok((target.as_path() != path).then_some(target))
}

pub(crate) fn number_prefix(path: &Path) -> Result<String, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    name.split('-')
        .next()
        .filter(|prefix| prefix.len() == 4 && prefix.chars().all(|c| c.is_ascii_digit()))
        .map(str::to_owned)
        .ok_or_else(|| format!("{}: filename carries no NNNN- number", path.display()))
}

fn refuse_collision(store: &dyn DocStore, docs_dir: &Path, target: &Path) -> Result<(), String> {
    let taken = store
        .list(docs_dir)
        .map_err(|e| e.to_string())?
        .iter()
        .any(|path| path == target);
    if taken {
        return Err(format!(
            "{}: a record already holds that slug; choose another title",
            target.display()
        ));
    }
    Ok(())
}

/// Rewrites the record's own heading line to the new title, keeping the
/// heading level and the `NNNN.` number prefix the template writes.
fn rewrite_heading(store: &dyn DocStore, path: &Path, title: &str) -> Result<(), String> {
    let contents = store.read(path).map_err(|e| e.to_string())?;
    let Some(updated) = replace_heading(&contents, title) else {
        return Ok(());
    };
    store.write(path, &updated).map_err(|e| e.to_string())
}

/// The record's heading line rewritten to `title`, or `None` when the body
/// carries no heading. Every other byte, line endings included, is kept.
/// Pure: every caller reads and writes around it.
pub(crate) fn replace_heading(contents: &str, title: &str) -> Option<String> {
    let lines = split_lines(contents);
    let index = heading_index(&lines, |_| true)?;
    Some(retitle_line(lines, index, title))
}

/// [`replace_heading`] restricted to the record's own title heading, as
/// [`title_heading_index`] finds it. `None` when the body has none, so a
/// section heading is never renamed. A heading without a number prefix gets
/// only its text replaced; no number is added.
pub(crate) fn replace_title_heading(
    contents: &str,
    number: Option<&str>,
    title: &str,
) -> Option<String> {
    let lines = split_lines(contents);
    let index = title_heading_index(&lines, number)?;
    Some(retitle_line(lines, index, title))
}

/// The record's own title heading: the first heading outside a fenced block
/// opening with `number`, else the first exact level-1 heading without a
/// number, else none.
fn title_heading_index(lines: &[String], number: Option<&str>) -> Option<usize> {
    number
        .and_then(|number| heading_index(lines, |line| heading_number(line) == Some(number)))
        .or_else(|| {
            heading_index(lines, |line| {
                line.starts_with("# ") && heading_number(line).is_none()
            })
        })
}

fn retitle_line(mut lines: Vec<String>, index: usize, title: &str) -> String {
    let heading = lines[index].trim_end_matches(['\r', '\n']);
    let ending = &lines[index][heading.len()..];
    let hashes = heading
        .chars()
        .take_while(|c| *c == '#')
        .collect::<String>();
    lines[index] = match heading_number(heading) {
        Some(number) => format!("{hashes} {number}. {title}{ending}"),
        None => format!("{hashes} {title}{ending}"),
    };
    lines.concat()
}

/// The text of the record's own title heading, as [`replace_title_heading`]
/// finds it, without its `#` marks and `NNNN.` prefix.
pub(crate) fn title_heading_text(contents: &str, number: Option<&str>) -> Option<String> {
    let lines = split_lines(contents);
    let index = title_heading_index(&lines, number)?;
    Some(text_of(lines[index].trim_end()))
}

fn text_of(heading: &str) -> String {
    let rest = heading.trim_start_matches('#').trim_start();
    let text = match heading_number(heading) {
        Some(_) => rest.split_once('.').map_or(rest, |(_, text)| text),
        None => rest,
    };
    text.trim().to_owned()
}

fn split_lines(contents: &str) -> Vec<String> {
    contents.split_inclusive('\n').map(str::to_owned).collect()
}

/// The index of the first heading line below the frontmatter that `accepts`,
/// skipping fenced code blocks.
fn heading_index(lines: &[String], accepts: impl Fn(&str) -> bool) -> Option<usize> {
    let body_start = match lines.first().map(|line| line.trim_end()) {
        Some("---") => lines.iter().skip(1).position(|l| l.trim_end() == "---")? + 2,
        _ => 0,
    };
    let mut fenced = false;
    for (index, line) in lines.iter().enumerate().skip(body_start) {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
        } else if !fenced && line.starts_with('#') && accepts(line) {
            return Some(index);
        }
    }
    None
}

/// The `NNNN` a numbered record's heading opens with, as written — `# 0061.
/// Title` yields `0061`. `None` for a heading with no number prefix.
fn heading_number(heading: &str) -> Option<&str> {
    let rest = heading.trim_start_matches('#').trim_start();
    let number = rest.split('.').next()?;
    (number.len() == 4 && number.chars().all(|c| c.is_ascii_digit())).then_some(number)
}

mod references;

#[cfg(test)]
mod tests;
