use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const SCRIPT_BASENAMES: [&str; 1] = ["session-context.sh"];
const LIVING_DOCS_MARKER: &str = ".living-docs/hooks/";

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("living-docs-hooks-install-test-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn project_with_bundle(label: &str, bundle: &str) -> PathBuf {
    let project = temp_dir(label);
    fs::create_dir_all(project.join(bundle)).unwrap();
    project
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skills/living-docs/hooks")
}

fn run_hooks_install(args: &[&str]) -> Output {
    living_docs()
        .arg("install")
        .arg("hooks")
        .args(args)
        .output()
        .expect("failed to run living-docs install hooks")
}

fn run_hooks_uninstall(args: &[&str]) -> Output {
    living_docs()
        .arg("uninstall")
        .arg("hooks")
        .args(args)
        .output()
        .expect("failed to run living-docs uninstall hooks")
}

fn run_uninstall(dir: &Path, dry_run: bool) -> Output {
    let dir_str = dir.to_str().unwrap();
    if dry_run {
        run_hooks_uninstall(&["--dir", dir_str, "--dry-run"])
    } else {
        run_hooks_uninstall(&["--dir", dir_str])
    }
}

fn init_git_repo(dir: &Path) {
    let output = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .output()
        .expect("failed to run git init");
    assert!(output.status.success(), "git init failed: {output:?}");
}

fn read_git_config(dir: &Path, key: &str) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["config", "--get", key])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn run_install(dir: &Path, dry_run: bool) -> Output {
    let dir_str = dir.to_str().unwrap();
    if dry_run {
        run_hooks_install(&["--dir", dir_str, "--dry-run"])
    } else {
        run_hooks_install(&["--dir", dir_str])
    }
}

fn read_settings(project: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(project.join(".claude/settings.json"))
        .expect("settings.json was written");
    serde_json::from_str(&raw).expect("settings.json parses as JSON")
}

/// The entries under `hooks.<section>`; an absent section is an empty list,
/// since `install` writes only the sections it wires.
fn entries(settings: &serde_json::Value, section: &str) -> Vec<serde_json::Value> {
    settings["hooks"][section]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn living_docs_entries(settings: &serde_json::Value, section: &str) -> Vec<serde_json::Value> {
    entries(settings, section)
        .into_iter()
        .filter(|entry| {
            entry["hooks"].as_array().into_iter().flatten().any(|hook| {
                hook["command"]
                    .as_str()
                    .is_some_and(|command| command.contains(LIVING_DOCS_MARKER))
            })
        })
        .collect()
}

#[test]
fn install_materializes_the_hook_script_byte_identical_at_mode_0755() {
    let project = project_with_bundle("materialize", "docs");

    let output = run_install(&project, false);

    assert!(output.status.success());
    let hooks_dir = project.join(".living-docs/hooks");
    for basename in SCRIPT_BASENAMES {
        let installed = hooks_dir.join(basename);
        let expected = fs::read(corpus_root().join(basename))
            .unwrap_or_else(|e| panic!("failed to read corpus {basename}: {e}"));
        let actual = fs::read(&installed)
            .unwrap_or_else(|e| panic!("failed to read installed {basename}: {e}"));
        assert_eq!(actual, expected, "{basename} bytes differ from the corpus");

        let mode = fs::metadata(&installed).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755, "{basename} unexpected mode: {mode:o}");
    }

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_is_idempotent_across_repeated_runs() {
    let project = project_with_bundle("idempotent", "docs");
    let hooks_dir = project.join(".living-docs/hooks");

    let first = run_install(&project, false);
    assert!(first.status.success());
    let first_bytes: Vec<Vec<u8>> = SCRIPT_BASENAMES
        .iter()
        .map(|basename| fs::read(hooks_dir.join(basename)).unwrap())
        .collect();

    let second = run_install(&project, false);
    assert!(second.status.success());
    let second_bytes: Vec<Vec<u8>> = SCRIPT_BASENAMES
        .iter()
        .map(|basename| fs::read(hooks_dir.join(basename)).unwrap())
        .collect();

    assert_eq!(
        first_bytes, second_bytes,
        "repeated install must leave byte-identical files"
    );

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn dry_run_reports_the_plan_and_writes_nothing() {
    let project = project_with_bundle("dry-run", "docs");

    let output = run_install(&project, true);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    for basename in SCRIPT_BASENAMES {
        assert!(
            stdout.contains(basename),
            "stdout missing {basename}: {stdout}"
        );
    }
    assert!(stdout.contains(".claude/settings.json"), "got: {stdout}");
    assert!(!project.join(".living-docs").exists());
    assert!(!project.join(".claude").exists());

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_defaults_dir_to_the_current_directory_when_omitted() {
    let project = project_with_bundle("default-dir", "docs");

    let output = living_docs()
        .args(["install", "hooks", "--dry-run"])
        .current_dir(&project)
        .output()
        .expect("failed to run living-docs install hooks");

    assert!(output.status.success());
    assert!(!project.join(".living-docs").exists());

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_wires_a_fresh_settings_json_with_one_sessionstart_entry_and_no_write_gate() {
    let project = project_with_bundle("fresh-wire", "docs");

    let output = run_install(&project, false);
    assert!(output.status.success());

    let settings = read_settings(&project);
    assert!(
        entries(&settings, "PreToolUse").is_empty(),
        "got: {settings:?}"
    );

    let session_start = entries(&settings, "SessionStart");
    assert_eq!(session_start.len(), 1, "got: {session_start:?}");
    let session_command = session_start[0]["hooks"][0]["command"]
        .as_str()
        .expect("SessionStart entry carries a command string");
    assert!(session_command.contains(".living-docs/hooks/session-context.sh"));
    assert!(session_command.starts_with("LIVING_DOCS_BUNDLE=docs "));

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_preserves_unrelated_settings_entries_and_top_level_keys() {
    let project = project_with_bundle("preserve", "docs");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(
        project.join(".claude/settings.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "unrelatedTopLevelKey": "keep-me",
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [ { "type": "command", "command": "echo custom-guard" } ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let output = run_install(&project, false);
    assert!(output.status.success());

    let settings = read_settings(&project);
    assert_eq!(settings["unrelatedTopLevelKey"], "keep-me");

    let pre_tool_use = entries(&settings, "PreToolUse");
    assert_eq!(pre_tool_use.len(), 1, "got: {pre_tool_use:?}");
    let unrelated_still_present = pre_tool_use.iter().any(|entry| {
        entry["hooks"][0]["command"] == "echo custom-guard" && entry["matcher"] == "Bash"
    });
    assert!(unrelated_still_present, "got: {pre_tool_use:?}");

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_replaces_living_docs_entries_on_reinstall_without_duplicating() {
    let project = project_with_bundle("reinstall", "docs");

    let first = run_install(&project, false);
    assert!(first.status.success());
    let second = run_install(&project, false);
    assert!(second.status.success());

    let settings = read_settings(&project);
    assert!(living_docs_entries(&settings, "PreToolUse").is_empty());
    assert_eq!(
        living_docs_entries(&settings, "SessionStart").len(),
        1,
        "got: {settings:?}"
    );

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_pins_a_custom_docs_dir_verbatim_into_the_generated_commands() {
    let project = project_with_bundle("custom-bundle", "handbook");

    let output = run_hooks_install(&["--docs-dir", "handbook", "--dir", project.to_str().unwrap()]);
    assert!(output.status.success());

    let settings = read_settings(&project);
    let session_command = entries(&settings, "SessionStart")[0]["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        session_command.starts_with("LIVING_DOCS_BUNDLE=handbook "),
        "got: {session_command}"
    );

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_fails_with_exit_2_when_docs_dir_does_not_exist_and_writes_nothing() {
    let project = temp_dir("missing-bundle");

    let output = run_hooks_install(&["--docs-dir", "nope", "--dir", project.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--docs-dir"), "got: {stderr}");
    assert!(!project.join(".living-docs").exists());
    assert!(!project.join(".claude").exists());

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_fails_with_exit_2_when_existing_settings_json_is_not_valid_json_and_never_overwrites_it()
{
    let project = project_with_bundle("invalid-json", "docs");
    fs::create_dir_all(project.join(".claude")).unwrap();
    let settings_path = project.join(".claude/settings.json");
    fs::write(&settings_path, "{ not valid json").unwrap();

    let output = run_install(&project, false);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("settings.json"), "got: {stderr}");
    let raw = fs::read_to_string(&settings_path).unwrap();
    assert_eq!(
        raw, "{ not valid json",
        "settings.json must never be overwritten on parse failure"
    );
    assert!(!project
        .join(".living-docs/hooks/session-context.sh")
        .exists());

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_materializes_pre_commit_byte_identical_at_mode_0755() {
    let project = project_with_bundle("precommit-materialize", "docs");
    init_git_repo(&project);

    let output = run_install(&project, false);

    assert!(output.status.success());
    let installed = project.join(".githooks/pre-commit");
    let expected =
        fs::read(corpus_root().join("pre-commit")).expect("corpus pre-commit is readable");
    let actual = fs::read(&installed).expect("pre-commit was installed");
    assert_eq!(actual, expected, "pre-commit bytes differ from the corpus");
    let mode = fs::metadata(&installed).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o755, "unexpected mode: {mode:o}");

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_succeeds_outside_a_git_repository_and_warns_on_stderr() {
    let project = project_with_bundle("precommit-no-git", "docs");

    let output = run_install(&project, false);

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("core.hooksPath"), "got: {stderr}");
    assert!(project.join(".githooks/pre-commit").exists());

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_dry_run_does_not_touch_githooks_or_git_config() {
    let project = project_with_bundle("precommit-dry-run", "docs");
    init_git_repo(&project);

    let output = run_install(&project, true);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pre-commit"), "got: {stdout}");
    assert!(!project.join(".githooks").exists());
    assert_eq!(read_git_config(&project, "core.hooksPath"), None);

    let _ = fs::remove_dir_all(&project);
}

#[test]
#[allow(clippy::too_many_lines)]
fn uninstall_removes_installed_artifacts_and_strips_settings_preserving_unrelated_entries() {
    let project = project_with_bundle("uninstall-full", "docs");
    init_git_repo(&project);
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(
        project.join(".claude/settings.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "unrelatedTopLevelKey": "keep-me",
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [ { "type": "command", "command": "echo custom-guard" } ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let install_output = run_install(&project, false);
    assert!(install_output.status.success());

    let uninstall_output = run_uninstall(&project, false);
    assert!(uninstall_output.status.success());

    for basename in SCRIPT_BASENAMES {
        assert!(!project.join(".living-docs/hooks").join(basename).exists());
    }
    assert!(!project.join(".githooks/pre-commit").exists());

    let settings = read_settings(&project);
    assert_eq!(settings["unrelatedTopLevelKey"], "keep-me");
    assert!(living_docs_entries(&settings, "PreToolUse").is_empty());
    assert!(living_docs_entries(&settings, "SessionStart").is_empty());
    let pre_tool_use = entries(&settings, "PreToolUse");
    assert!(
        pre_tool_use.iter().any(|entry| entry["matcher"] == "Bash"
            && entry["hooks"][0]["command"] == "echo custom-guard"),
        "got: {pre_tool_use:?}"
    );

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn uninstall_on_a_project_with_nothing_installed_is_a_clean_no_op() {
    let project = temp_dir("uninstall-clean");

    let output = run_uninstall(&project, false);

    assert!(output.status.success());
    assert!(!project.join(".living-docs").exists());
    assert!(!project.join(".githooks").exists());
    assert!(!project.join(".claude").exists());

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn uninstall_dry_run_reports_the_plan_and_changes_nothing() {
    let project = project_with_bundle("uninstall-dry-run", "docs");
    init_git_repo(&project);
    let install_output = run_install(&project, false);
    assert!(install_output.status.success());

    let before_settings = fs::read_to_string(project.join(".claude/settings.json")).unwrap();
    let before_scripts: Vec<Vec<u8>> = SCRIPT_BASENAMES
        .iter()
        .map(|basename| fs::read(project.join(".living-docs/hooks").join(basename)).unwrap())
        .collect();
    let before_pre_commit = fs::read(project.join(".githooks/pre-commit")).unwrap();

    let output = run_uninstall(&project, true);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for basename in SCRIPT_BASENAMES {
        assert!(stdout.contains(basename), "got: {stdout}");
    }
    assert!(stdout.contains("pre-commit"), "got: {stdout}");
    assert!(stdout.contains(".claude/settings.json"), "got: {stdout}");

    let after_settings = fs::read_to_string(project.join(".claude/settings.json")).unwrap();
    assert_eq!(before_settings, after_settings);
    for (basename, before_bytes) in SCRIPT_BASENAMES.iter().zip(before_scripts) {
        let after_bytes = fs::read(project.join(".living-docs/hooks").join(basename)).unwrap();
        assert_eq!(after_bytes, before_bytes);
    }
    let after_pre_commit = fs::read(project.join(".githooks/pre-commit")).unwrap();
    assert_eq!(after_pre_commit, before_pre_commit);

    let _ = fs::remove_dir_all(&project);
}

fn binary_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_living-docs"))
        .parent()
        .expect("the compiled binary has a parent directory")
        .to_path_buf()
}

fn path_with_binary_dir() -> std::ffi::OsString {
    let mut dirs = vec![binary_dir()];
    if let Some(current) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&current));
    }
    std::env::join_paths(dirs).expect("PATH components join cleanly")
}

/// The current `$PATH` with every directory carrying a `living-docs`
/// executable filtered out, so the pre-commit script's `command -v
/// living-docs` reliably fails regardless of what a developer's host or CI
/// runner happens to have installed.
fn path_without_living_docs() -> std::ffi::OsString {
    let current = std::env::var_os("PATH").unwrap_or_default();
    let filtered: Vec<PathBuf> = std::env::split_paths(&current)
        .filter(|dir| !dir.join("living-docs").is_file())
        .collect();
    std::env::join_paths(filtered).expect("PATH components join cleanly")
}

fn run_pre_commit_script(
    project: &Path,
    bundle: &str,
    path_override: Option<std::ffi::OsString>,
) -> Output {
    let script = corpus_root().join("pre-commit");
    let path_value = path_override.unwrap_or_else(path_with_binary_dir);
    Command::new("bash")
        .arg(&script)
        .current_dir(project)
        .env("LIVING_DOCS_BUNDLE", bundle)
        .env("PATH", path_value)
        .output()
        .expect("failed to run the pre-commit script")
}

fn only_record_path(adr_dir: &Path) -> PathBuf {
    fs::read_dir(adr_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.file_name().and_then(|name| name.to_str()) != Some("index.md"))
        .expect("the scaffolded record exists")
}

const CLEAN_FIXTURE_BODY: &str = "# 0001. Pre Commit Fixture\n\n## Context\n\nFixture body with no links, kept clean for the pre-commit script test.\n\n## Decision\n\nWe will keep this fixture minimal.\n\n## Consequences\n\nNone beyond the fixture.\n";

/// Replaces `contents`' body (below the closing frontmatter fence) with
/// [`CLEAN_FIXTURE_BODY`], keeping the frontmatter block byte-identical —
/// `check_canonical_frontmatter` compares only the frontmatter block, so
/// this never disturbs canonicality while dropping the ADR template's
/// placeholder links that would otherwise fail the link-validity invariant.
fn with_clean_body(contents: &str) -> String {
    let fence_end = contents
        .find("\n---\n")
        .expect("a scaffolded record has a closing frontmatter fence");
    let frontmatter = &contents[..fence_end + 5];
    format!("{frontmatter}\n{CLEAN_FIXTURE_BODY}")
}

/// Scaffolds a single canonical ADR record plus a bundle root `index.md`
/// under `<project>/<bundle>` via the real CLI (`new` + `index`), then
/// swaps in a link-free body — the whole tree passes `living-docs check`
/// unmodified, giving the pre-commit script fixture tests a bundle that is
/// clean by construction rather than hand-assembled.
fn scaffold_canonical_bundle(project: &Path, bundle: &str) {
    let docs_dir = project.join(bundle);
    let new_output = living_docs()
        .arg("--docs-dir")
        .arg(&docs_dir)
        .args(["new", "adr", "Pre Commit Fixture"])
        .output()
        .expect("failed to scaffold a record");
    assert!(new_output.status.success(), "new failed: {new_output:?}");

    let index_output = living_docs()
        .arg("--docs-dir")
        .arg(&docs_dir)
        .args(["index", "adr"])
        .output()
        .expect("failed to build the adr index");
    assert!(
        index_output.status.success(),
        "index failed: {index_output:?}"
    );

    let record_path = only_record_path(&docs_dir.join("adr"));
    let contents = fs::read_to_string(&record_path).unwrap();
    fs::write(&record_path, with_clean_body(&contents)).unwrap();

    fs::write(
        docs_dir.join("index.md"),
        "# Index\n\n- [ADRs](/adr/index.md)\n",
    )
    .unwrap();
}

/// Swaps the `type:`/`title:` frontmatter lines' order — a canonical record
/// always emits `type` before `title`, so this reordering is exactly the
/// hand-written drift `check_canonical_frontmatter` (ADR 0019) flags.
fn corrupt_frontmatter_key_order(record_path: &Path) {
    let contents = fs::read_to_string(record_path).unwrap();
    let mut lines: Vec<&str> = contents.lines().collect();
    let type_idx = lines
        .iter()
        .position(|line| line.starts_with("type:"))
        .expect("record carries a type: line");
    let title_idx = lines
        .iter()
        .position(|line| line.starts_with("title:"))
        .expect("record carries a title: line");
    lines.swap(type_idx, title_idx);
    fs::write(record_path, lines.join("\n") + "\n").unwrap();
}

#[test]
fn pre_commit_script_is_fail_open_when_no_binary_resolves() {
    let project = temp_dir("precommit-fail-open");
    init_git_repo(&project);

    let output = run_pre_commit_script(&project, "docs", Some(path_without_living_docs()));

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("skipped"), "got: {stderr}");

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn pre_commit_script_exits_zero_against_a_clean_bundle() {
    let project = temp_dir("precommit-clean");
    init_git_repo(&project);
    scaffold_canonical_bundle(&project, "bundle");

    let output = run_pre_commit_script(&project, "bundle", None);

    assert!(
        output.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn pre_commit_script_exits_nonzero_for_a_hand_written_noncanonical_record() {
    let project = temp_dir("precommit-noncanonical");
    init_git_repo(&project);
    scaffold_canonical_bundle(&project, "bundle");
    let record_path = only_record_path(&project.join("bundle/adr"));
    corrupt_frontmatter_key_order(&record_path);

    let output = run_pre_commit_script(&project, "bundle", None);

    assert!(!output.status.success());

    let _ = fs::remove_dir_all(&project);
}
