//! The text the CLI writes for a retired record names no command, and a
//! bundle written with an earlier wording is repaired by `manage fmt` and
//! `manage index`.

mod common;

use common::{index_bundle, living_docs, run_check, stdout_of, temp_bundle, write};
use std::fs;
use std::path::Path;

const SUPERSEDED: &str =
    "> **SUPERSEDED — do not act on this record.** Replaced by [0002](0002-d.md).";
const DEPRECATED: &str = "> **DEPRECATED — do not act on this record.** It has no successor.";
const NOTE: &str = "_History only. Do not act on these records._";
const EARLIER_COMMANDS: [&str; 3] = ["effective", "read", "read records"];
const BODY: &str = "## Context\n\nWhy.\n\n## Decision\n\nWhat.\n\n## Consequences\n\nSo.\n\n## Verification\n\n- Checked.\n";

fn adr(number: &str, extra: &str, callout: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: Decision {number}\ndescription: A record\nowner: Someone\n{extra}timestamp: 2026-01-01T00:00:00Z\n---\n\n{callout}# {number}. Decision {number}\n\n{BODY}"
    )
}

fn earlier(callout: &str, command: &str) -> String {
    format!("{callout} Run `living-docs {command}` for what is in force.\n\n")
}

fn bundle_with(label: &str, superseded: &str, deprecated: &str) -> common::Scratch {
    let bundle = temp_bundle("legacy-retired-text", label);
    write(&bundle, "index.md", "# Docs\n\n* [adr](adr/)\n");
    let retired = "status: Superseded\nsuperseded_by: 0002\n";
    write(&bundle, "adr/0001-d.md", &adr("0001", retired, superseded));
    write(
        &bundle,
        "adr/0002-d.md",
        &adr("0002", "status: Accepted\nsupersedes: 0001\n", ""),
    );
    write(
        &bundle,
        "adr/0003-d.md",
        &adr("0003", "status: Deprecated\n", deprecated),
    );
    index_bundle(&bundle);
    bundle
}

fn run_ok(bundle: &Path, args: &[&str]) {
    let output = living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap()])
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{args:?}: {}", stdout_of(&output));
}

fn run_verbs(bundle: &Path) {
    run_ok(bundle, &["manage", "fmt", bundle.to_str().unwrap()]);
    run_ok(bundle, &["manage", "index", "adr"]);
}

fn read_all(bundle: &Path) -> Vec<String> {
    [
        "adr/0001-d.md",
        "adr/0002-d.md",
        "adr/0003-d.md",
        "adr/index.md",
    ]
    .iter()
    .map(|rel| fs::read_to_string(bundle.join(rel)).unwrap())
    .collect()
}

fn replace_note_line(bundle: &Path, note: &str) {
    let path = bundle.join("adr/index.md");
    let text = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = text
        .lines()
        .map(|line| {
            if line.starts_with("_History only.") {
                note
            } else {
                line
            }
        })
        .collect();
    fs::write(path, format!("{}\n", lines.join("\n"))).unwrap();
}

fn lines_starting(text: &str, prefix: &str) -> Vec<String> {
    text.lines()
        .filter(|line| line.starts_with(prefix))
        .map(str::to_owned)
        .collect()
}

fn assert_green(bundle: &Path) {
    let output = run_check(bundle);
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}

fn adr_file(bundle: &Path, prefix: &str) -> String {
    let found = fs::read_dir(bundle.join("adr"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .find(|name| name.starts_with(prefix))
        .unwrap();
    format!("adr/{found}")
}

#[test]
fn no_command_is_named_in_what_the_cli_writes_for_a_retired_record() {
    let bundle = temp_bundle("legacy-retired-text", "no-command");
    for title in ["Old", "New", "Gone"] {
        assert!(common::run_new(&bundle, "adr", title).status.success());
    }
    let superseded = common::run_supersede(&bundle, "0001", "0002");
    assert!(superseded.status.success());
    run_ok(&bundle, &["write", "set", "0003", "status", "Deprecated"]);
    run_ok(&bundle, &["manage", "index", "adr"]);
    let read = |prefix: &str| fs::read_to_string(bundle.join(adr_file(&bundle, prefix))).unwrap();
    let successor = adr_file(&bundle, "0002-");
    let successor = successor.strip_prefix("adr/").unwrap();
    let expected =
        format!("> **SUPERSEDED — do not act on this record.** Replaced by [0002]({successor}).");
    let old = read("0001-");
    assert!(old.contains(&format!("\n{expected}\n\n")), "{old}");
    let gone = read("0003-");
    assert!(gone.contains(&format!("\n{DEPRECATED}\n\n")), "{gone}");
    let listing = read("index");
    assert!(listing.contains(&format!("\n{NOTE}\n")), "{listing}");
    let written = [
        lines_starting(&old, "> **"),
        lines_starting(&gone, "> **"),
        lines_starting(&listing, "_History only."),
    ]
    .concat();
    assert_eq!(written.len(), 3, "{written:?}");
    for line in written {
        assert!(!line.contains("living-docs"), "{line}");
    }
}

#[test]
fn a_bundle_with_an_earlier_wording_migrates_through_fmt_then_index() {
    for command in EARLIER_COMMANDS {
        let superseded = earlier(SUPERSEDED, command);
        let deprecated = earlier(DEPRECATED, command);
        let bundle = bundle_with("migrates", &superseded, &deprecated);
        let note = format!(
            "_History only. Do not act on these records — run `living-docs {command}` for what is in force._"
        );
        replace_note_line(&bundle, &note);
        let before = run_check(&bundle);
        let report = stdout_of(&before);
        assert_eq!(before.status.code(), Some(1), "{report}");
        for name in ["0001-d.md", "0003-d.md", "index.md"] {
            assert!(report.contains(name), "{command}: {report}");
        }
        run_verbs(&bundle);
        assert_green(&bundle);
        let once = read_all(&bundle);
        run_verbs(&bundle);
        assert_eq!(once, read_all(&bundle), "{command}");
        assert!(once[0].contains(&format!("\n{SUPERSEDED}\n\n")));
        assert!(!once.concat().contains("for what is in force"));
    }
}

#[test]
fn a_carriage_return_record_keeps_its_line_endings_when_it_migrates() {
    let superseded = earlier(SUPERSEDED, "effective");
    let bundle = bundle_with("crlf", &superseded, &format!("{DEPRECATED}\n\n"));
    let path = bundle.join("adr/0001-d.md");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, text.replace('\n', "\r\n")).unwrap();
    run_verbs(&bundle);
    let repaired = fs::read_to_string(&path).unwrap();
    assert!(
        repaired.contains(&format!("{SUPERSEDED}\r\n\r\n")),
        "{repaired:?}"
    );
    assert_eq!(
        repaired.matches('\n').count(),
        repaired.matches("\r\n").count()
    );
    assert_green(&bundle);
}

#[test]
fn a_record_whose_callout_was_deleted_migrates_to_the_new_callout() {
    let bundle = bundle_with("deleted", "", &format!("{DEPRECATED}\n\n"));
    assert_eq!(run_check(&bundle).status.code(), Some(1));
    run_verbs(&bundle);
    assert!(read_all(&bundle)[0].contains(&format!("\n{SUPERSEDED}\n\n")));
    assert_green(&bundle);
}
