use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod common;
use common::Scratch;

const SKILL_RELATIVE_PATHS: [(&str, &str); 5] = [
    ("living-docs", "SKILL.md"),
    ("okf-knowledge-format", "SKILL.md"),
    ("okf-knowledge-format", "reference/SPEC.md"),
    ("okf-knowledge-format", "reference/SPEC.source.md"),
    ("research-artifacts", "SKILL.md"),
];

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn temp_dir(label: &str) -> Scratch {
    Scratch::new(&format!("living-docs-skill-install-test-{label}"))
}

fn unused_child_path(parent: &Path, label: &str) -> PathBuf {
    parent.join(label)
}

fn skills_corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skills")
}

fn run_skill_install(args: &[&str]) -> Output {
    living_docs()
        .arg("install")
        .arg("skills")
        .args(args)
        .output()
        .expect("failed to run living-docs install skills")
}

fn run_install_into(dest: &Path, extra_args: &[&str]) -> Output {
    let mut args = vec!["--dir", dest.to_str().unwrap()];
    args.extend_from_slice(extra_args);
    run_skill_install(&args)
}

fn read_all_placed(dest: &Path) -> Vec<Vec<u8>> {
    SKILL_RELATIVE_PATHS
        .iter()
        .map(|(skill, relative)| {
            fs::read(dest.join(skill).join(relative))
                .unwrap_or_else(|e| panic!("failed to read installed {skill}/{relative}: {e}"))
        })
        .collect()
}

#[test]
fn install_materializes_every_skill_file_byte_identical_to_the_embedded_corpus() {
    let parent = temp_dir("materialize");
    let dest = unused_child_path(&parent, "dest");

    let output = run_install_into(&dest, &[]);

    assert!(
        output.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for (skill, relative) in SKILL_RELATIVE_PATHS {
        let installed = dest.join(skill).join(relative);
        let expected = fs::read(skills_corpus_root().join(skill).join(relative))
            .unwrap_or_else(|e| panic!("failed to read corpus {skill}/{relative}: {e}"));
        let actual = fs::read(&installed)
            .unwrap_or_else(|e| panic!("failed to read installed {skill}/{relative}: {e}"));
        assert_eq!(
            actual, expected,
            "{skill}/{relative} bytes differ from the corpus"
        );
    }

    let _ = fs::remove_dir_all(&parent);
}

#[test]
fn install_places_the_okf_knowledge_format_reference_subdirectory() {
    let parent = temp_dir("reference-subdir");
    let dest = unused_child_path(&parent, "dest");

    let output = run_install_into(&dest, &[]);

    assert!(output.status.success());
    assert!(
        dest.join("okf-knowledge-format/reference/SPEC.md")
            .is_file(),
        "reference/SPEC.md was not placed"
    );
    assert!(
        dest.join("okf-knowledge-format/reference/SPEC.source.md")
            .is_file(),
        "reference/SPEC.source.md was not placed"
    );

    let _ = fs::remove_dir_all(&parent);
}

#[test]
fn install_is_idempotent_across_repeated_runs() {
    let parent = temp_dir("idempotent");
    let dest = unused_child_path(&parent, "dest");

    let first = run_install_into(&dest, &[]);
    assert!(first.status.success());
    let first_bytes = read_all_placed(&dest);

    let second = run_install_into(&dest, &[]);
    assert!(second.status.success());
    let second_bytes = read_all_placed(&dest);

    assert_eq!(
        first_bytes, second_bytes,
        "repeated install must leave byte-identical files"
    );

    let _ = fs::remove_dir_all(&parent);
}

#[test]
fn dry_run_reports_the_plan_and_writes_nothing() {
    let parent = temp_dir("dry-run");
    let dest = unused_child_path(&parent, "dest");

    let output = run_install_into(&dest, &["--dry-run"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    for (skill, relative) in SKILL_RELATIVE_PATHS {
        assert!(
            stdout.contains(&format!("{skill}/{relative}"))
                || stdout.contains(&dest.join(skill).join(relative).display().to_string()),
            "stdout missing {skill}/{relative}: {stdout}"
        );
    }
    assert!(!dest.exists(), "dry-run must not create the destination");

    let _ = fs::remove_dir_all(&parent);
}

#[test]
fn install_defaults_the_harness_to_claude_when_omitted() {
    let parent = temp_dir("default-harness");
    let dest = unused_child_path(&parent, "dest");

    let output = run_install_into(&dest, &[]);

    assert!(output.status.success());
    assert!(dest.join("living-docs/SKILL.md").is_file());

    let _ = fs::remove_dir_all(&parent);
}

#[test]
fn unknown_harness_value_is_a_clean_clap_error_not_a_panic() {
    let parent = temp_dir("unknown-harness");
    let dest = unused_child_path(&parent, "dest");

    let output = run_install_into(&dest, &["--harness", "no-such-harness"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(!stderr.is_empty(), "expected a stderr message");
    assert!(
        !stderr.contains("panicked"),
        "must be a clean clap error, not a panic: {stderr}"
    );
    assert!(!dest.exists());

    let _ = fs::remove_dir_all(&parent);
}

#[test]
fn install_scopes_to_the_project_relative_directory_when_project_is_given_without_dir() {
    let project = temp_dir("project-scope");

    let output = living_docs()
        .args(["install", "skills", "--harness", "codex", "--project"])
        .current_dir(&project)
        .output()
        .expect("failed to run living-docs install skills");

    assert!(
        output.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(project.join(".codex/skills/living-docs/SKILL.md").is_file());
    assert!(project
        .join(".codex/skills/okf-knowledge-format/reference/SPEC.md")
        .is_file());

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn dir_override_ignores_harness_producing_identical_output_for_any_harness_value() {
    let parent = temp_dir("dir-ignores-harness");
    let claude_dest = unused_child_path(&parent, "claude-dest");
    let codex_dest = unused_child_path(&parent, "codex-dest");

    let claude_output = run_install_into(&claude_dest, &["--harness", "claude"]);
    let codex_output = run_install_into(&codex_dest, &["--harness", "codex"]);

    assert!(claude_output.status.success());
    assert!(codex_output.status.success());
    assert_eq!(
        read_all_placed(&claude_dest),
        read_all_placed(&codex_dest),
        "--dir must override the destination regardless of --harness"
    );

    let _ = fs::remove_dir_all(&parent);
}

#[test]
fn install_uses_an_overridden_home_for_the_global_destination_when_neither_project_nor_dir_is_given(
) {
    let fake_home = temp_dir("fake-home");

    let output = living_docs()
        .args(["install", "skills", "--harness", "pi"])
        .env("HOME", &fake_home)
        .output()
        .expect("failed to run living-docs install skills");

    assert!(
        output.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fake_home
        .join(".pi/agent/skills/living-docs/SKILL.md")
        .is_file());
    assert!(fake_home
        .join(".pi/agent/skills/research-artifacts/SKILL.md")
        .is_file());

    let _ = fs::remove_dir_all(&fake_home);
}
