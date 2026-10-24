use super::{attributes, project, stderr_of, UNION_LINE};
use crate::common::living_docs;
use std::fs;
use std::path::Path;
use std::process::Output;

const VENDOR_LINE: &str = "vendor/**/index.md merge=union\n";

fn install_in(project: &Path, docs_dir: &str, cwd: Option<&Path>) -> Output {
    let mut command = living_docs();
    command.args(["install", "hooks"]);
    match cwd {
        Some(dir) => command.current_dir(dir),
        None => command.arg("--dir").arg(project),
    };
    command.args(["--docs-dir", docs_dir]).output().unwrap()
}

fn uninstall_bundle(project: &Path, docs_dir: &str) -> Output {
    living_docs()
        .args(["uninstall", "hooks", "--dir"])
        .arg(project)
        .args(["--docs-dir", docs_dir])
        .output()
        .unwrap()
}

#[test]
fn a_foreign_glob_line_survives_install_and_uninstall_of_another_bundle() {
    let project = project("vendor-line");
    fs::write(project.join(".gitattributes"), VENDOR_LINE).unwrap();
    assert!(install_in(&project, "docs", None).status.success());
    assert!(uninstall_bundle(&project, "docs").status.success());
    assert_eq!(attributes(&project), VENDOR_LINE);
}

#[test]
fn two_bundles_each_add_their_own_line_and_uninstall_removes_only_its_own() {
    let project = project("two-bundles");
    fs::create_dir_all(project.join("handbook")).unwrap();
    assert!(install_in(&project, "docs", None).status.success());
    assert!(install_in(&project, "handbook", None).status.success());
    let both = attributes(&project);
    assert!(both.contains(UNION_LINE) && both.contains("handbook/**/index.md merge=union"));
    assert!(uninstall_bundle(&project, "handbook").status.success());
    assert_eq!(attributes(&project), format!("{UNION_LINE}\n"));
}

#[test]
fn an_absolute_docs_dir_inside_the_project_writes_the_relative_pattern() {
    let project = project("absolute-inside");
    let docs = fs::canonicalize(project.join("docs")).unwrap();
    let output = install_in(&project, docs.to_str().unwrap(), Some(&project));
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert_eq!(attributes(&project), format!("{UNION_LINE}\n"));
}

#[test]
fn a_parent_relative_docs_dir_resolving_outside_the_project_skips_the_line_and_says_so() {
    let project = project("parent-relative");
    let sibling = project.with_file_name(format!(
        "{}-sibling",
        project.file_name().unwrap().to_string_lossy()
    ));
    fs::create_dir_all(sibling.join("docs")).unwrap();
    let relative = format!(
        "../{}-sibling/docs",
        project.file_name().unwrap().to_string_lossy()
    );
    let output = install_in(&project, &relative, None);
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(!project.join(".gitattributes").exists());
    let stderr = stderr_of(&output);
    assert_eq!(stderr.matches("merge=union").count(), 1, "{stderr}");
    let _ = fs::remove_dir_all(&sibling);
}

#[test]
fn a_failing_hooks_step_leaves_the_attributes_file_as_it_was() {
    let project = project("failing-hooks");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(project.join(".claude/settings.json"), "{not json").unwrap();
    let output = install_in(&project, "docs", None);
    assert!(!output.status.success());
    assert!(!project.join(".gitattributes").exists());
    fs::write(project.join(".gitattributes"), VENDOR_LINE).unwrap();
    assert!(!install_in(&project, "docs", None).status.success());
    assert_eq!(attributes(&project), VENDOR_LINE);
}

fn uninstall_with_global(project: &Path, global: &str, own: &str) -> Output {
    living_docs()
        .args(["--docs-dir", global, "uninstall", "hooks", "--dir"])
        .arg(project)
        .args(["--docs-dir", own])
        .output()
        .unwrap()
}

fn uninstall_dry_run(project: &Path, docs_dir: &str) -> Output {
    living_docs()
        .args(["uninstall", "hooks", "--dry-run", "--dir"])
        .arg(project)
        .args(["--docs-dir", docs_dir])
        .output()
        .unwrap()
}

#[test]
fn a_repeated_docs_dir_flag_removes_the_line_of_the_last_one_in_either_order() {
    let project = project("repeated-flag");
    fs::create_dir_all(project.join("hb")).unwrap();
    assert!(install_in(&project, "docs", None).status.success());
    assert!(install_in(&project, "hb", None).status.success());
    assert!(uninstall_with_global(&project, "docs", "hb")
        .status
        .success());
    assert_eq!(attributes(&project), format!("{UNION_LINE}\n"));
    assert!(install_in(&project, "hb", None).status.success());
    assert!(uninstall_with_global(&project, "hb", "docs")
        .status
        .success());
    assert_eq!(attributes(&project), "hb/**/index.md merge=union\n");
}

#[test]
fn uninstall_with_a_non_utf8_dir_does_not_panic() {
    use std::os::unix::ffi::OsStrExt;
    let dir = std::ffi::OsStr::from_bytes(b"x\xff");
    let output = living_docs()
        .args(["uninstall", "hooks", "--dry-run", "--dir"])
        .arg(dir)
        .output()
        .unwrap();
    assert_ne!(output.status.code(), Some(101), "{}", stderr_of(&output));
}

#[test]
fn a_failing_uninstall_hooks_step_keeps_the_union_line() {
    let project = project("failing-uninstall");
    assert!(install_in(&project, "docs", None).status.success());
    let before = attributes(&project);
    fs::write(project.join(".claude/settings.json"), "{not json").unwrap();
    assert!(!uninstall_bundle(&project, "docs").status.success());
    assert_eq!(attributes(&project), before);
}

#[test]
fn uninstall_after_the_docs_dir_was_deleted_still_removes_its_line() {
    let project = project("deleted-docs");
    assert!(install_in(&project, "docs", None).status.success());
    fs::remove_dir_all(project.join("docs")).unwrap();
    assert!(uninstall_bundle(&project, "docs").status.success());
    assert!(!project.join(".gitattributes").exists());
}

#[test]
fn uninstall_dry_run_names_the_line_only_when_present_and_writes_nothing() {
    let project = project("dry-run-remove");
    assert!(install_in(&project, "docs", None).status.success());
    let before = attributes(&project);
    let plan = |project: &Path| {
        let output = uninstall_dry_run(project, "docs");
        String::from_utf8_lossy(&output.stdout).to_string()
    };
    let present = plan(&project);
    assert!(
        present.contains(&format!("would remove {UNION_LINE}")),
        "{present}"
    );
    assert_eq!(attributes(&project), before);
    fs::write(project.join(".gitattributes"), VENDOR_LINE).unwrap();
    let absent = plan(&project);
    assert!(!absent.contains("merge=union"), "{absent}");
    assert_eq!(attributes(&project), VENDOR_LINE);
}
