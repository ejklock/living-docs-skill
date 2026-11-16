//! The decision-prompt hook still reads its working directory when the path
//! holds characters JSON escapes.

mod common;
use common::decision_prompt::{edit_code, repo, run_from, Cleanup};
use std::path::{Path, PathBuf};

fn escaped_json(dir: &Path, file: &str) -> String {
    let esc = |p: PathBuf| {
        p.display()
            .to_string()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    };
    let (file, cwd) = (esc(dir.join(file)), esc(dir.to_path_buf()));
    format!(r#"{{"tool_name":"Edit","tool_input":{{"file_path":"{file}"}},"cwd":"{cwd}"}}"#)
}

#[test]
fn decision_prompt_cwd_with_a_quote_a_backslash_or_a_space_leads_the_hook_to_the_repository() {
    let elsewhere = std::env::temp_dir().join("decision-prompt-elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("mkdir");
    let _elsewhere = Cleanup(elsewhere.clone());
    for name in ["q\"uote", "back\\slash", "sp ace"] {
        for project_dir in [None, Some(elsewhere.as_path())] {
            let dir = repo(&format!("{name}-{}", project_dir.is_some()), "main");
            let _dir = Cleanup(dir.clone());
            edit_code(&dir, "fn b() {}\n");
            std::fs::write(dir.join("docs/x.md"), "x\n").expect("write");
            let bundle = escaped_json(&dir, "docs/x.md");
            assert_eq!(run_from(&elsewhere, project_dir, &bundle), "", "{name}");
            std::fs::remove_file(dir.join("docs/x.md")).expect("rm");
            let code = escaped_json(&dir, "code.rs");
            let out = run_from(&elsewhere, project_dir, &code);
            assert!(out.contains("additionalContext"), "{name}: {out}");
            assert_eq!(run_from(&elsewhere, project_dir, &code), "", "{name}");
        }
    }
}
