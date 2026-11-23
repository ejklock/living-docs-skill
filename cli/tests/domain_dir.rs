//! `write new domain --dir`: a domain record placed in a subfolder of
//! `docs/context/`, with an unsafe folder refused before anything is written.

mod common;

use common::mcp_session::Session;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Output;

fn new_of(doc_type: &str, docs: &Path, extra: &[&str]) -> Output {
    common::living_docs()
        .args(["--docs-dir", docs.to_str().unwrap()])
        .args(["write", "new", doc_type])
        .args(extra)
        .output()
        .unwrap()
}

fn new_domain(docs: &Path, extra: &[&str]) -> Output {
    new_of("domain", docs, extra)
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn markdown_files(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(markdown_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "md") {
            found.push(path.display().to_string());
        }
    }
    found
}

#[test]
fn a_domain_record_lands_at_the_depth_its_dir_names() {
    let docs = common::temp_bundle("domain-dir", "depth");

    let deep = new_domain(&docs, &["Billing rules", "--dir", "billing/invoices"]);
    assert!(deep.status.success(), "{deep:?}");
    let record = fs::read_to_string(docs.join("context/billing/invoices/billing-rules.md"))
        .expect("record at the nested path");
    assert!(record.contains("type: Context"), "{record}");
    assert!(record.contains("# Billing rules"), "{record}");
    assert!(record.contains("## Terms"), "{record}");

    let shallow = new_domain(&docs, &["Billing rules", "--dir", "billing"]);
    assert!(shallow.status.success(), "{shallow:?}");
    assert!(docs.join("context/billing/billing-rules.md").is_file());

    let flat = new_domain(&docs, &["Billing rules"]);
    assert!(flat.status.success(), "{flat:?}");
    assert!(docs.join("context/billing-rules.md").is_file());
}

#[test]
fn a_second_record_of_the_same_title_and_dir_is_refused() {
    let docs = common::temp_bundle("domain-dir", "twice");
    let args = ["Billing rules", "--dir", "billing"];
    assert!(new_domain(&docs, &args).status.success());

    let again = new_domain(&docs, &args);

    assert_eq!(again.status.code(), Some(2), "{again:?}");
    assert!(stderr_of(&again).contains("already exists"), "{again:?}");
}

#[test]
fn the_mcp_write_tool_places_a_domain_record_in_its_dir() {
    let docs = common::temp_bundle("domain-dir", "mcp");
    let mut session = Session::start(&docs);
    session.initialize();

    let result = session.call(
        2,
        "write",
        &json!({"operation": "new", "type": "domain", "title": "X", "dir": "a/b"}),
    );

    assert!(docs.join("context/a/b/x.md").is_file(), "{result}");
}

#[test]
fn the_mcp_write_tool_refuses_an_unsafe_dir() {
    let docs = common::temp_bundle("domain-dir", "mcp-refuses");
    let mut session = Session::start(&docs);
    session.initialize();

    let result = session.call(
        2,
        "write",
        &json!({"operation": "new", "type": "domain", "title": "X", "dir": "../x"}),
    );

    assert_eq!(result["isError"], json!(true), "{result}");
    assert!(result.to_string().contains("'..'"), "{result}");
    assert!(markdown_files(&docs).is_empty());
}

#[test]
fn refuses_an_unsafe_dir_and_writes_nothing() {
    let long = format!("a/b/c/{}", "x".repeat(300));
    let cases: [(&str, &str); 10] = [
        ("/abs", "absolute path"),
        (&long, "255 bytes"),
        ("../x", "'..'"),
        ("a/../b", "'..'"),
        ("./a", "."),
        ("a//b", "empty"),
        ("a/", "empty"),
        ("a\\b", "a\\b"),
        ("Billing", "Billing"),
        ("a b", "a b"),
    ];
    for (dir, named) in cases {
        let docs = common::temp_bundle("domain-dir", "refuses");

        let out = new_domain(&docs, &["X", "--dir", dir]);

        assert_eq!(out.status.code(), Some(2), "{dir}: {out:?}");
        assert!(stderr_of(&out).contains(named), "{dir}: {out:?}");
        assert!(markdown_files(&docs).is_empty(), "{dir}: wrote a file");
        assert!(!docs.join("context").exists(), "{dir}: left a folder");
    }
}

#[test]
fn refuses_a_dir_on_a_type_that_is_not_nested() {
    let docs = common::temp_bundle("domain-dir", "refuses-type");

    let out = new_of("adr", &docs, &["X", "--dir", "a"]);

    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(
        stderr_of(&out).contains("applies only to nested"),
        "{out:?}"
    );
    assert!(markdown_files(&docs).is_empty());
}

#[cfg(unix)]
#[test]
fn refuses_a_dir_that_is_a_link_leaving_the_bundle() {
    let docs = common::temp_bundle("domain-dir", "refuses-link");
    let outside = common::Scratch::new("domain-dir-outside");
    fs::create_dir_all(docs.join("context")).unwrap();
    std::os::unix::fs::symlink(outside.root(), docs.join("context/out")).unwrap();

    let out = new_domain(&docs, &["X", "--dir", "out"]);

    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(stderr_of(&out).contains("outside the bundle"), "{out:?}");
    assert!(markdown_files(outside.root()).is_empty());
    assert_eq!(fs::read_dir(outside.root()).unwrap().count(), 0);
    assert_eq!(fs::read_dir(docs.join("context/out")).unwrap().count(), 0);
}
