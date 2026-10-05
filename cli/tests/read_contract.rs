//! End-to-end `read --contract`: the contract-tier body of each in-force record.

use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, temp_bundle, write};

fn read(docs_dir: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["--docs-dir", docs_dir.to_str().unwrap(), "read"];
    args.extend_from_slice(extra);
    living_docs()
        .args(&args)
        .output()
        .expect("failed to run read")
}

const FRONT: &str = "---\ntype: Issue\ntitle: Widget\ndescription: d\nstatus: open\n---\n\n";

fn write_issue(bundle: &Path, body: &str) {
    write(bundle, "issues/0001-widget.md", &format!("{FRONT}{body}"));
}

fn contract_text(bundle: &Path) -> String {
    stdout_of(&read(bundle, &["--contract", "--plain"]))
}

const FULL_BODY: &str = "## Widget\n\nThe preamble.\n\n### Scope\n\nscope text\n\n### Acceptance\n\naccept text\n\n### Plan\n\nplan text\n\n#### Step\n\nstep text\n\n### Context manifest\n\nmanifest text\n\n### Outcome\n\noutcome text\n";

#[test]
fn detail_sections_are_dropped_and_everything_else_is_kept() {
    let dir = temp_bundle("read-contract", "contract-basic");
    write_issue(&dir, FULL_BODY);
    let out = contract_text(&dir);
    for kept in [
        "## Widget",
        "The preamble.",
        "### Scope",
        "scope text",
        "### Acceptance",
        "accept text",
        "### Context manifest",
        "manifest text",
    ] {
        assert!(out.contains(kept), "missing {kept:?} in {out}");
    }
    for dropped in [
        "Plan",
        "plan text",
        "Step",
        "step text",
        "Outcome",
        "outcome text",
    ] {
        assert!(!out.contains(dropped), "found {dropped:?} in {out}");
    }
}

#[test]
fn a_trailing_plan_runs_to_the_end_of_the_body() {
    let dir = temp_bundle("read-contract", "contract-trailing");
    write_issue(&dir, "## W\n\n### Scope\n\ns\n\n### Plan\n\nlast words\n");
    let out = contract_text(&dir);
    assert!(
        out.contains("### Scope") && !out.contains("last words"),
        "{out}"
    );
}

#[test]
fn a_contract_section_after_plan_is_kept() {
    let dir = temp_bundle("read-contract", "contract-after");
    write_issue(
        &dir,
        "## W\n\n### Plan\n\nhidden\n\n### Acceptance\n\nshown\n",
    );
    let out = contract_text(&dir);
    assert!(out.contains("shown") && !out.contains("hidden"), "{out}");
}

#[test]
fn a_fenced_plan_heading_does_not_cut_scope() {
    let dir = temp_bundle("read-contract", "contract-fence");
    write_issue(
        &dir,
        "## W\n\n### Scope\n\n```\n### Plan\n```\n\nafter fence\n",
    );
    assert!(contract_text(&dir).contains("after fence"));
}

#[test]
fn heading_case_does_not_save_a_detail_section() {
    let dir = temp_bundle("read-contract", "contract-case");
    write_issue(&dir, "## W\n\n### Scope\n\ns\n\n### PLAN\n\nloud plan\n");
    assert!(!contract_text(&dir).contains("loud plan"));
}

#[test]
fn a_record_without_a_title_heading_keeps_its_first_section() {
    let dir = temp_bundle("read-contract", "contract-untitled");
    write_issue(&dir, "### Scope\n\nfirst section\n\n### Plan\n\nhidden\n");
    let out = contract_text(&dir);
    assert!(
        out.contains("first section") && !out.contains("hidden"),
        "{out}"
    );
}

#[test]
fn json_carries_the_filtered_body_and_an_adr_matches_full() {
    let dir = temp_bundle("read-contract", "contract-json");
    write_issue(&dir, FULL_BODY);
    write(
        &dir,
        "adr/0001-a.md",
        "---\ntype: ADR\ntitle: A\ndescription: d\nstatus: Accepted\n---\n\n# 0001. A\n\n### Plan\n\nadr plan\n",
    );
    let contract = json_bodies(&dir, "--contract");
    let full = json_bodies(&dir, "--full");
    assert_eq!(contract[0], full[0], "ADR body must be unchanged");
    assert!(contract[1].contains("scope text") && !contract[1].contains("plan text"));
}

fn json_bodies(dir: &Path, flag: &str) -> Vec<String> {
    let out = stdout_of(&read(dir, &[flag, "--json"]));
    let value: serde_json::Value = serde_json::from_str(&out).expect("json");
    value["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["body"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn contract_with_full_is_a_usage_error() {
    let dir = temp_bundle("read-contract", "contract-conflict");
    write_issue(&dir, FULL_BODY);
    let out = read(&dir, &["--contract", "--full"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn topic_filters_the_same_records_with_or_without_contract() {
    let dir = temp_bundle("read-contract", "contract-topic");
    write_issue(
        &dir,
        "## W\n\n### Scope\n\nzebra\n\n### Plan\n\nonly in plan\n",
    );
    write(
        &dir,
        "adr/0001-a.md",
        "---\ntype: ADR\ntitle: A\ndescription: d\nstatus: Accepted\n---\n\n# 0001. A\n\nunrelated\n",
    );
    let titles = |flags: &[&str]| -> Vec<String> {
        let out = stdout_of(&read(
            &dir,
            &[&["--topic", "only in plan", "--json"], flags].concat(),
        ));
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        v["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["title"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(titles(&["--contract"]), titles(&[]));
    assert_eq!(titles(&[]), ["Widget"]);
}
