//! The operations live under `write`, `read` and `manage`; the former
//! top-level names and a bare verb are usage errors.

mod common;

use common::{living_docs, Scratch};
use std::process::Output;

fn run(args: &[&str]) -> Output {
    living_docs()
        .args(args)
        .output()
        .expect("failed to run living-docs")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn each_former_top_level_name_is_an_unknown_subcommand() {
    for name in [
        "new",
        "set",
        "supersede",
        "index",
        "fmt",
        "guide",
        "install",
        "uninstall",
        "completions",
    ] {
        let output = run(&[name]);
        assert_eq!(output.status.code(), Some(2), "{name}");
        assert!(
            stderr_of(&output).contains("unrecognized subcommand"),
            "{name}: {}",
            stderr_of(&output)
        );
    }
}

#[test]
fn a_bare_verb_prints_its_operations_and_exits_2() {
    for (verb, operations) in [
        ("write", ["new", "set", "supersede"].as_slice()),
        ("read", ["records", "guide"].as_slice()),
        (
            "manage",
            ["index", "fmt", "install", "uninstall", "completions"].as_slice(),
        ),
    ] {
        let output = run(&["--plain", verb]);
        assert_eq!(output.status.code(), Some(2), "{verb}");
        let text = stderr_of(&output);
        for operation in operations {
            assert!(text.contains(operation), "{verb} lacks {operation}: {text}");
        }
    }
}

#[test]
fn read_records_without_a_flag_exits_0() {
    let scratch = Scratch::new("grouped-verbs-read-records");
    let docs = scratch.join("docs");
    let docs = docs.to_str().unwrap();
    let created = run(&["--docs-dir", docs, "write", "new", "adr", "A"]);
    assert_eq!(created.status.code(), Some(0), "{}", stderr_of(&created));
    let output = run(&["--docs-dir", docs, "read", "records"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
}

#[test]
fn check_without_an_argument_validates_the_docs_dir_bundle() {
    let scratch = Scratch::new("grouped-verbs-check");
    let docs = scratch.join("docs");
    let docs = docs.to_str().unwrap();
    let created = run(&["--docs-dir", docs, "write", "new", "adr", "A"]);
    assert_eq!(created.status.code(), Some(0), "{}", stderr_of(&created));
    let output = run(&["--docs-dir", docs, "check", "--plain"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains(docs), "{stdout}");
}
