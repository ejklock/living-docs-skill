//! The generated `context/index.md` lists each domain record with its title
//! and description, and `write new domain --dir` refreshes that index at any
//! depth so the record is never an orphan.

mod common;

use common::{living_docs, run_check, stdout_of};
use std::fs;
use std::path::Path;
use std::process::Output;

const BODY: &str = "## Terms\n\nA term.\n";

fn run(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap()])
        .args(args)
        .output()
        .unwrap()
}

fn new_domain(docs: &Path, title: &str, dir: Option<&str>, description: Option<&str>) -> Output {
    let body = docs.parent().unwrap().join("body.md");
    fs::write(&body, BODY).unwrap();
    let mut args = vec!["write", "new", "domain", title];
    args.extend(dir.into_iter().flat_map(|dir| ["--dir", dir]));
    args.extend(
        description
            .into_iter()
            .flat_map(|text| ["--description", text]),
    );
    args.extend(["--body", body.to_str().unwrap()]);
    let output = run(docs, &args);
    assert!(output.status.success(), "{output:?}");
    output
}

fn context_index(docs: &Path) -> String {
    fs::read_to_string(docs.join("context/index.md")).unwrap()
}

#[test]
fn domain_rows_carry_the_description_after_a_dash() {
    let docs = common::temp_bundle("domain-index-rows", "described");
    new_domain(
        &docs,
        "Billing rules",
        Some("billing/invoices"),
        Some("Rules of billing."),
    );

    let index = context_index(&docs);

    assert!(
        index
            .contains("* [Billing rules](billing/invoices/billing-rules.md) — Rules of billing.\n"),
        "{index}"
    );
}

#[test]
fn domain_rows_omit_the_dash_for_a_placeholder_description() {
    let docs = common::temp_bundle("domain-index-rows", "placeholder");
    new_domain(&docs, "Glossary", None, None);

    let index = context_index(&docs);

    assert!(index.contains("* [Glossary](glossary.md)\n"), "{index}");
    assert!(!index.contains("glossary.md) —"), "{index}");
}

#[test]
fn domain_rows_are_stable_under_a_second_index_run() {
    let docs = common::temp_bundle("domain-index-rows", "stable");
    new_domain(&docs, "Billing rules", Some("billing"), Some("Rules."));
    let before = context_index(&docs);

    let again = run(&docs, &["manage", "index", "domain"]);

    assert!(again.status.success(), "{again:?}");
    assert_eq!(context_index(&docs), before);
}

#[test]
fn view_rows_keep_their_kind_suffix() {
    let docs = common::temp_bundle("domain-index-rows", "view");
    common::write(
        &docs,
        "architecture/context.md",
        "---\ntype: Architecture View\ntitle: Context\ndescription: Shows the system.\nkind: context\n---\n\n# Context\n",
    );

    let indexed = run(&docs, &["manage", "index", "view"]);

    assert!(indexed.status.success(), "{indexed:?}");
    let index = fs::read_to_string(docs.join("architecture/index.md")).unwrap();
    assert!(
        index.contains("* [Context](context.md) - context\n"),
        "{index}"
    );
}

#[test]
fn numbered_rows_keep_their_status_suffix() {
    let docs = common::temp_bundle("domain-index-rows", "numbered");
    let created = run(&docs, &["write", "new", "adr", "Pick one", "--plain"]);
    assert!(created.status.success(), "{created:?}");

    let index = fs::read_to_string(docs.join("adr/index.md")).unwrap();

    assert!(
        index.contains("* [0001 — Pick one](0001-pick-one.md) - Proposed\n"),
        "{index}"
    );
}

fn assert_listed_and_checked(docs: &Path, dir: &str) {
    new_domain(docs, "Glossary", None, Some("Terms."));
    let nested = new_domain(docs, "Fees", Some(dir), Some("Fees."));

    let reported = String::from_utf8_lossy(&nested.stderr).into_owned() + &stdout_of(&nested);
    assert!(reported.contains("context/index.md"), "{reported}");
    let index = context_index(docs);
    assert!(index.contains("glossary.md) — Terms."), "{index}");
    assert!(
        index.contains(&format!("{dir}/fees.md) — Fees.")),
        "{index}"
    );
    let checked = run_check(docs);
    assert_eq!(checked.status.code(), Some(0), "{}", stdout_of(&checked));
}

#[test]
fn refresh_after_a_nested_write_lists_the_new_record() {
    let docs = common::temp_bundle("domain-index-rows", "refresh-one");
    assert_listed_and_checked(&docs, "billing/fees");
}

#[test]
fn refresh_after_a_deeply_nested_write_lists_the_new_record() {
    let docs = common::temp_bundle("domain-index-rows", "refresh-deep");
    assert_listed_and_checked(&docs, "a/b/c");
}

#[test]
fn domain_rows_omit_the_dash_for_a_missing_or_empty_description() {
    let docs = common::temp_bundle("domain-index-rows", "missing-or-empty");
    let record = |description: &str| {
        format!("---\ntype: Context\ntitle: T\n{description}timestamp: 2026-06-20T00:00:00Z\n---\n\n# T\n")
    };
    common::write(&docs, "context/none.md", &record(""));
    common::write(&docs, "context/empty.md", &record("description: \"\"\n"));

    let indexed = run(&docs, &["manage", "index", "domain"]);

    assert!(indexed.status.success(), "{indexed:?}");
    let index = context_index(&docs);
    assert!(index.contains("* [T](none.md)\n"), "{index}");
    assert!(index.contains("* [T](empty.md)\n"), "{index}");
}

#[test]
fn domain_rows_show_a_description_link_as_its_text_and_check_passes() {
    let docs = common::temp_bundle("domain-index-rows", "link");
    new_domain(&docs, "Glossary", None, Some("Terms."));
    common::write(&docs, "context/billing/fees/l.png", "png");
    new_domain(
        &docs,
        "Fees",
        Some("billing/fees"),
        Some("See [the glossary](../../glossary.md), ![logo](l.png) and <https://x.dev>."),
    );

    let index = context_index(&docs);

    assert!(
        index.contains("billing/fees/fees.md) — See the glossary, logo and https://x.dev.\n"),
        "{index}"
    );
    let checked = run_check(&docs);
    assert_eq!(checked.status.code(), Some(0), "{}", stdout_of(&checked));
}

#[test]
fn domain_rows_drop_inline_html_tags_and_keep_their_text() {
    let docs = common::temp_bundle("domain-index-rows", "html");
    common::write(&docs, "context/billing/q.md", "# Q\n");
    new_domain(
        &docs,
        "Fees",
        Some("billing/fees"),
        Some(r#"Html <a href="../q.md">q</a> tag."#),
    );

    let index = context_index(&docs);

    assert!(
        index.contains("billing/fees/fees.md) — Html q tag.\n"),
        "{index}"
    );
}

#[test]
fn refresh_after_a_description_change_updates_a_deeply_nested_row() {
    let docs = common::temp_bundle("domain-index-rows", "set-description");
    new_domain(&docs, "Tax", Some("a/b/c"), Some("Old."));

    let set = run(
        &docs,
        &[
            "write",
            "set",
            "domain/a/b/c/tax",
            "description",
            "Taxes and levies.",
        ],
    );

    assert!(set.status.success(), "{set:?}");
    let index = context_index(&docs);
    assert!(
        index.contains("* [Tax](a/b/c/tax.md) — Taxes and levies.\n"),
        "{index}"
    );
}
