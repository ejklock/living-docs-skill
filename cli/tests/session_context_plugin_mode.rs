//! In plugin mode the SessionStart hook prints its line only for a living-docs
//! bundle root: an `index.md` whose frontmatter holds `okf_version:`.

use std::path::Path;
use std::process::Command;

const MARKER: &str = "---\nokf_version: \"0.1\"\n---\n\n# Docs\n";

/// Writes a bundle root marker file at `dir/index.md` under `root`.
fn write_marked_bundle(root: &Path, dir: &str, contents: &str) {
    let bundle = root.join(dir);
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(bundle.join("index.md"), contents).unwrap();
}

fn fresh_project(name: &str) -> std::path::PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create scratch project");
    let init = Command::new("git")
        .args(["init", "-q"])
        .arg(&root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("git init");
    assert!(init.success());
    root
}

/// Runs the script against `root`; `plugin` is the value of
/// `LIVING_DOCS_PLUGIN` (unset when `None`), `path` replaces `PATH` when given.
fn run_hook(
    root: &Path,
    plugin: Option<&str>,
    bundle: Option<&str>,
    path: Option<&str>,
) -> (String, bool) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skills/living-docs/hooks/session-context.sh");
    let mut cmd = Command::new("bash");
    cmd.arg(&script)
        .env_remove("LIVING_DOCS_PLUGIN")
        .env_remove("LIVING_DOCS_BUNDLE")
        .env("CLAUDE_PROJECT_DIR", root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    if let Some(value) = plugin {
        cmd.env("LIVING_DOCS_PLUGIN", value);
    }
    if let Some(value) = bundle {
        cmd.env("LIVING_DOCS_BUNDLE", value);
    }
    if let Some(value) = path {
        cmd.env("PATH", value);
    }
    let output = cmd
        .output()
        .expect("failed to run the session-context script");
    (
        String::from_utf8(output.stdout).expect("hook output is utf-8"),
        output.status.success(),
    )
}

fn hooks_path(root: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "--local", "core.hooksPath"])
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git config");
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn assert_silent(root: &Path, bundle: Option<&str>) {
    let (out, ok) = run_hook(root, Some("1"), bundle, None);
    assert!(ok);
    assert_eq!(out, "");
}

#[test]
fn plugin_mode_is_silent_without_a_bundle_directory() {
    assert_silent(&fresh_project("plugin-no-bundle"), None);
}

#[test]
fn plugin_mode_is_silent_when_the_bundle_is_a_regular_file() {
    let root = fresh_project("plugin-bundle-file");
    std::fs::write(root.join("docs"), "not a directory").unwrap();
    assert_silent(&root, None);
}

#[test]
fn plugin_mode_is_silent_for_a_bundle_directory_without_an_index() {
    let root = fresh_project("plugin-no-index");
    std::fs::create_dir(root.join("docs")).unwrap();
    assert_silent(&root, None);
}

#[test]
fn plugin_mode_is_silent_for_an_index_without_frontmatter() {
    let root = fresh_project("plugin-index-no-frontmatter");
    write_marked_bundle(&root, "docs", "# Welcome\nokf_version: \"0.1\"\n");
    assert_silent(&root, None);
}

#[test]
fn plugin_mode_is_silent_when_okf_version_is_only_in_the_body() {
    let root = fresh_project("plugin-marker-in-body");
    write_marked_bundle(
        &root,
        "docs",
        "---\ntitle: Notes\n---\n\nokf_version: \"0.1\"\n",
    );
    assert_silent(&root, None);
}

#[test]
fn plugin_mode_is_silent_for_an_empty_index() {
    let root = fresh_project("plugin-empty-index");
    write_marked_bundle(&root, "docs", "");
    assert_silent(&root, None);
}

#[test]
fn plugin_mode_is_silent_when_the_index_is_a_directory() {
    let root = fresh_project("plugin-index-directory");
    std::fs::create_dir_all(root.join("docs/index.md")).unwrap();
    assert_silent(&root, None);
}

#[test]
fn plugin_mode_follows_the_configured_bundle_directory() {
    let root = fresh_project("plugin-custom-bundle");
    write_marked_bundle(&root, "notes", MARKER);
    let (out, ok) = run_hook(&root, Some("1"), Some("notes"), None);
    assert!(ok);
    assert!(out.contains("living-docs: "), "{out}");
    assert_eq!(out.trim_end_matches('\n').lines().count(), 1, "{out}");
}

#[test]
fn plugin_mode_prints_the_single_session_line_for_a_marked_bundle() {
    let root = fresh_project("plugin-with-bundle");
    write_marked_bundle(&root, "docs", MARKER);
    let (out, ok) = run_hook(&root, Some("1"), None, None);
    assert!(ok);
    assert_eq!(out.trim_end_matches('\n').lines().count(), 1, "{out}");
    assert!(out.contains("TYPE/NNNN"), "{out}");
    assert!(out.contains("set <TYPE/NNNN> number next"), "{out}");
}

#[test]
fn plugin_mode_accepts_a_marker_file_with_crlf_line_endings() {
    let root = fresh_project("plugin-crlf-marker");
    write_marked_bundle(&root, "docs", &MARKER.replace('\n', "\r\n"));
    let (out, ok) = run_hook(&root, Some("1"), None, None);
    assert!(ok);
    assert!(out.contains("living-docs: "), "{out}");
}

#[test]
fn plugin_mode_never_changes_git_configuration() {
    let root = fresh_project("plugin-keeps-git-config");
    write_marked_bundle(&root, "docs", MARKER);
    std::fs::create_dir(root.join(".githooks")).unwrap();
    let (_, ok) = run_hook(&root, Some("1"), None, None);
    assert!(ok);
    assert_eq!(hooks_path(&root), "");
}

#[test]
fn outside_plugin_mode_the_hook_arms_the_githooks_directory() {
    for plugin in [None, Some("0"), Some("")] {
        let root = fresh_project("plugin-off-arms-githooks");
        std::fs::create_dir(root.join("docs")).unwrap();
        std::fs::create_dir(root.join(".githooks")).unwrap();
        let (_, ok) = run_hook(&root, plugin, None, None);
        assert!(ok);
        assert_eq!(hooks_path(&root), ".githooks", "plugin={plugin:?}");
    }
}

#[test]
fn outside_plugin_mode_the_hook_prints_without_a_bundle_directory() {
    let root = fresh_project("plugin-off-no-bundle");
    let (out, ok) = run_hook(&root, Some("0"), None, None);
    assert!(ok);
    assert!(out.contains("living-docs: "), "{out}");
}

#[test]
fn plugin_mode_without_a_cli_names_the_npm_and_install_script_installs() {
    let root = fresh_project("plugin it's here");
    write_marked_bundle(&root, "docs", MARKER);
    let (out, ok) = run_hook(&root, Some("1"), None, Some("/usr/bin:/bin"));
    assert!(ok);
    assert!(out.contains("npm install -g @ejklock/living-docs"), "{out}");
    assert!(out.contains("install.sh"), "{out}");
    assert!(!out.contains("make build"), "{out}");
    assert_eq!(out.trim_end_matches('\n').lines().count(), 1, "{out}");
}

#[test]
fn outside_plugin_mode_without_a_cli_still_names_make_build() {
    let root = fresh_project("plugin off it's here");
    std::fs::create_dir(root.join("docs")).unwrap();
    let (out, ok) = run_hook(&root, None, None, Some("/usr/bin:/bin"));
    assert!(ok);
    assert!(out.contains("make build"), "{out}");
    assert!(!out.contains("npm install -g"), "{out}");
}

#[test]
fn plugin_mode_accepts_a_marker_file_behind_a_byte_order_mark() {
    for (name, contents) in [
        ("plugin-bom-marker", format!("\u{feff}{MARKER}")),
        (
            "plugin-bom-crlf-marker",
            format!("\u{feff}{}", MARKER.replace('\n', "\r\n")),
        ),
    ] {
        let root = fresh_project(name);
        write_marked_bundle(&root, "docs", &contents);
        let (out, ok) = run_hook(&root, Some("1"), None, None);
        assert!(ok);
        assert!(out.contains("living-docs: "), "{name}: {out}");
    }
}

#[test]
fn plugin_mode_is_silent_when_okf_version_is_outside_the_frontmatter_behind_a_byte_order_mark() {
    let root = fresh_project("plugin-bom-marker-in-body");
    write_marked_bundle(
        &root,
        "docs",
        "\u{feff}---\ntitle: Notes\n---\n\nokf_version: \"0.1\"\n",
    );
    assert_silent(&root, None);
}

#[test]
fn plugin_mode_accepts_a_byte_order_mark_marker_under_posix_awk() {
    let root = fresh_project("plugin-bom-posix");
    write_marked_bundle(&root, "docs", &format!("\u{feff}{MARKER}"));
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skills/living-docs/hooks/session-context.sh");
    let output = Command::new("bash")
        .arg(&script)
        .env("LIVING_DOCS_PLUGIN", "1")
        .env("POSIXLY_CORRECT", "1")
        .env("CLAUDE_PROJECT_DIR", &root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("failed to run the session-context script");
    let out = String::from_utf8(output.stdout).expect("hook output is utf-8");
    assert!(output.status.success());
    assert!(out.contains("living-docs: "), "{out}");
}

#[test]
fn plugin_mode_is_silent_for_two_byte_order_marks_before_the_frontmatter() {
    let root = fresh_project("plugin-bom-double");
    write_marked_bundle(&root, "docs", &format!("\u{feff}\u{feff}{MARKER}"));
    assert_silent(&root, None);
}

#[test]
fn plugin_mode_is_silent_for_a_byte_order_mark_inside_the_frontmatter() {
    let root = fresh_project("plugin-bom-line-two");
    write_marked_bundle(
        &root,
        "docs",
        "---\n\u{feff}okf_version: \"0.1\"\n---\n\n# Docs\n",
    );
    assert_silent(&root, None);
}
