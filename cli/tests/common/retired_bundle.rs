//! A bundle holding every retirement case `read records` must tell apart:
//! open and retired issues, accepted, deprecated and superseded decisions,
//! an unregistered type and a record without a status.

use crate::common::write;
use std::path::Path;

/// One record per row: path, type, title, frontmatter lines after the description.
const RECORDS: [(&str, &str, &str, &str); 10] = [
    ("issue/0001-open.md", "Issue", "Open one", "status: open\n"),
    (
        "issue/0002-shut.md",
        "Issue",
        "Zebrafish shut",
        "status: closed\n",
    ),
    ("issue/0003-fin.md", "Issue", "Finished", "status: done\n"),
    ("issue/0004-cap.md", "Issue", "Capital", "status: Closed\n"),
    (
        "adr/0001-acc.md",
        "ADR",
        "Accepted rule",
        "status: Accepted\n",
    ),
    (
        "adr/0002-dep.md",
        "ADR",
        "Dropped rule",
        "status: Deprecated\n",
    ),
    ("adr/0003-old.md", "ADR", "Old rule", "status: Superseded\n"),
    (
        "adr/0004-new.md",
        "ADR",
        "New rule",
        "status: Accepted\nsupersedes: \"0003\"\n",
    ),
    (
        "memo/0001-odd.md",
        "Memo",
        "Unregistered",
        "status: closed\n",
    ),
    ("adr/0005-bare.md", "ADR", "Statusless", ""),
];

/// Writes the ten records of the bundle.
pub fn write_bundle(bundle: &Path) {
    for (path, doc_type, title, extra) in RECORDS {
        write_record(bundle, path, doc_type, title, extra, "body");
    }
}

/// Adds an open and a closed issue that each link the accepted decision.
pub fn write_linkers(bundle: &Path) {
    let link = "[rule](/adr/0001-acc.md)";
    write_record(
        bundle,
        "issue/0006-linkopen.md",
        "Issue",
        "Open linker",
        "status: open\n",
        link,
    );
    write_record(
        bundle,
        "issue/0007-linkshut.md",
        "Issue",
        "Closed linker",
        "status: closed\n",
        link,
    );
}

fn write_record(bundle: &Path, path: &str, doc_type: &str, title: &str, extra: &str, body: &str) {
    let contents = format!(
        "---\ntype: {doc_type}\ntitle: {title}\ndescription: d\n{extra}---\n\n# {title}\n\n{body}\n"
    );
    write(bundle, path, &contents);
}
