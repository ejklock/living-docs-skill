//! The embedded skills, served as MCP skills and as `skill://` resources.
//!
//! The served set is exactly the files `install skills` places, read only from
//! the corpus embedded in `living-docs-core`. A URI is parsed against that
//! closed table, never turned into a path, so a `..` segment, an encoded
//! escape or a foreign scheme can only miss.

use living_docs_core::guide::{asset, SkillAssetSpec, SKILL_ASSET_SPECS};
use rmcp::model::{
    ReadResourceResult, Resource, ResourceContents, Skill, SkillFrontmatter, SkillResource,
    SkillResources,
};
use rmcp::ErrorData;
use std::borrow::Cow;

const SCHEME: &str = "skill://";
const ENTRY: &str = "SKILL.md";
const FRONTMATTER_FENCE: &str = "---\n";

fn file_uri(skill: &str, path: &str) -> String {
    format!("{SCHEME}{skill}/{path}")
}

/// Splits `uri` into a skill and a path that the install table names, or
/// `None` for anything else.
fn installable(uri: &str) -> Option<(&'static SkillAssetSpec, &'static str)> {
    let (skill, path) = uri.strip_prefix(SCHEME)?.split_once('/')?;
    let spec = SKILL_ASSET_SPECS.iter().find(|spec| spec.skill == skill)?;
    let path = spec.relative_paths.iter().find(|known| **known == path)?;
    Some((spec, path))
}

fn bytes(spec: &SkillAssetSpec, path: &str) -> Result<Cow<'static, [u8]>, ErrorData> {
    asset(&format!("{}/{path}", spec.skill)).ok_or_else(|| {
        ErrorData::internal_error(
            format!("embedded asset missing: {}/{path}", spec.skill),
            None,
        )
    })
}

fn frontmatter(spec: &SkillAssetSpec) -> Result<SkillFrontmatter, ErrorData> {
    let internal = |reason: String| ErrorData::internal_error(reason, None);
    let markdown = bytes(spec, ENTRY)?;
    let markdown = std::str::from_utf8(&markdown)
        .map_err(|_| internal(format!("{} {ENTRY} is not UTF-8", spec.skill)))?;
    let block = markdown
        .strip_prefix(FRONTMATTER_FENCE)
        .and_then(|rest| rest.split_once("\n---"))
        .map(|(block, _)| block)
        .ok_or_else(|| internal(format!("{} {ENTRY} has no frontmatter", spec.skill)))?;
    serde_yaml::from_str(block)
        .map_err(|error| internal(format!("{} frontmatter: {error}", spec.skill)))
}

fn entry(spec: &SkillAssetSpec) -> Result<Skill, ErrorData> {
    let files = spec
        .relative_paths
        .iter()
        .map(|path| {
            let content = bytes(spec, path)?;
            Ok(SkillResource::from_bytes(
                file_uri(spec.skill, path),
                &content,
            ))
        })
        .collect::<Result<Vec<_>, ErrorData>>()?;
    Ok(Skill::new(
        file_uri(spec.skill, ENTRY),
        frontmatter(spec)?,
        SkillResources::Files(files),
    ))
}

/// Every skill, ordered by name.
pub(crate) fn skills() -> Result<Vec<Skill>, ErrorData> {
    let mut skills = SKILL_ASSET_SPECS
        .iter()
        .map(entry)
        .collect::<Result<Vec<_>, _>>()?;
    skills.sort_by(|a, b| a.name().cmp(b.name()));
    Ok(skills)
}

/// The skill whose `SKILL.md` lives at `uri`; invalid params for any other URI.
pub(crate) fn skill(uri: &str) -> Result<Skill, ErrorData> {
    match installable(uri) {
        Some((spec, ENTRY)) => entry(spec),
        _ => Err(ErrorData::invalid_params(
            format!("no skill at {uri}"),
            None,
        )),
    }
}

/// Every installable file of every skill, one resource each.
pub(crate) fn resources() -> Vec<Resource> {
    let mut resources: Vec<Resource> = SKILL_ASSET_SPECS
        .iter()
        .flat_map(|spec| {
            spec.relative_paths.iter().map(move |path| {
                Resource::new(file_uri(spec.skill, path), format!("{}/{path}", spec.skill))
                    .with_mime_type(mime_type(path))
            })
        })
        .collect();
    resources.sort_by(|a, b| a.uri.cmp(&b.uri));
    resources
}

fn mime_type(path: &str) -> &'static str {
    if path.ends_with(".md") {
        "text/markdown"
    } else {
        "text/plain"
    }
}

/// The embedded content of the installable file at `uri`; a resource-not-found
/// error for any other URI.
pub(crate) fn read(uri: &str) -> Result<ReadResourceResult, ErrorData> {
    let Some((spec, path)) = installable(uri) else {
        return Err(ErrorData::resource_not_found(
            format!("no skill file at {uri}"),
            None,
        ));
    };
    let content = bytes(spec, path)?;
    let text = String::from_utf8(content.into_owned())
        .map_err(|_| ErrorData::internal_error(format!("{uri} is not text"), None))?;
    Ok(ReadResourceResult::new(vec![ResourceContents::text(
        text, uri,
    )
    .with_mime_type(mime_type(path))]))
}
