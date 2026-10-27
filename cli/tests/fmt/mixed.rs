use super::*;

const PROSE: &str = "# Notes\r\n\r\nplain prose\nwithout frontmatter\r\n";

#[test]
fn fmt_leaves_mixed_files_it_never_rewrites_untouched() {
    let bundle = temp_bundle("mixed-unrewritten");
    let index =
        "---\r\nokf_version: 0.1\ntitle: ADRs\r\n---\r\n# Index\r\n\r\n- [Doc](0001-doc.md)\n";
    let log = "---\nokf_version: 0.1\r\n---\r\n# Log\r\n\r\nentry\n";
    write(&bundle, "index.md", "# Index\n");
    write(&bundle, "adr/index.md", index);
    write(&bundle, "adr/log.md", log);
    write(&bundle, "adr/0001-doc.md", CANONICAL_RECORD);
    write(&bundle, "notes.md", PROSE);

    assert_eq!(run_fmt_check(&bundle).status.code(), Some(0));
    let output = run_fmt(&bundle);
    assert_eq!(output.status.code(), Some(0), "{output:?}");

    assert_eq!(read(&bundle, "adr/index.md"), index);
    assert_eq!(read(&bundle, "adr/log.md"), log);
    assert_eq!(read(&bundle, "notes.md"), PROSE);
    assert_eq!(read(&bundle, "adr/0001-doc.md"), CANONICAL_RECORD);
}

#[test]
fn fmt_still_refuses_a_mixed_record_with_frontmatter_beside_mixed_prose() {
    let bundle = temp_bundle("mixed-record");
    let record = NON_CANONICAL_RECORD.replacen('\n', "\r\n", 1);
    write(&bundle, "index.md", "# Index\n");
    write(&bundle, "adr/0001-doc.md", &record);
    write(&bundle, "notes.md", PROSE);

    for output in [run_fmt_check(&bundle), run_fmt(&bundle)] {
        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(stderr.contains("mixed line endings"), "got: {stderr}");
        assert!(!stderr.contains("notes.md"), "got: {stderr}");
    }
    assert_eq!(read(&bundle, "adr/0001-doc.md"), record);
}
