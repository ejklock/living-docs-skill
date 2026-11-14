//! The authoring guide: the `skills/**` corpus embedded at compile time and
//! the plain-text and minified-JSON renderers both fronts serve from it.

use rust_embed::RustEmbed;
use serde::Serialize;
use std::borrow::Cow;
use std::collections::BTreeSet;

#[cfg(test)]
mod tests;

/// The skill a guide request names when it names none.
pub const DEFAULT_SKILL: &str = "living-docs";

/// The `skills/**` corpus embedded in the binary at compile time (ADR
/// 0014). `folder` is resolved relative to this crate's
/// `CARGO_MANIFEST_DIR`, so `../skills/` reaches the repo-root tree.
#[derive(RustEmbed)]
#[folder = "../skills/"]
struct SkillAssets;

const RULES_DIR: &str = "rules";
const TEMPLATES_DIR: &str = "templates";
const CONVENTIONS_SUFFIX: &str = "-conventions";

/// Lists every embedded skill with its sorted topic keys from `rules/` and
/// `templates/`; a topicless skill names the command printing its SKILL.md.
/// Err only when no skills are embedded.
pub fn list() -> Result<String, String> {
    let names = skill_names();
    if names.is_empty() {
        return Err("no skills are embedded in this binary".to_owned());
    }
    Ok(names.into_iter().map(render_skill_line).collect())
}

fn render_skill_line(name: String) -> String {
    let topics = topic_keys_for_skill(&name);
    if topics.is_empty() {
        return format!("{name}: no topics; living-docs read guide {name} prints its SKILL.md\n");
    }
    let joined = topics.into_iter().collect::<Vec<_>>().join(", ");
    format!("{name}: {joined}\n")
}

/// Minified-JSON counterpart of [`list`]: the same sorted, deduped skills
/// and topics, serialized as `{"skills":[{"name","topics":[...]},...]}`.
/// Err under the same condition as `list` (no embedded skills).
pub fn list_json() -> Result<String, String> {
    let names = skill_names();
    if names.is_empty() {
        return Err("no skills are embedded in this binary".to_owned());
    }
    let skills = names.into_iter().map(skill_summary).collect();
    to_minified_json(&SkillListJson { skills })
}

fn skill_summary(name: String) -> SkillSummary {
    let topics = topic_keys_for_skill(&name).into_iter().collect();
    SkillSummary { name, topics }
}

#[derive(Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
struct SkillListJson {
    skills: Vec<SkillSummary>,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
struct SkillSummary {
    name: String,
    topics: Vec<String>,
}

/// Returns the embedded `<name>/SKILL.md` body. Err when `name` has no
/// embedded `SKILL.md` asset.
pub fn body(name: &str) -> Result<String, String> {
    read_asset(&format!("{name}/SKILL.md")).map_err(|_| format!("unknown skill: {name}"))
}

/// Minified-JSON counterpart of [`body`]: `{"skill","content"}`. Err under
/// the same condition as `body` (unknown skill).
pub fn body_json(name: &str) -> Result<String, String> {
    let content = body(name)?;
    to_minified_json(&SkillBodyJson {
        skill: name.to_owned(),
        content,
    })
}

#[derive(Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
struct SkillBodyJson {
    skill: String,
    content: String,
}

/// Returns the resolved `rules/`/`templates/` file(s) for `topic` under
/// `name`, each prefixed with a `## <relative-path>` header line. Err when
/// the resolution set is empty (unknown skill or unknown topic).
pub fn topic(name: &str, topic: &str) -> Result<String, String> {
    let paths = resolve_topic_paths(name, topic);
    if paths.is_empty() {
        return Err(format!("no topic '{topic}' found for skill '{name}'"));
    }
    paths
        .into_iter()
        .map(render_topic_section)
        .collect::<Result<String, String>>()
}

fn render_topic_section(path: String) -> Result<String, String> {
    let contents = read_asset(&path)?;
    Ok(format!("## {path}\n{contents}\n"))
}

/// Minified-JSON counterpart of [`topic`]: `{"skill","topic","parts":[{"path","content"},...]}`,
/// preserving the same resolution set and ordering (rules before templates). Err under the
/// same condition as `topic` (empty resolution set).
pub fn topic_json(name: &str, topic: &str) -> Result<String, String> {
    let paths = resolve_topic_paths(name, topic);
    if paths.is_empty() {
        return Err(format!("no topic '{topic}' found for skill '{name}'"));
    }
    let parts = paths
        .into_iter()
        .map(topic_part)
        .collect::<Result<Vec<_>, String>>()?;
    to_minified_json(&SkillTopicJson {
        skill: name.to_owned(),
        topic: topic.to_owned(),
        parts,
    })
}

fn topic_part(path: String) -> Result<TopicPart, String> {
    let content = read_asset(&path)?;
    Ok(TopicPart { path, content })
}

#[derive(Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
struct SkillTopicJson {
    skill: String,
    topic: String,
    parts: Vec<TopicPart>,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
struct TopicPart {
    path: String,
    content: String,
}

fn to_minified_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|err| format!("failed to serialize JSON: {err}"))
}

fn read_asset(path: &str) -> Result<String, String> {
    let file = SkillAssets::get(path).ok_or_else(|| format!("missing embedded asset: {path}"))?;
    String::from_utf8(file.data.into_owned()).map_err(|_| format!("{path} is not valid UTF-8"))
}

/// Raw bytes of a `skills/**`-relative embedded asset (e.g.
/// `living-docs/hooks/session-context.sh`), for consumers that need the
/// asset's bytes as-is rather than [`read_asset`]'s UTF-8-decoded form. None
/// when `path` has no embedded asset.
pub fn asset(path: &str) -> Option<Cow<'static, [u8]>> {
    SkillAssets::get(path).map(|file| file.data)
}

/// The names of every embedded skill, sorted.
pub fn skill_names() -> BTreeSet<String> {
    SkillAssets::iter()
        .filter_map(|path| path.split('/').next().map(str::to_owned))
        .collect()
}

fn topic_keys_for_skill(name: &str) -> BTreeSet<String> {
    let rules_prefix = format!("{name}/{RULES_DIR}/");
    let templates_prefix = format!("{name}/{TEMPLATES_DIR}/");
    SkillAssets::iter()
        .filter_map(|path| topic_key_from_asset(&path, &rules_prefix, &templates_prefix))
        .collect()
}

fn topic_key_from_asset(path: &str, rules_prefix: &str, templates_prefix: &str) -> Option<String> {
    if let Some(basename) = path.strip_prefix(rules_prefix) {
        return Some(normalize_topic_key(strip_md_extension(basename)));
    }
    path.strip_prefix(templates_prefix)
        .map(|basename| strip_md_extension(basename).to_owned())
}

/// Strips a single trailing `-conventions` from a rules-file basename, so
/// `adr-conventions` and `adr.md`'s own basename `adr` resolve to the same
/// topic key `adr`.
fn normalize_topic_key(basename: &str) -> String {
    basename
        .strip_suffix(CONVENTIONS_SUFFIX)
        .unwrap_or(basename)
        .to_owned()
}

fn strip_md_extension(basename: &str) -> &str {
    basename.strip_suffix(".md").unwrap_or(basename)
}

fn resolve_topic_paths(name: &str, topic: &str) -> Vec<String> {
    let mut paths = matching_rules_paths(name, topic);
    let template_path = format!("{name}/{TEMPLATES_DIR}/{topic}.md");
    if SkillAssets::get(&template_path).is_some() {
        paths.push(template_path);
    }
    paths
}

fn matching_rules_paths(name: &str, topic: &str) -> Vec<String> {
    let rules_prefix = format!("{name}/{RULES_DIR}/");
    let mut paths: Vec<String> = SkillAssets::iter()
        .filter(|path| rules_path_matches_topic(path, &rules_prefix, topic))
        .map(|path| path.into_owned())
        .collect();
    paths.sort();
    paths
}

fn rules_path_matches_topic(path: &str, rules_prefix: &str, topic: &str) -> bool {
    path.strip_prefix(rules_prefix)
        .map(|basename| normalize_topic_key(strip_md_extension(basename)) == topic)
        .unwrap_or(false)
}
