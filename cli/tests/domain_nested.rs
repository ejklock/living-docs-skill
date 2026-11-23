mod common;

use common::{index_bundle, living_docs, run_check, stdout_of, temp_bundle, write};
use std::fs;
use std::path::Path;
use std::process::Output;

fn record(title: &str) -> String {
    format!(
        "---\ntype: Context\ntitle: {title}\ndescription: A record.\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n# {title}\n\nFilled text.\n"
    )
}

fn nested_bundle(label: &str) -> common::Scratch {
    let docs = temp_bundle("domain-nested", label);
    for (path, title) in [
        ("glossary", "Glossary"),
        ("billing/invoices", "Invoices"),
        ("billing/late-fees", "Late fees"),
        ("billing/tax/vat", "VAT"),
        ("scheduling/slots", "Slots"),
    ] {
        write(&docs, &format!("context/{path}.md"), &record(title));
    }
    index_bundle(&docs);
    docs
}

fn read(docs: &Path, rel: &str) -> String {
    fs::read_to_string(docs.join(rel)).unwrap()
}

fn cli(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap()])
        .args(args)
        .output()
        .unwrap()
}

fn position(index: &str, needle: &str) -> usize {
    index
        .find(needle)
        .unwrap_or_else(|| panic!("{needle} not listed in:\n{index}"))
}

#[test]
fn index_groups_nested_records_by_folder_and_check_stays_clean() {
    let docs = nested_bundle("index");
    let index = read(&docs, "context/index.md");
    let order = [
        "(glossary.md)",
        "## billing\n",
        "(billing/invoices.md)",
        "(billing/late-fees.md)",
        "## billing/tax\n",
        "(billing/tax/vat.md)",
        "## scheduling\n",
        "(scheduling/slots.md)",
    ];
    let places: Vec<usize> = order.iter().map(|n| position(&index, n)).collect();
    assert!(places.windows(2).all(|pair| pair[0] < pair[1]), "{index}");

    index_bundle(&docs);
    assert_eq!(read(&docs, "context/index.md"), index);
    let checked = run_check(&docs);
    assert_eq!(checked.status.code(), Some(0), "{}", stdout_of(&checked));

    write(&docs, "context/legal/terms.md", &record("Terms"));
    let drift = stdout_of(&run_check(&docs));
    assert!(drift.contains("regeneration"), "{drift}");
    index_bundle(&docs);
    let healed = run_check(&docs);
    assert_eq!(healed.status.code(), Some(0), "{}", stdout_of(&healed));
}

fn set_description(docs: &Path, reference: &str, value: &str) -> Output {
    cli(
        docs,
        &["write", "set", reference, "description", value, "--plain"],
    )
}

#[test]
fn reach_nested_records_by_path_and_by_reference() {
    let docs = nested_bundle("reach-set");
    let by_path = set_description(&docs, "context/billing/tax/vat.md", "x");
    assert!(by_path.status.success(), "{by_path:?}");
    assert!(read(&docs, "context/billing/tax/vat.md").contains("description: x"));

    let by_reference = set_description(&docs, "domain/billing/tax/vat", "y");
    assert!(by_reference.status.success(), "{by_reference:?}");
    assert!(read(&docs, "context/billing/tax/vat.md").contains("description: y"));
    let checked = run_check(&docs);
    assert_eq!(checked.status.code(), Some(0), "{}", stdout_of(&checked));

    let refused = set_description(&docs, "domain/Billing/vat", "z");
    assert!(!refused.status.success());
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("'Billing'"), "{stderr}");
}

#[test]
fn reach_nested_records_by_read_and_keep_a_hand_index_reserved() {
    let docs = nested_bundle("reach-read");
    let records = stdout_of(&cli(&docs, &["read", "records", "--plain"]));
    assert!(records.contains("VAT"), "{records}");
    assert!(records.contains("Slots"), "{records}");

    write(&docs, "context/billing/index.md", "# Billing\n");
    let hand = run_check(&docs);
    assert_eq!(hand.status.code(), Some(0), "{}", stdout_of(&hand));
    index_bundle(&docs);
    assert!(!read(&docs, "context/index.md").contains("billing/index.md"));
}

fn report_line<'a>(report: &'a str, path: &str) -> &'a str {
    report_line_with(report, path, "")
}

fn report_line_with<'a>(report: &'a str, path: &str, text: &str) -> &'a str {
    report
        .lines()
        .find(|line| line.contains(path) && line.contains(text))
        .unwrap_or_else(|| panic!("{path} not reported in:\n{report}"))
}

fn one_level_bundle() -> common::Scratch {
    let docs = temp_bundle("domain-nested", "unchanged");
    let view = "---\ntype: Architecture View\ntitle: X\nkind: context\ndescription: A view.\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n# X\n";
    let adr = "---\ntype: ADR\ntitle: X\ndescription: A decision.\nowner: Dev\nstatus: Accepted\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n# 0001. X\n\n## Context\n\nc\n\n## Decision\n\nd\n\n## Consequences\n\ne\n";
    write(&docs, "architecture/index.md", "# Architecture\n");
    write(&docs, "architecture/sub/x.md", view);
    write(&docs, "architecture/other/index.md", "# Other\n");
    write(&docs, "adr/index.md", "# ADR Index\n");
    write(&docs, "adr/sub/0001-x.md", adr);
    write(
        &docs,
        "index.md",
        "# Docs\n\n* [Architecture](architecture/)\n* [ADRs](adr/)\n",
    );
    docs
}

#[test]
fn unchanged_views_and_numbered_records_keep_one_level() {
    let docs = one_level_bundle();
    let listed = cli(&docs, &["manage", "index", "architecture", "--plain"]);
    assert!(listed.status.success(), "{listed:?}");
    assert!(!read(&docs, "architecture/index.md").contains("x.md"));
    assert!(!read(&docs, "adr/index.md").contains("0001-x.md"));

    let report = stdout_of(&run_check(&docs));
    for (record, folder) in [
        ("architecture/sub/x.md", "architecture/sub"),
        ("adr/sub/0001-x.md", "adr/sub"),
    ] {
        let line = report_line(&report, record);
        let expected = format!(
            "no index.md in its directory ({}) — orphan",
            docs.join(folder).display()
        );
        assert!(line.contains(&expected), "{line}");
    }
    let unreachable = report_line(&report, "architecture/other/index.md");
    assert!(unreachable.contains("not reachable"), "{unreachable}");
}

fn placeholder_record(title: &str) -> String {
    format!(
        "---\ntype: Context\ntitle: {title}\ndescription: <One sentence — what this domain record defines.>\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n# {title}\n\nFilled text.\n"
    )
}

#[test]
fn repair_hint_names_the_reference_of_a_nested_record() {
    let docs = temp_bundle("domain-nested", "hint");
    write(
        &docs,
        "context/billing/tax/rates.md",
        &placeholder_record("Rates"),
    );
    write(
        &docs,
        "context/billing/context/deep.md",
        &placeholder_record("Deep"),
    );
    index_bundle(&docs);
    let report = stdout_of(&run_check(&docs));
    let rates = report_line_with(&report, "billing/tax/rates.md", "placeholder");
    assert!(
        rates.contains("write set domain/billing/tax/rates description"),
        "{rates}"
    );
    let deep = report_line_with(&report, "billing/context/deep.md", "placeholder");
    assert!(
        deep.contains("write set domain/billing/context/deep description"),
        "{deep}"
    );
}

#[test]
fn repair_hint_keeps_sits_outside_for_a_bundle_under_a_context_folder() {
    let docs =
        common::Scratch::with_subdir("living-docs-domain-nested-test-outside", "context/docs");
    write(&docs, "notes/stray.md", &placeholder_record("Stray"));
    let report = stdout_of(&run_check(&docs));
    let stray = report_line_with(&report, "notes/stray.md", "placeholder");
    assert!(
        stray.contains("sits outside the place `write new` gives its type"),
        "{stray}"
    );
    assert!(!stray.contains("write set domain"), "{stray}");
}

#[test]
fn an_empty_reference_segment_is_refused_by_name() {
    let docs = nested_bundle("empty-segment");
    for reference in ["domain/billing//tax/vat", "domain/billing/tax/"] {
        let refused = set_description(&docs, reference, "x");
        assert_eq!(refused.status.code(), Some(2), "{reference}: {refused:?}");
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert!(
            stderr.contains("'' is not a domain slug"),
            "{reference}: {stderr}"
        );
    }
    assert!(read(&docs, "context/billing/tax/vat.md").contains("description: A record."));
}

#[test]
fn folder_headings_sort_by_path_component() {
    let docs = nested_bundle("sort");
    write(&docs, "context/billing-x/note.md", &record("Note"));
    index_bundle(&docs);
    let index = read(&docs, "context/index.md");
    assert!(
        position(&index, "## billing/tax\n") < position(&index, "## billing-x\n"),
        "{index}"
    );
}
