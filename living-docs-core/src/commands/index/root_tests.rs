use super::root::{ensure_root_index, NoTypeIndex};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn bundle_with_type_indexes(label: &str, dirs: &[&str]) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let docs = std::env::temp_dir().join(format!("living-docs-root-{label}-{nanos}"));
    for dir in dirs {
        fs::create_dir_all(docs.join(dir)).unwrap();
        fs::write(docs.join(dir).join("index.md"), "# T\n\n").unwrap();
    }
    fs::create_dir_all(&docs).unwrap();
    docs
}

#[test]
fn missing_root_is_created_with_okf_frontmatter_and_a_link_per_present_type_index() {
    let docs = bundle_with_type_indexes("create", &["adr", "issues"]);

    let written = ensure_root_index(&docs, &["adr"], NoTypeIndex::WriteNothing).unwrap();

    assert_eq!(written, Some(docs.join("index.md")));
    let root = fs::read_to_string(docs.join("index.md")).unwrap();
    assert!(
        root.starts_with("---\nokf_version: \"0.1\"\n---\n"),
        "{root}"
    );
    assert!(root.contains("* [ADRs](adr/)\n"), "{root}");
    assert!(root.contains("* [Issues](issues/)\n"), "{root}");
    assert!(!root.contains("](prd/)"), "{root}");
    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn missing_root_with_no_type_index_is_not_created() {
    let docs = bundle_with_type_indexes("none", &[]);

    assert_eq!(
        ensure_root_index(&docs, &["adr"], NoTypeIndex::WriteNothing).unwrap(),
        None
    );
    assert!(!docs.join("index.md").exists());
    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn link_less_root_is_created_only_when_asked() {
    let docs = bundle_with_type_indexes("linkless", &[]);

    let written = ensure_root_index(&docs, &[], NoTypeIndex::WriteLinkless).unwrap();

    assert_eq!(written, Some(docs.join("index.md")));
    let root = fs::read_to_string(docs.join("index.md")).unwrap();
    assert!(
        root.starts_with("---\nokf_version: \"0.1\"\n---\n"),
        "{root}"
    );
    assert!(!root.contains("]("), "{root}");
    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn crlf_root_ending_in_a_newline_gets_the_line_with_no_blank_line_inserted() {
    let docs = bundle_with_type_indexes("crlf-final", &["adr", "issues"]);
    fs::write(docs.join("index.md"), "# Mine\r\n\r\n* [ADRs](adr/)\r\n").unwrap();

    ensure_root_index(&docs, &["adr", "issue"], NoTypeIndex::WriteNothing).unwrap();

    let root = fs::read_to_string(docs.join("index.md")).unwrap();
    assert_eq!(
        root,
        "# Mine\r\n\r\n* [ADRs](adr/)\r\n* [Issues](issues/)\r\n"
    );
    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn mixed_ending_root_without_a_final_newline_gets_a_crlf_separator_and_line() {
    let docs = bundle_with_type_indexes("mixed", &["adr"]);
    fs::write(
        docs.join("index.md"),
        "---\nokf_version: \"0.1\"\n---\n\r\nbody",
    )
    .unwrap();

    ensure_root_index(&docs, &["adr"], NoTypeIndex::WriteNothing).unwrap();

    let root = fs::read_to_string(docs.join("index.md")).unwrap();
    assert_eq!(
        root,
        "---\nokf_version: \"0.1\"\n---\n\r\nbody\r\n* [ADRs](adr/)\r\n"
    );
    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn crlf_root_gets_its_appended_line_with_a_crlf_ending() {
    let docs = bundle_with_type_indexes("crlf", &["adr", "issues"]);
    fs::write(docs.join("index.md"), "# Mine\r\n\r\n* [ADRs](adr/)").unwrap();

    ensure_root_index(&docs, &["adr", "issue"], NoTypeIndex::WriteNothing).unwrap();

    let root = fs::read_to_string(docs.join("index.md")).unwrap();
    assert_eq!(
        root,
        "# Mine\r\n\r\n* [ADRs](adr/)\r\n* [Issues](issues/)\r\n"
    );
    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn present_root_gets_a_newline_then_one_line_for_each_unreached_touched_index() {
    let docs = bundle_with_type_indexes("append", &["adr", "issues"]);
    fs::write(docs.join("index.md"), "# Mine\n\n* [ADRs](adr/)").unwrap();

    ensure_root_index(&docs, &["adr", "issue"], NoTypeIndex::WriteNothing).unwrap();

    let root = fs::read_to_string(docs.join("index.md")).unwrap();
    assert_eq!(root, "# Mine\n\n* [ADRs](adr/)\n* [Issues](issues/)\n");
    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn present_root_reaching_an_index_through_another_index_is_left_alone() {
    let docs = bundle_with_type_indexes("transitive", &["adr", "issues"]);
    fs::write(docs.join("index.md"), "* [Work](issues/)\n").unwrap();
    fs::write(docs.join("issues/index.md"), "* [ADRs](../adr/index.md)\n").unwrap();

    assert_eq!(
        ensure_root_index(&docs, &["adr", "issue"], NoTypeIndex::WriteNothing).unwrap(),
        None
    );
    assert_eq!(
        fs::read_to_string(docs.join("index.md")).unwrap(),
        "* [Work](issues/)\n"
    );
    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn root_that_is_a_directory_is_an_error_naming_it() {
    let docs = bundle_with_type_indexes("dir", &["adr"]);
    fs::create_dir_all(docs.join("index.md")).unwrap();

    let error = ensure_root_index(&docs, &["adr"], NoTypeIndex::WriteNothing).unwrap_err();

    assert!(error.contains("index.md"), "{error}");
    assert!(docs.join("index.md").is_dir());
    let _ = fs::remove_dir_all(&docs);
}
