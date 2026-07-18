use rust_embed::RustEmbed;
use serde::Serialize;
use std::collections::BTreeSet;

/// The `skills/**` corpus embedded in the binary at compile time (ADR
/// 0014). `folder` is resolved relative to this crate's
/// `CARGO_MANIFEST_DIR`, so `../skills/` reaches the repo-root tree.
#[derive(RustEmbed)]
#[folder = "../skills/"]
struct SkillAssets;

const RULES_DIR: &str = "rules";
const TEMPLATES_DIR: &str = "templates";
const CONVENTIONS_SUFFIX: &str = "-conventions";

/// Lists every embedded skill and, per skill, its sorted, deduped set of
/// topic keys derived from `rules/` and `templates/` basenames. Err only
/// when the binary was built with no embedded skills at all.
pub(crate) fn list() -> Result<String, String> {
    let names = skill_names();
    if names.is_empty() {
        return Err("no skills are embedded in this binary".to_owned());
    }
    Ok(names.into_iter().map(render_skill_line).collect())
}

fn render_skill_line(name: String) -> String {
    let topics = topic_keys_for_skill(&name);
    if topics.is_empty() {
        return format!("{name}:\n");
    }
    let joined = topics.into_iter().collect::<Vec<_>>().join(", ");
    format!("{name}: {joined}\n")
}

/// Minified-JSON counterpart of [`list`]: the same sorted, deduped skills
/// and topics, serialized as `{"skills":[{"name","topics":[...]},...]}`.
/// Err under the same condition as `list` (no embedded skills).
pub(crate) fn list_json() -> Result<String, String> {
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
pub(crate) fn body(name: &str) -> Result<String, String> {
    read_asset(&format!("{name}/SKILL.md")).map_err(|_| format!("unknown skill: {name}"))
}

/// Minified-JSON counterpart of [`body`]: `{"skill","content"}`. Err under
/// the same condition as `body` (unknown skill).
pub(crate) fn body_json(name: &str) -> Result<String, String> {
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
pub(crate) fn topic(name: &str, topic: &str) -> Result<String, String> {
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
pub(crate) fn topic_json(name: &str, topic: &str) -> Result<String, String> {
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

fn skill_names() -> BTreeSet<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_includes_every_embedded_skill_and_the_adr_topic() {
        let output = list().expect("list succeeds with an embedded corpus");
        assert!(output.contains("living-docs"));
        assert!(output.contains("okf-knowledge-format"));
        assert!(output.contains("research-artifacts"));
        assert!(output.contains("adr"));
    }

    #[test]
    fn body_returns_the_skill_md_contents() {
        let output = body("living-docs").expect("living-docs/SKILL.md is embedded");
        assert!(output.contains("# Living Docs"));
    }

    #[test]
    fn body_errors_for_an_unknown_skill() {
        assert!(body("no-such-skill").is_err());
    }

    #[test]
    fn topic_adr_concatenates_the_rule_and_template() {
        let output = topic("living-docs", "adr").expect("adr topic resolves");
        assert!(output.contains("rules/adr-conventions.md"));
        assert!(output.contains("templates/adr.md"));
    }

    #[test]
    fn topic_issue_workflow_resolves_only_the_rules_file() {
        let output = topic("living-docs", "issue-workflow").expect("issue-workflow topic resolves");
        assert!(output.contains("rules/issue-workflow.md"));
        assert!(!output.contains("templates/issue-workflow.md"));
    }

    #[test]
    fn topic_errors_for_an_unknown_topic() {
        assert!(topic("living-docs", "no-such-topic").is_err());
    }

    #[test]
    fn topic_errors_for_an_unknown_skill() {
        assert!(topic("no-such-skill", "adr").is_err());
    }

    #[test]
    fn list_json_matches_list_names_and_topics() {
        let plain = list().expect("list succeeds with an embedded corpus");
        let output = list_json().expect("list_json succeeds with an embedded corpus");
        let parsed: SkillListJson =
            serde_json::from_str(&output).expect("list_json emits valid JSON");
        let living_docs = parsed
            .skills
            .iter()
            .find(|skill| skill.name == "living-docs")
            .expect("living-docs is present in list_json");
        assert!(living_docs.topics.contains(&"adr".to_owned()));
        assert!(plain.contains("living-docs"));
        assert!(!output.contains('\n'));
    }

    #[test]
    fn body_json_matches_body_content() {
        let plain = body("living-docs").expect("living-docs/SKILL.md is embedded");
        let output = body_json("living-docs").expect("body_json succeeds for a known skill");
        let parsed: SkillBodyJson =
            serde_json::from_str(&output).expect("body_json emits valid JSON");
        assert_eq!(parsed.skill, "living-docs");
        assert_eq!(parsed.content, plain);
        assert!(!output.contains('\n'));
    }

    #[test]
    fn body_json_errors_for_an_unknown_skill() {
        assert!(body_json("no-such-skill").is_err());
    }

    #[test]
    fn topic_json_adr_matches_the_rule_and_template_ordering() {
        let output = topic_json("living-docs", "adr").expect("adr topic_json resolves");
        let parsed: SkillTopicJson =
            serde_json::from_str(&output).expect("topic_json emits valid JSON");
        assert_eq!(parsed.skill, "living-docs");
        assert_eq!(parsed.topic, "adr");
        let paths: Vec<&str> = parsed.parts.iter().map(|part| part.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "living-docs/rules/adr-conventions.md",
                "living-docs/templates/adr.md"
            ]
        );
        assert!(!output.contains('\n'));
    }

    #[test]
    fn topic_json_errors_for_an_unknown_topic() {
        assert!(topic_json("living-docs", "no-such-topic").is_err());
    }
}
