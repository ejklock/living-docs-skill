mod common;

use common::{living_docs, run_new, temp_bundle};
use std::fs;

#[test]
fn items_the_parser_would_store_as_something_else_are_written_quoted() {
    let docs = temp_bundle("set-quoted-items", "quote-reinterpreted");
    for _ in 0..3 {
        assert!(run_new(&docs, "issue", "T").status.success());
    }
    let path = docs.join("issues/0001-t.md");
    for (key, value, line) in [
        ("labels", "1e3", "labels: [\"1e3\"]"),
        ("labels", "null, a", "labels: [\"null\", a]"),
        ("labels", "true, 1.5, x", "labels: [\"true\", \"1.5\", x]"),
        (
            "labels",
            "false, 2.5, .inf",
            "labels: [\"false\", \"2.5\", \".inf\"]",
        ),
        ("blocked_by", "~", "blocked_by: [\"~\"]"),
    ] {
        let out = living_docs()
            .args(["--docs-dir", docs.to_str().unwrap(), "write", "set"])
            .args(["0001", key, value, "--plain"])
            .output()
            .unwrap();
        assert!(out.status.success(), "{value}");
        let prefix = format!("{key}:");
        let written = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = written.lines().filter(|l| l.starts_with(&prefix)).collect();
        assert_eq!(lines, [line], "{value}");
    }
}
