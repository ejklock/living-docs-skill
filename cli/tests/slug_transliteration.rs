use living_docs_core::paths::transliterate;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

mod common;
use common::Scratch;

fn temp_dir(label: &str) -> Scratch {
    Scratch::new(&format!("living-docs-slug-test-{label}"))
}

fn run(docs: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
        .args(["--docs-dir", docs.to_str().unwrap()])
        .args(args)
        .arg("--plain")
        .output()
        .expect("failed to run living-docs")
}

fn first_stdout_line(output: &Output) -> String {
    assert!(output.status.success(), "{output:?}");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap()
        .to_string()
}

#[test]
fn new_issue_with_an_accented_title_gets_an_ascii_filename() {
    let docs = temp_dir("new");
    let path = first_stdout_line(&run(&docs, &["new", "issue", "Horário de atendimento"]));
    assert!(path.ends_with("-horario-de-atendimento.md"), "{path}");
}

#[test]
fn set_title_with_accents_renames_to_an_ascii_slug() {
    let docs = temp_dir("retitle");
    first_stdout_line(&run(&docs, &["new", "issue", "Plain"]));
    let out = run(&docs, &["set", "1", "title", "Ação"]);
    assert!(out.status.success(), "{out:?}");
    let names: Vec<String> = fs::read_dir(docs.join("issues"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert!(names.iter().any(|n| n.ends_with("-acao.md")), "{names:?}");
}

fn title_of(record: &str) -> String {
    let line = record
        .lines()
        .find(|l| l.starts_with("title:"))
        .expect("record has a title");
    line["title:".len()..]
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string()
}

#[test]
fn transliteration_leaves_every_ascii_title_in_the_bundle_untouched() {
    let docs = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../docs"));
    let mut checked = 0;
    for dir in ["adr", "issues"] {
        for entry in fs::read_dir(docs.join(dir)).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let numbered = name.split_once('-').is_some_and(|(num, _)| {
                num.chars().all(|c| c.is_ascii_digit()) && name.ends_with(".md")
            });
            if !numbered {
                continue;
            }
            let title = title_of(&fs::read_to_string(&path).unwrap());
            if !title.is_ascii() {
                continue;
            }
            assert_eq!(transliterate(&title), title, "{name}");
            checked += 1;
        }
    }
    assert!(checked > 50, "only {checked} ascii titles found");
}
