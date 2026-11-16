//! Reading, shaping and writing the Claude Code `settings.json` the hook
//! installer wires into; every helper leaves keys it does not own untouched.

use serde_json::{json, Value};
use std::path::Path;
use std::{fs, io};

pub(super) fn load_settings(path: &Path) -> Result<Value, String> {
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

pub(super) fn ensure_hooks_section<'a>(
    settings: &'a mut Value,
    section: &str,
) -> &'a mut Vec<Value> {
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

pub(super) fn is_living_docs_entry(entry: &Value, marker: &str) -> bool {
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

pub(super) fn write_settings(path: &Path, settings: &Value) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut rendered = serde_json::to_string_pretty(settings)
        .expect("a settings value built from string literals always serializes");
    rendered.push('\n');
    fs::write(path, rendered)
}

pub(super) fn strip_section_entries(settings: &mut Value, section: &str, marker: &str) -> bool {
    let Some(array) = settings
        .get_mut("hooks")
        .and_then(|hooks| hooks.get_mut(section))
        .and_then(Value::as_array_mut)
    else {
        return false;
    };
    let before = array.len();
    array.retain(|entry| !is_living_docs_entry(entry, marker));
    let changed = array.len() != before;
    if changed && array.is_empty() {
        drop_emptied_section(settings, section);
    }
    changed
}

/// Removes a section left empty by the strip, and the `hooks` object when
/// that was its last section, so an uninstall leaves no scaffolding behind.
fn drop_emptied_section(settings: &mut Value, section: &str) {
    let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) else {
        return;
    };
    hooks.remove(section);
    if hooks.is_empty() {
        settings
            .as_object_mut()
            .expect("settings value is validated as an object before this call")
            .remove("hooks");
    }
}
