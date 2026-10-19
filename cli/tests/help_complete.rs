//! Completeness of the help: every argument described, vocabularies drawn
//! from the registries, and no help string aimed at a maintainer.

mod common;

use common::living_docs;
use living_docs_core::check::kinds;
use living_docs_core::doc_type::DOC_TYPES;
use serde_json::Value;

fn stdout_of(args: &[&str]) -> String {
    let output = living_docs()
        .args(args)
        .output()
        .expect("failed to run living-docs");
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn plain_help(verb: &[&str]) -> String {
    let mut args = verb.to_vec();
    args.extend(["--help", "--plain"]);
    stdout_of(&args)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn root_doc() -> Value {
    serde_json::from_str(&stdout_of(&["--help"])).expect("root help is JSON")
}

fn lacks_text(value: &Value, key: &str) -> bool {
    value[key]
        .as_str()
        .is_none_or(|text| text.trim().is_empty())
}

fn collect_missing_help(command: &Value, trail: &str, missing: &mut Vec<String>) {
    let here = format!("{trail} {}", command["name"].as_str().unwrap_or_default());
    for key in ["args", "options"] {
        for item in command[key].as_array().into_iter().flatten() {
            if lacks_text(item, "help") {
                let label = item["name"].as_str().or(item["long"].as_str());
                missing.push(format!("{here} {}", label.unwrap_or("?")));
            }
        }
    }
    if lacks_text(command, "about") {
        missing.push(format!("{here} about"));
    }
    for sub in command["subcommands"].as_array().into_iter().flatten() {
        collect_missing_help(sub, &here, missing);
    }
}

#[test]
fn every_command_argument_and_option_has_help_text() {
    let doc = root_doc();
    let mut missing = Vec::new();
    for option in doc["options"].as_array().expect("root options") {
        if lacks_text(option, "help") {
            missing.push(format!("root --{}", option["long"]));
        }
    }
    for command in doc["commands"].as_array().expect("commands") {
        collect_missing_help(command, "", &mut missing);
    }

    assert!(missing.is_empty(), "undocumented: {missing:?}");
}

#[test]
fn no_help_string_cites_a_record_number_or_a_test_concern() {
    let doc = root_doc();
    let mut surfaces = vec![stdout_of(&["--help"]), plain_help(&[])];
    for command in doc["commands"].as_array().expect("commands") {
        let verb = command["name"].as_str().unwrap_or_default();
        surfaces.push(plain_help(&[verb]));
        for sub in command["subcommands"].as_array().into_iter().flatten() {
            surfaces.push(plain_help(&[
                verb,
                sub["name"].as_str().unwrap_or_default(),
            ]));
        }
    }

    for surface in surfaces {
        for banned in ["ADR ", "issue 0", "tests can", "temp tree"] {
            assert!(!surface.contains(banned), "'{banned}' in:\n{surface}");
        }
    }
}

#[test]
fn set_help_names_the_keys_and_every_registered_status_vocabulary() {
    let help = plain_help(&["set"]);

    for key in ["status", "description", "owner", "title", "section"] {
        assert!(help.contains(key), "missing key {key}:\n{help}");
    }
    for spec in DOC_TYPES
        .iter()
        .filter(|spec| !spec.status_vocabulary.is_empty())
    {
        let expected = format!(
            "{}: {}",
            spec.frontmatter,
            spec.status_vocabulary.join(", ")
        );
        assert!(help.contains(&expected), "missing '{expected}':\n{help}");
    }
    assert!(
        help.contains("Superseded is set only by `supersede`"),
        "{help}"
    );
}

#[test]
fn set_and_supersede_help_state_the_three_reference_forms() {
    for verb in ["set", "supersede"] {
        let help = plain_help(&[verb]);
        assert!(help.contains("NNNN"), "{verb}:\n{help}");
        assert!(help.contains("TYPE/NNNN"), "{verb}:\n{help}");
        assert!(help.contains("directory name"), "{verb}:\n{help}");
        assert!(help.contains(".md"), "{verb}:\n{help}");
    }
}

#[test]
fn new_help_says_owner_is_seeded_from_git_config_on_types_that_require_one() {
    let help = plain_help(&["new"]);

    assert!(help.contains("git config user.name"), "{help}");
    assert!(help.contains("user.email"), "{help}");
    for spec in DOC_TYPES.iter().filter(|spec| spec.requires_owner) {
        assert!(
            help.contains(spec.token),
            "{} not named:\n{help}",
            spec.token
        );
    }
}

#[test]
fn guide_help_names_every_topic_that_guide_list_prints() {
    let listing = stdout_of(&["guide", "--list", "--plain"]);
    let help = plain_help(&["guide"]);

    let mut compared = 0;
    for line in listing.lines() {
        let (skill, topics) = line.split_once(':').expect("skill: topics");
        assert!(
            help.contains(skill.trim()),
            "skill {skill} missing:\n{help}"
        );
        let topics = topics
            .split(',')
            .map(str::trim)
            .filter(|topic| !topic.is_empty());
        for topic in topics {
            assert!(help.contains(topic), "topic {topic} missing:\n{help}");
            compared += 1;
        }
    }
    assert!(compared > 10, "only {compared} topics compared");
}

#[test]
fn check_help_lists_every_advisory_kind_and_the_exit_code_split() {
    let help = plain_help(&["check"]);

    assert!(!kinds::ALL.is_empty());
    for kind in kinds::ALL {
        let expected = format!("{} {}", kind.name, kind.meaning);
        assert!(help.contains(&expected), "missing '{expected}':\n{help}");
    }
    assert!(help.contains("Violations exit 1"), "{help}");
    assert!(
        help.contains("advisories never change the exit code"),
        "{help}"
    );
}

#[test]
fn check_json_help_carries_the_kinds_too() {
    let text = stdout_of(&["check", "--help"]);

    for kind in kinds::ALL {
        assert!(
            text.contains(kind.name),
            "{} missing from {text}",
            kind.name
        );
    }
}
