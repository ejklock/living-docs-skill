mod common;

use common::{
    index_bundle, living_docs, run_check, run_check_advisories, stdout_of, temp_bundle, write,
};
use std::fs;
use std::path::Path;
use std::process::Output;

fn docs_arg(docs: &Path) -> [&str; 2] {
    ["--docs-dir", docs.to_str().unwrap()]
}

fn new_domain(docs: &Path, extra: &[&str]) -> Output {
    living_docs()
        .args(docs_arg(docs))
        .args(["write", "new"])
        .args(extra)
        .output()
        .unwrap()
}

fn set(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(docs_arg(docs))
        .args(["write", "set"])
        .args(args)
        .arg("--plain")
        .output()
        .unwrap()
}

fn read(docs: &Path, rel: &str) -> String {
    fs::read_to_string(docs.join(rel)).unwrap()
}

fn fill_slots(docs: &Path, rel: &str) {
    let filled: Vec<String> = read(docs, rel)
        .lines()
        .map(|line| match line.contains("{{SLOT") {
            true => "Filled text.".to_string(),
            false => line.to_string(),
        })
        .collect();
    fs::write(docs.join(rel), filled.join("\n") + "\n").unwrap();
}

fn billing(docs: &Path) {
    let args = [
        "domain",
        "Billing rules",
        "--description",
        "Rules of billing.",
    ];
    let out = new_domain(docs, &args);
    assert!(out.status.success(), "{out:?}");
}

#[test]
fn a_new_domain_record_is_scaffolded_indexed_and_checked() {
    let docs = temp_bundle("domain-records", "scaffold");
    billing(&docs);
    let record = read(&docs, "context/billing-rules.md");
    assert!(record.contains("type: Context"), "{record}");
    assert!(record.contains("# Billing rules"), "{record}");
    assert!(
        record.contains("## Terms") && record.contains("## Rules"),
        "{record}"
    );
    assert!(record.contains("{{SLOT"), "{record}");

    fill_slots(&docs, "context/billing-rules.md");
    let checked = run_check(&docs);
    assert_eq!(checked.status.code(), Some(0), "{}", stdout_of(&checked));
    assert!(read(&docs, "context/index.md").contains("(billing-rules.md)"));
    assert!(read(&docs, "index.md").contains("(context/)"));

    let before = (read(&docs, "context/index.md"), read(&docs, "index.md"));
    index_bundle(&docs);
    let after = (read(&docs, "context/index.md"), read(&docs, "index.md"));
    assert_eq!(before, after);

    let listed = living_docs()
        .args(docs_arg(&docs))
        .args(["read", "records", "--plain"])
        .output()
        .unwrap();
    let listing = stdout_of(&listed);
    assert!(listing.contains("Billing rules"), "{listing}");
}

#[test]
fn a_domain_record_is_edited_retitled_and_refuses_a_status() {
    let docs = temp_bundle("domain-records", "edit");
    billing(&docs);
    fill_slots(&docs, "context/billing-rules.md");

    let described = set(&docs, &["context/billing-rules.md", "description", "x"]);
    assert!(described.status.success(), "{described:?}");
    assert!(read(&docs, "context/billing-rules.md").contains("description: x"));

    let retitled = set(
        &docs,
        &["context/billing-rules.md", "title", "Billing policy"],
    );
    assert!(retitled.status.success(), "{retitled:?}");
    assert!(!docs.join("context/billing-rules.md").exists());
    assert!(read(&docs, "context/billing-policy.md").contains("title: Billing policy"));
    let index = read(&docs, "context/index.md");
    assert!(
        index.contains("billing-policy.md") && !index.contains("billing-rules.md"),
        "{index}"
    );

    let refused = set(&docs, &["context/billing-policy.md", "status", "Accepted"]);
    assert_eq!(refused.status.code(), Some(2), "{refused:?}");
}

#[test]
fn a_domain_record_refuses_a_kind_and_accepts_any_token_case() {
    let docs = temp_bundle("domain-records", "token");
    let kind = new_domain(&docs, &["domain", "x", "--kind", "state"]);
    assert!(!kind.status.success(), "{kind:?}");
    assert!(!docs.join("context/x.md").exists());
    let cased = new_domain(&docs, &["Domain", "Y"]);
    assert!(cased.status.success(), "{cased:?}");
    assert!(docs.join("context/y.md").is_file());
}

const GLOSSARY: &str = "---\ntype: Context\ntitle: Glossary\ndescription: Terms.\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n# Glossary\n\n## Order\n\nA purchase.\n";
const VOCABULARY: &str = "---\ntype: Context\ntitle: Vocabulary\ndescription: Words.\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n# Vocabulary\n\n## Cart\n\nItems held.\n";

fn existing_bundle(label: &str) -> common::Scratch {
    let docs = temp_bundle("domain-records", label);
    write(&docs, "context/glossary.md", GLOSSARY);
    write(&docs, "context/vocabulary.md", VOCABULARY);
    let table = "# Context\n\n| Name | Note |\n|---|---|\n| glossary | terms |\n";
    write(&docs, "context/index.md", table);
    docs
}

#[test]
fn existing_hand_written_records_are_indexed_untouched() {
    let docs = existing_bundle("existing");
    let drift = stdout_of(&run_check(&docs));
    assert!(
        drift.contains("context") && drift.contains("regeneration"),
        "{drift}"
    );

    index_bundle(&docs);
    let checked = run_check(&docs);
    assert_eq!(checked.status.code(), Some(0), "{}", stdout_of(&checked));
    assert_eq!(read(&docs, "context/glossary.md"), GLOSSARY);
    assert_eq!(read(&docs, "context/vocabulary.md"), VOCABULARY);
    let index = read(&docs, "context/index.md");
    assert!(
        index.contains("(glossary.md)") && index.contains("(vocabulary.md)"),
        "{index}"
    );
}

#[test]
fn existing_record_with_a_status_key_draws_an_advisory_without_failing() {
    let docs = existing_bundle("existing-status");
    let with_status = GLOSSARY.replace("timestamp:", "status: current\ntimestamp:");
    write(&docs, "context/glossary.md", &with_status);
    index_bundle(&docs);
    let checked = run_check_advisories(&docs);
    let report = stdout_of(&checked);
    assert_eq!(checked.status.code(), Some(0), "{report}");
    assert!(report.contains("STATUS-VOCABULARY"), "{report}");
}

fn context_record(title: &str, extra: &str) -> String {
    format!(
        "---\ntype: Context\ntitle: {title}\ndescription: A record.\n{extra}timestamp: 2026-01-01T00:00:00Z\n---\n\n# {title}\n"
    )
}

#[test]
fn a_domain_index_lists_records_by_filename_and_shows_no_kind() {
    let docs = temp_bundle("domain-records", "index-order");
    write(
        &docs,
        "context/a.md",
        &context_record("Zed", "kind: context\n"),
    );
    write(&docs, "context/z.md", &context_record("Aye", ""));
    index_bundle(&docs);
    let index = read(&docs, "context/index.md");
    assert!(!index.contains("- context"), "{index}");
    let first = index.find("(a.md)").expect("a.md listed");
    let second = index.find("(z.md)").expect("z.md listed");
    assert!(first < second, "{index}");
}

#[test]
fn a_record_type_that_differs_from_the_context_directory_draws_an_advisory() {
    let docs = temp_bundle("domain-records", "record-type");
    let concept = context_record("Odd", "").replace("type: Context", "type: Concept");
    write(&docs, "context/odd.md", &concept);
    write(&docs, "context/fine.md", &context_record("Fine", ""));
    index_bundle(&docs);
    let checked = run_check_advisories(&docs);
    let report = stdout_of(&checked);
    assert_eq!(checked.status.code(), Some(0), "{report}");
    let message = "declares `type: Concept` in a directory of Context records";
    assert!(report.contains(message), "{report}");
    assert_eq!(report.matches("declares `type:").count(), 1, "{report}");
}

#[test]
fn an_unfilled_domain_description_names_the_set_repair() {
    let docs = temp_bundle("domain-records", "placeholder");
    let out = new_domain(&docs, &["domain", "Billing rules"]);
    assert!(out.status.success(), "{out:?}");
    let report = stdout_of(&run_check(&docs));
    assert!(
        report.contains("write set domain/billing-rules description"),
        "{report}"
    );
}

#[test]
fn the_reference_help_lists_the_context_directory() {
    let out = living_docs()
        .args(["write", "set", "--help"])
        .output()
        .unwrap();
    let help = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        help.contains("domain") && help.contains("context"),
        "{help}"
    );
}
