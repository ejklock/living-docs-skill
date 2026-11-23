//! `manage view <view> --splice`: writes Mermaid text into an architecture
//! view's generated region and stamps the region with its input globs and
//! their hash. A view with no region gets one at the end of its body; a view
//! with one has that region replaced; a view with several is refused. Text
//! outside the region is never changed, and the command a marker names is
//! text the binary never runs.

use crate::commands::index::{refresh_for_records, Indexed};
use crate::commands::reference::find_settable_record;
use crate::doc_type::{self, DocTypeSpec};
use crate::frontmatter::{has_unparseable_frontmatter, read_scalar_from_str, UNPARSEABLE_MESSAGE};
use crate::generated_region::{parse_regions, render_closing, render_opening, Region};
use crate::inputs_hash::{inputs_hash, InputsError};
use crate::line_ending::{refuse_mixed, LineEndingStore};
use crate::record::strip_frontmatter;
use crate::store::{DocStore, GlobReach};
use input::{check_command, check_globs, fenced};
use std::ops::Range;
use std::path::{Path, PathBuf};

/// What to splice: the view reference, the input globs, the marker's command
/// (`None` keeps the existing region's) and the Mermaid text.
pub struct Request<'a> {
    pub view: &'a str,
    pub inputs: &'a [String],
    pub command: Option<&'a str>,
    pub mermaid: &'a str,
}

/// What a splice did: the view's path, whether it created the region, the
/// globs that matched nothing, and the index refresh.
#[derive(Debug)]
pub struct Spliced {
    pub path: PathBuf,
    pub created: bool,
    pub warnings: Vec<String>,
    pub indexed: Indexed,
}

/// The region a view already holds and the line its opening marker sits on,
/// counted from the start of the record.
struct Existing {
    region: Region,
    opening_line: usize,
}

const NOT_A_VIEW: &str = "splice writes a generated region into an architecture view only";

/// Splices `request.mermaid` into the generated region of the architecture
/// view `request.view` names and stamps the region. Nothing is written when
/// the view is unresolvable or is not an architecture view, the input holds
/// no Mermaid text, the globs are refused, or the view holds several regions.
/// A second identical call writes no byte.
pub fn apply(store: &dyn DocStore, docs_dir: &Path, request: &Request) -> Result<Spliced, String> {
    let (path, spec) = find_settable_record(store, docs_dir, request.view)?;
    refuse_mixed(store, &path)?;
    let store: &dyn DocStore = &LineEndingStore::new(store, false);
    let contents = store.read(&path).map_err(|e| e.to_string())?;
    require_view(&path, spec, &contents, request.view)?;
    let existing = existing_region(&contents)?;
    let (region, warnings) = region_of(store, request, existing.as_ref())?;
    let updated = place(&contents, existing.as_ref(), &region);
    if updated != contents {
        store.write(&path, &updated).map_err(|e| e.to_string())?;
    }
    let indexed = refresh_for_records(store, docs_dir, &[path.as_path()]);
    let created = existing.is_none();
    Ok(Spliced {
        path,
        created,
        warnings,
        indexed,
    })
}

/// Refuses a record that is not an architecture view, by its reference's type
/// and by the `type:` its own frontmatter declares.
fn require_view(
    path: &Path,
    spec: Option<&DocTypeSpec>,
    contents: &str,
    reference: &str,
) -> Result<(), String> {
    let view = doc_type::spec_for_dir("architecture");
    let declared = read_scalar_from_str(contents, "type")
        .and_then(|declared| doc_type::spec_for_frontmatter(&declared));
    let is_view = |found: Option<&DocTypeSpec>| match (found, view) {
        (Some(found), Some(view)) => std::ptr::eq(found, view),
        _ => false,
    };
    if !is_view(declared) || (spec.is_some() && !is_view(spec)) {
        return Err(format!(
            "{reference} is not an architecture view; {NOT_A_VIEW}"
        ));
    }
    if has_unparseable_frontmatter(contents) {
        return Err(format!("{}: {UNPARSEABLE_MESSAGE}", path.display()));
    }
    Ok(())
}

/// The one generated region of `contents`, refusing several.
fn existing_region(contents: &str) -> Result<Option<Existing>, String> {
    let body = strip_frontmatter(contents);
    let offset = contents[..contents.len() - body.len()]
        .matches('\n')
        .count();
    let regions = parse_regions(body)
        .map_err(|error| format!("line {}: {}", error.line + offset, error.message))?;
    let opening = |region: &Region| region.span.start - 1 + offset;
    if regions.len() > 1 {
        let lines: Vec<String> = regions.iter().map(|r| opening(r).to_string()).collect();
        return Err(format!(
            "several generated regions; splice supports one (opening markers at lines {})",
            lines.join(", ")
        ));
    }
    Ok(regions.into_iter().next().map(|region| Existing {
        opening_line: opening(&region),
        region,
    }))
}

/// The stamped region text for `request` and the warnings about its globs.
fn region_of(
    store: &dyn DocStore,
    request: &Request,
    existing: Option<&Existing>,
) -> Result<(String, Vec<String>), String> {
    let mermaid = fenced(request.mermaid)?;
    let command = command_of(request.command, existing)?;
    let (hash, warnings) = stamp_inputs(store, request.inputs)?;
    let opening = render_opening(&command, request.inputs, &hash);
    let region = format!("{opening}\n{mermaid}\n{}", render_closing());
    ensure_whole_region(&region, &command, request.inputs)?;
    Ok((region, warnings))
}

/// Requires `region` to parse as exactly one region with this command and
/// these globs, opening on its first line and closing on its last, so nothing
/// of it can land outside the markers.
fn ensure_whole_region(region: &str, command: &str, inputs: &[String]) -> Result<(), String> {
    let refusal = || {
        "the command, the globs or the Mermaid text cannot be written in a generated region"
            .to_string()
    };
    let regions = parse_regions(region).map_err(|_| refusal())?;
    let [parsed] = regions.as_slice() else {
        return Err(refusal());
    };
    let covers = parsed.span.start == 2 && parsed.span.end == region.lines().count();
    match covers && parsed.command == command && parsed.inputs == inputs {
        true => Ok(()),
        false => Err(refusal()),
    }
}

/// The marker command: the one given, else the existing region's.
fn command_of(given: Option<&str>, existing: Option<&Existing>) -> Result<String, String> {
    let command = match (given, existing) {
        (Some(command), _) => command.trim().to_string(),
        (None, Some(existing)) => existing.region.command.clone(),
        (None, None) => {
            return Err("--command is required when the view has no generated region".to_string())
        }
    };
    check_command(&command)?;
    Ok(command)
}

/// The hash of the inputs, refusing a glob the store refuses or one that
/// names only paths `.gitignore` excludes, and a warning for each glob that
/// matches nothing.
fn stamp_inputs(store: &dyn DocStore, globs: &[String]) -> Result<(String, Vec<String>), String> {
    if globs.is_empty() {
        return Err("--inputs needs at least one glob".to_string());
    }
    check_globs(globs)?;
    let need_repository = || "--inputs need a git repository".to_string();
    let hash = inputs_hash(store, globs).map_err(|error| match error {
        InputsError::Unavailable => need_repository(),
        other => other.to_string(),
    })?;
    let reach = store
        .repository_glob_reach(globs)
        .ok_or_else(need_repository)?
        .map_err(|error| error.to_string())?;
    let mut warnings = Vec::new();
    for (glob, reach) in globs.iter().zip(reach) {
        match reach {
            GlobReach::Matched => {}
            GlobReach::IgnoredOnly => {
                return Err(format!(
                    "glob `{glob}` matches only paths .gitignore excludes; name a file git can see"
                ))
            }
            GlobReach::Nothing => warnings.push(format!(
                "glob `{glob}` matches no file; the region is stamped over an empty set"
            )),
        }
    }
    Ok((hash, warnings))
}

/// `contents` with `region` in place of the existing region, or appended
/// after one blank line when the view holds none.
fn place(contents: &str, existing: Option<&Existing>, region: &str) -> String {
    let Some(existing) = existing else {
        return format!("{}\n\n{region}\n", contents.trim_end_matches('\n'));
    };
    let closing =
        existing.opening_line + (existing.region.span.end - existing.region.span.start) + 1;
    let at = byte_range(contents, existing.opening_line - 1, closing - 1);
    format!("{}{region}{}", &contents[..at.start], &contents[at.end..])
}

/// The bytes of the 0-based lines `first` to `last`, both included, of
/// `contents`, without the newline that ends the last of them.
fn byte_range(contents: &str, first: usize, last: usize) -> Range<usize> {
    let (mut start, mut end) = (0, 0);
    for (index, line) in contents.split_inclusive('\n').enumerate() {
        if index < first {
            start += line.len();
        }
        if index <= last {
            end += line.len();
        }
    }
    if contents[..end].ends_with('\n') {
        end -= 1;
    }
    start..end
}

mod input;
#[cfg(test)]
mod tests;
