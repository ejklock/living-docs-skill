//! End-to-end wiring for the `read` verb: arg parsing, the `--docs-dir`
//! bundle, the compiled output reaching stdout and the empty-result hint
//! reaching stderr.

use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, write};

fn read(docs_dir: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["--docs-dir", docs_dir.to_str().unwrap(), "read", "--plain"];
    args.extend_from_slice(extra);
    living_docs()
        .args(&args)
        .output()
        .expect("failed to run living-docs read")
}

fn write_chain_bundle(bundle: &Path) {
    write(
        bundle,
        "adr/0001-old.md",
        "---\ntype: ADR\ntitle: Old\ndescription: d\nstatus: Superseded\nsuperseded_by: \"0002\"\n---\n\n# 0001. Old\n\nabout widgets\n",
    );
    write(
        bundle,
        "adr/0002-new.md",
        "---\ntype: ADR\ntitle: New\ndescription: the widget rule in force\nstatus: Accepted\nsupersedes: \"0001\"\n---\n\n# 0002. New\n\nabout widgets\n",
    );
}

#[test]
fn read_topic_returns_the_head_only_with_a_lineage_line() {
    let bundle = common::temp_bundle("read", "chain");
    write_chain_bundle(&bundle);

    let stdout = stdout_of(&read(&bundle, &["--topic", "widget"]));

    assert!(stdout.contains("[ADR 0002]"), "head present:\n{stdout}");
    assert!(
        stdout.contains("supersedes 0001"),
        "lineage present:\n{stdout}"
    );
    assert!(
        !stdout.contains("[ADR 0001]"),
        "superseded record absent:\n{stdout}"
    );
}

#[test]
fn empty_topic_result_keeps_json_shape_and_explains_itself_on_stderr() {
    let bundle = common::temp_bundle("read", "miss-json");
    write_chain_bundle(&bundle);
    let dir = bundle.to_str().unwrap();

    let output = living_docs()
        .args(["--docs-dir", dir, "read", "--json", "--topic", "zzqq"])
        .output()
        .expect("failed to run living-docs read");

    assert!(output.status.success());
    assert_eq!(stdout_of(&output), "{\"withheld\":1,\"records\":[]}");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "living-docs read: no live record mentions 'zzqq' (1 retired withheld); try a broader term or `grep -ril 'zzqq' {dir}`\n"
        )
    );
}

#[test]
fn empty_topic_result_in_plain_mode_prints_the_same_hint() {
    let bundle = common::temp_bundle("read", "miss-plain");
    write_chain_bundle(&bundle);

    let output = read(&bundle, &["--topic", "zzqq"]);

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no live record mentions 'zzqq' (1 retired withheld)"));
    assert_eq!(stderr.lines().count(), 1, "one hint line:\n{stderr}");
}

#[test]
fn a_matching_topic_leaves_stderr_empty() {
    let bundle = common::temp_bundle("read", "hit");
    write_chain_bundle(&bundle);

    let output = read(&bundle, &["--topic", "widget"]);

    assert!(output.stderr.is_empty(), "no hint on a match");
}

#[test]
fn quiet_silences_the_empty_result_hint() {
    let bundle = common::temp_bundle("read", "quiet");
    write_chain_bundle(&bundle);
    let dir = bundle.to_str().unwrap();

    let output = living_docs()
        .args(["--docs-dir", dir, "--quiet", "read", "--plain"])
        .args(["--topic", "zzqq"])
        .output()
        .expect("failed to run living-docs read");

    assert!(output.status.success());
    assert!(output.stderr.is_empty(), "quiet hides the hint");
}

#[test]
fn retired_effective_command_is_refused_as_unknown_usage() {
    let output = living_docs()
        .args(["effective", "--plain"])
        .output()
        .expect("failed to run living-docs effective");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unrecognized subcommand 'effective'"),
        "got:\n{stderr}"
    );
}
