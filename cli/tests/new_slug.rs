//! `write new` refuses a title that slugifies to nothing, the same rule
//! `set title` applies, so no record is written under an empty file name.

mod common;
use common::living_docs;

fn new_in(bundle: &std::path::Path, args: &[&str]) -> std::process::Output {
    living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap(), "write", "new"])
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn a_title_with_no_alphanumerics_is_refused_for_every_slugged_type() {
    let bundle = common::temp_bundle("new-slug", "empty");
    for args in [
        &["view", "!!!", "--kind", "component"][..],
        &["issue", "!!!"][..],
        &["adr", ""][..],
    ] {
        let output = new_in(&bundle, args);

        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("has no slug"),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(!bundle.join("architecture/.md").exists());
    assert!(!bundle.join("issues/0001-.md").exists());
    assert!(!bundle.join("adr/0001-.md").exists());
}

#[test]
fn a_singleton_needs_no_slug_from_its_title() {
    let bundle = common::temp_bundle("new-slug", "singleton");

    let output = new_in(&bundle, &["constitution", "!!!"]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(bundle.join("constitution.md").exists());
}
