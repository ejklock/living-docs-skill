use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

mod common;
use common::{living_docs, run_check, run_check_advisories, stdout_of, write};

/// Filler lines that bring a record to a 121-line body, one over the advisory target.
const OVER_TARGET: usize = 107;
const BODY: &str = "## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n";

fn record(owner: bool, extra_lines: usize) -> String {
    let owner = if owner { "owner: a@b.c\n" } else { "" };
    let filler = (0..extra_lines)
        .map(|i| format!("line {i}\n"))
        .collect::<String>();
    format!("---\ntype: ADR\ntitle: Doc\ndescription: A record.\n{owner}status: Accepted\n---\n\n# Doc\n\n{BODY}{filler}")
}

fn bundle_with(label: &str, records: &[(String, String)], orphan: bool) -> PathBuf {
    let bundle = common::temp_bundle("digest", label);
    let mut index = String::from("# ADR Index\n\n");
    for (name, contents) in records {
        index.push_str(&format!("- [Doc](/adr/{name})\n"));
        write(&bundle, &format!("adr/{name}"), contents);
    }
    write(&bundle, "index.md", "# Docs\n\n- [ADRs](/adr/index.md)\n");
    write(&bundle, "adr/index.md", &index);
    common::index_bundle(&bundle);
    if orphan {
        write(&bundle, "adr/0099-orphan.md", &record(true, 0));
    }
    bundle
}

fn fixture(label: &str) -> PathBuf {
    bundle_with(
        label,
        &[
            ("0001-a.md".to_owned(), record(true, OVER_TARGET)),
            ("0002-b.md".to_owned(), record(true, OVER_TARGET)),
            ("0003-c.md".to_owned(), record(false, 0)),
        ],
        true,
    )
}

fn lines(output: &Output) -> Vec<String> {
    stdout_of(output).lines().map(str::to_owned).collect()
}

fn cleanup(bundle: &Path) {
    let _ = fs::remove_dir_all(bundle.parent().unwrap());
}

fn position(lines: &[String], needle: &str) -> usize {
    lines
        .iter()
        .position(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no line containing {needle:?} in {lines:#?}"))
}

#[test]
fn summary_collapses_advisories_into_one_line_per_kind_after_the_violation() {
    let bundle = fixture("summary");

    let output = run_check(&bundle);

    let out = lines(&output);
    let violation = position(&out, "0099-orphan.md");
    let size = position(&out, "2 SIZE advisories: 0001, 0002");
    let owner = position(&out, "1 OWNER advisory: 0003");
    let hint = position(&out, "--advisories");
    let verdict = position(&out, "FAIL");
    assert!(
        violation < size && size < owner && owner < hint && hint < verdict,
        "{out:#?}"
    );
    let summaries = out.iter().filter(|l| l.contains("advisor")).count();
    assert_eq!(summaries, 3, "{out:#?}");
    let text = stdout_of(&output);
    assert!(
        !text.contains("121 lines") && !text.contains("has no owner"),
        "{text}"
    );
    assert_eq!(output.status.code(), Some(1));
    cleanup(&bundle);
}

#[test]
fn summary_names_the_first_five_records_and_counts_the_rest() {
    let records: Vec<(String, String)> = (1..=6)
        .map(|n| (format!("000{n}-r.md"), record(true, OVER_TARGET)))
        .collect();
    let bundle = bundle_with("many", &records, false);

    let output = run_check(&bundle);

    let text = stdout_of(&output);
    assert!(
        text.contains("6 SIZE advisories: 0001, 0002, 0003, 0004, 0005 (+1)"),
        "{text}"
    );
    assert_eq!(output.status.code(), Some(0));
    cleanup(&bundle);
}

#[test]
fn summary_follows_the_header_after_exactly_one_blank_line_when_there_are_no_violations() {
    let bundle = bundle_with(
        "blank",
        &[("0001-a.md".to_owned(), record(true, OVER_TARGET))],
        false,
    );

    let output = run_check(&bundle);

    let out = lines(&output);
    assert_eq!(out[1], "", "{out:#?}");
    assert!(out[2].contains("1 SIZE advisory: 0001"), "{out:#?}");
    cleanup(&bundle);
}

#[test]
fn summary_is_absent_when_there_are_no_advisories() {
    let bundle = bundle_with("none", &[("0001-a.md".to_owned(), record(true, 0))], false);

    let output = run_check(&bundle);

    let text = stdout_of(&output);
    assert!(
        !text.contains("advisor") && !text.contains("--advisories"),
        "{text}"
    );
    assert!(text.contains("OK"), "{text}");
    assert_eq!(output.status.code(), Some(0));
    cleanup(&bundle);
}

#[test]
fn advisories_flag_lists_every_advisory_after_the_summary() {
    let bundle = fixture("flag");

    let output = run_check_advisories(&bundle);

    let out = lines(&output);
    position(&out, "2 SIZE advisories: 0001, 0002");
    let summary = position(&out, "1 OWNER advisory: 0003");
    let size = position(&out, "121 lines");
    let owner = position(&out, "has no owner");
    let verdict = position(&out, "FAIL");
    assert!(
        summary < size && size < verdict && owner < verdict,
        "{out:#?}"
    );
    assert_eq!(output.status.code(), Some(1));
    cleanup(&bundle);
}

#[test]
fn advisories_flag_is_accepted_with_mermaid_only() {
    let bundle = fixture("mermaid");
    let file = bundle.join("adr/0001-a.md");
    let file = file.to_str().unwrap();

    let plain = living_docs()
        .args(["check", "--mermaid-only", file, "--plain"])
        .output()
        .unwrap();
    let flagged = living_docs()
        .args(["check", "--mermaid-only", "--advisories", file, "--plain"])
        .output()
        .unwrap();

    assert_eq!(stdout_of(&plain), stdout_of(&flagged));
    assert_eq!(plain.status.code(), flagged.status.code());
    assert_eq!(flagged.status.code(), Some(0));
    cleanup(&bundle);
}

#[test]
fn json_is_byte_identical_with_and_without_the_flag_and_keeps_absolute_files() {
    let bundle = fixture("json");
    let path = bundle.to_str().unwrap();

    let without = living_docs()
        .args(["check", path, "--json"])
        .output()
        .unwrap();
    let with = living_docs()
        .args(["check", path, "--json", "--advisories"])
        .output()
        .unwrap();

    assert_eq!(without.stdout, with.stdout);
    let value: serde_json::Value = serde_json::from_slice(&without.stdout).unwrap();
    let advisories = value["advisories"].as_array().unwrap();
    assert_eq!(advisories.len(), 3);
    for advisory in advisories {
        let file = advisory["file"].as_str().unwrap();
        assert!(file.starts_with(path), "{advisory}");
        assert!(advisory["kind"].is_string() && advisory["message"].is_string());
    }
    cleanup(&bundle);
}

#[test]
fn relative_paths_show_the_bundle_relative_form_for_absolute_and_relative_bundles() {
    let bundle = fixture("relative");
    let absolute = bundle.to_str().unwrap();

    let by_absolute = stdout_of(&run_check_advisories(&bundle));
    let by_relative = stdout_of(
        &living_docs()
            .current_dir(bundle.parent().unwrap())
            .args(["check", "docs", "--plain", "--advisories"])
            .output()
            .unwrap(),
    );

    for text in [&by_absolute, &by_relative] {
        for line in text
            .lines()
            .filter(|l| l.starts_with("  adr/") || l.starts_with("  /"))
        {
            assert!(line.starts_with("  adr/"), "{line}");
        }
        assert!(!text.contains(&format!("{absolute}/adr/0001")), "{text}");
        assert!(text.contains("  adr/0099-orphan.md"), "{text}");
        assert!(text.contains("  adr/0001-a.md"), "{text}");
    }
    cleanup(&bundle);
}
