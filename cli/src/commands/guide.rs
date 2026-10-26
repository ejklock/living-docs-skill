//! `guide` verb wrapper: resolves the
//! guide's target skill/topic, then delegates to the embedded skill corpus
//! through the shared `crate::output::OutputMode`.

use crate::args::GuideArgs;
use crate::output::OutputMode;
use crate::skill;
use crate::store::report_failure;
use std::process::ExitCode;

const DEFAULT_SKILL: &str = "living-docs";

/// The skill and, optionally, the topic `guide` resolves to run against.
struct Target {
    skill: String,
    topic: Option<String>,
}

/// Resolves [`GuideArgs`] to a [`Target`] (see the struct's own docblock for
/// the shapes this reconciles):
/// 1. `--skill` given explicitly names the skill; the positional, if any, is
///    the topic.
/// 2. Otherwise, a positional that names a known embedded skill is the
///    skill, with no topic — prints that skill's full `SKILL.md` body.
/// 3. Otherwise, the default skill (`living-docs`); the positional, if any,
///    is the topic.
fn resolve_target(args: &GuideArgs) -> Target {
    if let Some(skill) = &args.skill {
        return Target {
            skill: skill.clone(),
            topic: args.topic_or_skill.clone(),
        };
    }
    if let Some(candidate) = &args.topic_or_skill {
        if skill::skill_names().contains(candidate) {
            return Target {
                skill: candidate.clone(),
                topic: None,
            };
        }
    }
    Target {
        skill: DEFAULT_SKILL.to_owned(),
        topic: args.topic_or_skill.clone(),
    }
}

pub(crate) fn run_guide(args: GuideArgs, mode: OutputMode) -> ExitCode {
    let as_json = mode.is_json();
    if args.list {
        return print_guide_result(if as_json {
            skill::list_json()
        } else {
            skill::list()
        });
    }
    let target = resolve_target(&args);
    match target.topic {
        Some(topic) => print_guide_result(if as_json {
            skill::topic_json(&target.skill, &topic)
        } else {
            skill::topic(&target.skill, &topic)
        }),
        None => print_guide_result(if as_json {
            skill::body_json(&target.skill)
        } else {
            skill::body(&target.skill)
        }),
    }
}

fn print_guide_result(result: Result<String, String>) -> ExitCode {
    match result {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(topic_or_skill: Option<&str>, skill: Option<&str>) -> GuideArgs {
        GuideArgs {
            topic_or_skill: topic_or_skill.map(str::to_owned),
            skill: skill.map(str::to_owned),
            list: false,
        }
    }

    #[test]
    fn bare_positional_topic_resolves_under_the_default_skill() {
        let target = resolve_target(&args(Some("adr"), None));
        assert_eq!(target.skill, "living-docs");
        assert_eq!(target.topic.as_deref(), Some("adr"));
    }

    #[test]
    fn positional_matching_a_known_skill_selects_it_with_no_topic() {
        let target = resolve_target(&args(Some("okf-knowledge-format"), None));
        assert_eq!(target.skill, "okf-knowledge-format");
        assert_eq!(target.topic, None);
    }

    #[test]
    fn no_positional_resolves_to_the_default_skill_with_no_topic() {
        let target = resolve_target(&args(None, None));
        assert_eq!(target.skill, "living-docs");
        assert_eq!(target.topic, None);
    }

    #[test]
    fn explicit_skill_flag_treats_the_positional_as_a_topic() {
        let target = resolve_target(&args(Some("conformance"), Some("okf-knowledge-format")));
        assert_eq!(target.skill, "okf-knowledge-format");
        assert_eq!(target.topic.as_deref(), Some("conformance"));
    }
}
