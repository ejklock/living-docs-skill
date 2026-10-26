use crate::skill;
use serde_json::{json, Value};
use std::borrow::Cow;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{fs, io};

pub(crate) mod gitattributes;

const HOOK_ASSET_PATHS: [&str; 1] = ["living-docs/hooks/session-context.sh"];
const PRE_COMMIT_ASSET_PATH: &str = "living-docs/hooks/pre-commit";
const GIT_HOOKS_DEST_SUBDIR: &str = ".githooks";
const GIT_HOOKS_PATH_VALUE: &str = ".githooks";

const HOOKS_DEST_SUBDIR: &str = ".living-docs/hooks";
const SCRIPT_MODE: u32 = 0o755;
const SETTINGS_REL_PATH: &str = ".claude/settings.json";
/// Stripped on uninstall only, so a project wired by an older release that
/// still carried the write-gate entry is left clean.
const LEGACY_PRE_TOOL_USE_SECTION: &str = "PreToolUse";
const SESSION_START_SECTION: &str = "SessionStart";

struct HookScript {
    basename: &'static str,
    bytes: Cow<'static, [u8]>,
}

struct HookEntrySpec {
    script_basename: &'static str,
    section: &'static str,
    matcher: Option<&'static str>,
}

/// Materializes the session-teaching hook into `<project_root>/.living-docs/hooks/`
/// at mode 0755, materializes the pre-commit doc-gate to
/// `<project_root>/.githooks/pre-commit` and points `core.hooksPath` at it,
/// then wires the `SessionStart` hook into `<project_root>/.claude/settings.json`
/// idempotently, replacing any prior living-docs entry by identity. The
/// bundle pinned into each generated command's `LIVING_DOCS_BUNDLE=` is
/// `docs_dir`, required to already exist; `dry_run` reports the same plan
/// and changes nothing. A missing asset, absent `docs_dir`, or unparseable
/// settings file is a hard error (stderr, `ExitCode::from(2)`); a failed
/// `core.hooksPath` write is only a warning (silenced by `quiet`).
#[allow(clippy::too_many_lines)]
pub(crate) fn install(
    project_root: &Path,
    docs_dir: &Path,
    dry_run: bool,
    quiet: bool,
) -> ExitCode {
    if let Err(message) = validate_docs_dir(project_root, docs_dir) {
        return report_failure(&message);
    }
    let scripts = match resolve_scripts() {
        Ok(scripts) => scripts,
        Err(message) => return report_failure(&message),
    };
    let pre_commit = match resolve_one(PRE_COMMIT_ASSET_PATH) {
        Ok(script) => script,
        Err(message) => return report_failure(&message),
    };
    let settings_path = project_root.join(SETTINGS_REL_PATH);
    let mut settings = match load_settings(&settings_path) {
        Ok(settings) => settings,
        Err(message) => return report_failure(&message),
    };
    let bundle = docs_dir.to_string_lossy().into_owned();
    apply_hook_entries(&mut settings, &bundle);
    if dry_run {
        announce_dry_run(&scripts);
        announce_dry_run_pre_commit();
        announce_dry_run_wiring(&settings_path, &bundle);
        return ExitCode::SUCCESS;
    }
    if let Err(err) = write_scripts(project_root, &scripts) {
        return report_failure(&err.to_string());
    }
    if let Err(err) = write_script_to(&project_root.join(GIT_HOOKS_DEST_SUBDIR), &pre_commit) {
        return report_failure(&err.to_string());
    }
    arm_git_hooks_path(project_root, quiet);
    match write_settings(&settings_path, &settings) {
        Ok(()) => {
            println!("wired {}", settings_path.display());
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err.to_string()),
    }
}

/// Removes the artifacts [`install`] wrote — the `.living-docs/hooks/`
/// script, `.githooks/pre-commit`, and the living-docs entries in
/// `<project_root>/.claude/settings.json` — leaving unrelated entries,
/// unrelated top-level keys, and `core.hooksPath` untouched. A project
/// carrying none of these artifacts is a clean no-op: exit 0, nothing
/// created, nothing deleted. Under `dry_run`, reports the same removal plan
/// and changes nothing on disk. An existing settings file that fails to
/// parse as JSON is a hard error, never overwritten.
pub(crate) fn uninstall(project_root: &Path, dry_run: bool) -> ExitCode {
    let removable = removable_artifacts(project_root);
    let settings_path = project_root.join(SETTINGS_REL_PATH);
    let settings_update = match plan_settings_removal(&settings_path) {
        Ok(update) => update,
        Err(message) => return report_failure(&message),
    };
    if dry_run {
        announce_uninstall_dry_run(&removable, settings_update.is_some(), &settings_path);
        return ExitCode::SUCCESS;
    }
    if let Err(err) = remove_artifacts(&removable) {
        return report_failure(&err.to_string());
    }
    if let Some(settings) = settings_update {
        if let Err(err) = write_settings(&settings_path, &settings) {
            return report_failure(&err.to_string());
        }
        println!("stripped {}", settings_path.display());
    }
    ExitCode::SUCCESS
}

fn validate_docs_dir(project_root: &Path, docs_dir: &Path) -> Result<(), String> {
    if project_root.join(docs_dir).is_dir() {
        return Ok(());
    }
    Err(format!(
        "--docs-dir {} does not name an existing directory under {}",
        docs_dir.display(),
        project_root.display()
    ))
}

fn resolve_scripts() -> Result<Vec<HookScript>, String> {
    HOOK_ASSET_PATHS.into_iter().map(resolve_one).collect()
}

fn resolve_one(path: &'static str) -> Result<HookScript, String> {
    let bytes = skill::asset(path).ok_or_else(|| format!("missing embedded asset: {path}"))?;
    Ok(HookScript {
        basename: basename_of(path),
        bytes,
    })
}

fn basename_of(path: &'static str) -> &'static str {
    path.rsplit('/').next().unwrap_or(path)
}

fn announce_dry_run(scripts: &[HookScript]) {
    for script in scripts {
        println!("[dry-run] would write {}", dest_display(script.basename));
    }
}

fn announce_dry_run_pre_commit() {
    println!(
        "[dry-run] would write {GIT_HOOKS_DEST_SUBDIR}/{}",
        basename_of(PRE_COMMIT_ASSET_PATH)
    );
}

fn announce_dry_run_wiring(settings_path: &Path, bundle: &str) {
    println!(
        "[dry-run] would wire {} (LIVING_DOCS_BUNDLE={bundle})",
        settings_path.display()
    );
}

fn dest_display(basename: &str) -> String {
    format!("{HOOKS_DEST_SUBDIR}/{basename}")
}

fn write_scripts(project_root: &Path, scripts: &[HookScript]) -> io::Result<()> {
    let dest_dir = project_root.join(HOOKS_DEST_SUBDIR);
    scripts
        .iter()
        .try_for_each(|script| write_script_to(&dest_dir, script))
}

/// Writes `script` into `dest_dir` (creating it if needed) at [`SCRIPT_MODE`]
/// — shared by [`write_scripts`] and [`install`]'s own pre-commit write.
fn write_script_to(dest_dir: &Path, script: &HookScript) -> io::Result<()> {
    fs::create_dir_all(dest_dir)?;
    let dest = dest_dir.join(script.basename);
    fs::write(&dest, script.bytes.as_ref())?;
    fs::set_permissions(&dest, fs::Permissions::from_mode(SCRIPT_MODE))?;
    println!("wrote {}", dest.display());
    Ok(())
}

/// Best-effort `git config core.hooksPath`; any failure is a warning only,
/// never fatal (the installer must succeed outside a git repo).
fn arm_git_hooks_path(project_root: &Path, quiet: bool) {
    let outcome = crate::doc_gate::git_in(project_root)
        .args(["config", "core.hooksPath", GIT_HOOKS_PATH_VALUE])
        .output();
    match outcome {
        Ok(result) if result.status.success() => {}
        Ok(result) => warn_hooks_path(quiet, String::from_utf8_lossy(&result.stderr).trim()),
        Err(err) => warn_hooks_path(quiet, &err.to_string()),
    }
}

fn warn_hooks_path(quiet: bool, detail: &str) {
    let message = format!(
        "living-docs install hooks: could not set core.hooksPath ({detail}) — the pre-commit doc-gate will not run automatically outside a git repository"
    );
    crate::output::note(quiet, &message);
}

fn hook_entry_specs() -> [HookEntrySpec; 1] {
    [HookEntrySpec {
        script_basename: basename_of(HOOK_ASSET_PATHS[0]),
        section: SESSION_START_SECTION,
        matcher: None,
    }]
}

fn living_docs_hook_marker() -> String {
    format!("{HOOKS_DEST_SUBDIR}/")
}

fn build_entry(spec: &HookEntrySpec, bundle: &str) -> Value {
    let command = format!(
        "LIVING_DOCS_BUNDLE={bundle} \"$CLAUDE_PROJECT_DIR\"/{HOOKS_DEST_SUBDIR}/{}",
        spec.script_basename
    );
    let mut entry = json!({ "hooks": [ { "type": "command", "command": command } ] });
    if let Some(matcher) = spec.matcher {
        entry["matcher"] = json!(matcher);
    }
    entry
}

fn apply_hook_entries(settings: &mut Value, bundle: &str) {
    let marker = living_docs_hook_marker();
    for spec in hook_entry_specs() {
        let entry = build_entry(&spec, bundle);
        let section = ensure_hooks_section(settings, spec.section);
        section.retain(|existing| !is_living_docs_entry(existing, &marker));
        section.push(entry);
    }
}

fn load_settings(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|err| format!("{} is not valid JSON: {err}", path.display()))?;
    if !value.is_object() {
        return Err(format!("{} does not contain a JSON object", path.display()));
    }
    Ok(value)
}

fn ensure_hooks_section<'a>(settings: &'a mut Value, section: &str) -> &'a mut Vec<Value> {
    let hooks = ensure_child_object(settings, "hooks");
    ensure_child_array(hooks, section)
}

fn ensure_child_object<'a>(value: &'a mut Value, key: &str) -> &'a mut Value {
    let object = value
        .as_object_mut()
        .expect("settings value is validated as an object before this call");
    let child = object.entry(key).or_insert_with(|| json!({}));
    if !child.is_object() {
        *child = json!({});
    }
    object.get_mut(key).expect("key was just inserted")
}

fn ensure_child_array<'a>(value: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    let object = value
        .as_object_mut()
        .expect("settings value is validated as an object before this call");
    let child = object.entry(key).or_insert_with(|| json!([]));
    if !child.is_array() {
        *child = json!([]);
    }
    object
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("key was just normalized to an array")
}

fn is_living_docs_entry(entry: &Value, marker: &str) -> bool {
    entry
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|hooks| hooks.iter().any(|hook| hook_command_contains(hook, marker)))
}

fn hook_command_contains(hook: &Value, marker: &str) -> bool {
    hook.get("command")
        .and_then(Value::as_str)
        .is_some_and(|command| command.contains(marker))
}

fn write_settings(path: &Path, settings: &Value) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut rendered = serde_json::to_string_pretty(settings)
        .expect("a settings value built from string literals always serializes");
    rendered.push('\n');
    fs::write(path, rendered)
}

fn removable_artifacts(project_root: &Path) -> Vec<PathBuf> {
    let hooks_dir = project_root.join(HOOKS_DEST_SUBDIR);
    let mut candidates: Vec<PathBuf> = HOOK_ASSET_PATHS
        .iter()
        .map(|asset_path| hooks_dir.join(basename_of(asset_path)))
        .collect();
    candidates.push(
        project_root
            .join(GIT_HOOKS_DEST_SUBDIR)
            .join(basename_of(PRE_COMMIT_ASSET_PATH)),
    );
    candidates
        .into_iter()
        .filter(|path| path.is_file())
        .collect()
}

/// `None` when `settings_path` does not exist (untouched, so uninstall stays
/// a clean no-op) or carries no living-docs entry to strip; `Some` with the
/// filtered value otherwise.
fn plan_settings_removal(settings_path: &Path) -> Result<Option<Value>, String> {
    if !settings_path.is_file() {
        return Ok(None);
    }
    let mut settings = load_settings(settings_path)?;
    if strip_hook_entries(&mut settings) {
        Ok(Some(settings))
    } else {
        Ok(None)
    }
}

fn strip_hook_entries(settings: &mut Value) -> bool {
    let marker = living_docs_hook_marker();
    let pre_tool_use_changed =
        strip_section_entries(settings, LEGACY_PRE_TOOL_USE_SECTION, &marker);
    let session_start_changed = strip_section_entries(settings, SESSION_START_SECTION, &marker);
    pre_tool_use_changed || session_start_changed
}

fn strip_section_entries(settings: &mut Value, section: &str, marker: &str) -> bool {
    let Some(array) = settings
        .get_mut("hooks")
        .and_then(|hooks| hooks.get_mut(section))
        .and_then(Value::as_array_mut)
    else {
        return false;
    };
    let before = array.len();
    array.retain(|entry| !is_living_docs_entry(entry, marker));
    array.len() != before
}

fn remove_artifacts(paths: &[PathBuf]) -> io::Result<()> {
    paths.iter().try_for_each(|path| remove_one(path))
}

fn remove_one(path: &Path) -> io::Result<()> {
    fs::remove_file(path)?;
    println!("removed {}", path.display());
    Ok(())
}

fn announce_uninstall_dry_run(removable: &[PathBuf], strips_settings: bool, settings_path: &Path) {
    for path in removable {
        println!("[dry-run] would remove {}", path.display());
    }
    if strips_settings {
        println!(
            "[dry-run] would strip living-docs entries from {}",
            settings_path.display()
        );
    }
}

fn report_failure(message: &str) -> ExitCode {
    eprintln!("living-docs: {message}");
    ExitCode::from(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_scripts_finds_the_session_teaching_asset() {
        let scripts = resolve_scripts().expect("the hook script is embedded");
        let basenames: Vec<&str> = scripts.iter().map(|script| script.basename).collect();
        assert_eq!(basenames, vec!["session-context.sh"]);
        assert!(scripts.iter().all(|script| !script.bytes.is_empty()));
    }

    #[test]
    fn is_living_docs_entry_true_when_any_nested_command_contains_the_marker() {
        let marker = living_docs_hook_marker();
        let entry = json!({
            "hooks": [
                { "type": "command", "command": "echo unrelated" },
                {
                    "type": "command",
                    "command": format!(
                        "LIVING_DOCS_BUNDLE=docs \"$CLAUDE_PROJECT_DIR\"/{marker}session-context.sh"
                    )
                }
            ]
        });
        assert!(is_living_docs_entry(&entry, &marker));
    }

    #[test]
    fn is_living_docs_entry_false_when_no_command_mentions_the_marker() {
        let marker = living_docs_hook_marker();
        let entry = json!({ "hooks": [ { "type": "command", "command": "echo unrelated" } ] });
        assert!(!is_living_docs_entry(&entry, &marker));
    }

    #[test]
    fn is_living_docs_entry_false_when_the_entry_carries_no_hooks_array() {
        let marker = living_docs_hook_marker();
        assert!(!is_living_docs_entry(&json!({ "matcher": "X" }), &marker));
    }

    #[test]
    fn ensure_child_array_normalizes_a_non_array_value_instead_of_panicking() {
        let mut settings = json!({ "hooks": "not-an-object" });
        let section = ensure_hooks_section(&mut settings, SESSION_START_SECTION);
        assert!(section.is_empty());
    }

    #[test]
    fn resolve_one_finds_the_embedded_pre_commit_asset() {
        let script = resolve_one(PRE_COMMIT_ASSET_PATH).expect("pre-commit is embedded");
        assert_eq!(script.basename, "pre-commit");
        assert!(script.bytes.starts_with(b"#!"));
    }

    fn scratch_project(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("living-docs-hooks-unit-{label}-{nanos}"));
        fs::create_dir_all(&dir).expect("create scratch project dir");
        dir
    }

    #[test]
    fn removable_artifacts_is_empty_for_a_project_with_nothing_installed() {
        let project = scratch_project("removable-empty");
        assert!(removable_artifacts(&project).is_empty());
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn removable_artifacts_lists_only_the_files_actually_present() {
        let project = scratch_project("removable-partial");
        let hooks_dir = project.join(HOOKS_DEST_SUBDIR);
        fs::create_dir_all(&hooks_dir).unwrap();
        fs::write(
            hooks_dir.join("session-context.sh"),
            b"#!/usr/bin/env bash\n",
        )
        .unwrap();

        let artifacts = removable_artifacts(&project);

        assert_eq!(artifacts, vec![hooks_dir.join("session-context.sh")]);
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn strip_hook_entries_removes_only_the_living_docs_entries_including_a_legacy_write_gate() {
        let mut settings = json!({
            "hooks": {
                "PreToolUse": [
                    { "matcher": "Bash", "hooks": [ { "type": "command", "command": "echo custom" } ] },
                    { "matcher": "Write|Edit|MultiEdit", "hooks": [ { "type": "command", "command": format!("\"$CLAUDE_PROJECT_DIR\"/{}block-docs-handwrite.sh", living_docs_hook_marker()) } ] }
                ]
            }
        });

        let changed = strip_hook_entries(&mut settings);

        assert!(changed);
        let remaining = settings["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0]["matcher"], "Bash");
    }

    #[test]
    fn strip_hook_entries_reports_no_change_when_there_is_nothing_to_strip() {
        let mut settings = json!({
            "hooks": {
                "PreToolUse": [
                    { "matcher": "Bash", "hooks": [ { "type": "command", "command": "echo custom" } ] }
                ]
            }
        });

        assert!(!strip_hook_entries(&mut settings));
        assert_eq!(settings["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn strip_hook_entries_reports_no_change_when_the_hooks_key_is_absent() {
        let mut settings = json!({ "unrelatedTopLevelKey": "keep-me" });
        assert!(!strip_hook_entries(&mut settings));
    }
}
