//! A record whose filename holds a space is linked in the angle-bracket
//! form, so `check` resolves every generated link.

mod common;

use common::{index_bundle, run_check, run_supersede, stdout_of, temp_bundle, write};
use std::fs;

const BODY: &str = "## Context\n\nWhy.\n\n## Decision\n\nWhat.\n\n## Consequences\n\nSo.\n\n## Verification\n\n- Checked.\n";

fn adr(number: &str, status: &str, extra: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: Decision {number}\ndescription: A record\nowner: Someone\nstatus: {status}\n{extra}timestamp: 2026-01-01T00:00:00Z\n---\n\n# {number}. Decision {number}\n\n{BODY}"
    )
}

#[test]
fn an_index_row_for_a_spaced_filename_is_angle_form_and_check_passes() {
    let bundle = temp_bundle("index-spaced", "row");
    write(&bundle, "index.md", "# Docs\n\n* [adr](adr/)\n");
    write(&bundle, "adr/0001-a b.md", &adr("0001", "Accepted", ""));
    index_bundle(&bundle);
    let index = fs::read_to_string(bundle.join("adr/index.md")).unwrap();
    assert!(index.contains("](<0001-a b.md>)"), "{index}");
    let output = run_check(&bundle);
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
    index_bundle(&bundle);
    assert_eq!(
        fs::read_to_string(bundle.join("adr/index.md")).unwrap(),
        index
    );
}

#[test]
fn a_spaced_successor_is_linked_in_angle_form_by_index_and_callout() {
    let bundle = temp_bundle("index-spaced", "successor");
    write(&bundle, "index.md", "# Docs\n\n* [adr](adr/)\n");
    write(&bundle, "adr/0001-old.md", &adr("0001", "Accepted", ""));
    write(&bundle, "adr/0002-n w.md", &adr("0002", "Accepted", ""));
    let superseded = run_supersede(&bundle, "0001", "0002");
    assert!(superseded.status.success(), "{}", stdout_of(&superseded));
    index_bundle(&bundle);
    let index = fs::read_to_string(bundle.join("adr/index.md")).unwrap();
    assert!(
        index.contains("Superseded by [0002](<0002-n w.md>)"),
        "{index}"
    );
    let old = fs::read_to_string(bundle.join("adr/0001-old.md")).unwrap();
    assert!(old.contains("[0002](<0002-n w.md>)"), "{old}");
    let output = run_check(&bundle);
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}

#[test]
fn a_named_view_with_a_space_is_indexed_in_angle_form_and_check_passes() {
    let bundle = temp_bundle("index-spaced", "view");
    write(
        &bundle,
        "index.md",
        "# Docs\n\n* [architecture](architecture/)\n",
    );
    let view = "---\ntype: Architecture View\ntitle: Ctx\ndescription: d.\nkind: context\n---\n\n# Ctx\n\nProse.\n";
    write(&bundle, "architecture/ctx view.md", view);
    index_bundle(&bundle);
    let index = fs::read_to_string(bundle.join("architecture/index.md")).unwrap();
    assert!(index.contains("](<ctx view.md>)"), "{index}");
    let output = run_check(&bundle);
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}
