//! The fixture bundles each parity case starts from, and the body text its
//! writes carry.

use crate::common::{self, copy_tree, living_docs};
use std::fs;
use std::path::Path;

pub const BODY: &str =
    "Why this matters, in prose.\n\nSee [Cache strategy](../adr/0001-cache-strategy.md).\n";

#[derive(Clone, Copy)]
pub enum Bundle {
    Scaffolded,
    NonCanonical,
    Canonical,
    StaleIndexes,
    ReadOnlyIndex,
    Nested,
}

impl Bundle {
    /// Picks the bundle a row's flags name, `Scaffolded` when none is named.
    pub fn from_flags(flags: &[&str]) -> Bundle {
        [
            ("canonical", Bundle::Canonical),
            ("noncanonical", Bundle::NonCanonical),
            ("stale", Bundle::StaleIndexes),
            ("readonly", Bundle::ReadOnlyIndex),
            ("nested", Bundle::Nested),
        ]
        .into_iter()
        .find(|(flag, _)| flags.contains(flag))
        .map_or(Bundle::Scaffolded, |(_, bundle)| bundle)
    }
}

/// The records every scaffolded bundle holds, one `|`-separated CLI call per row.
const SCAFFOLD: [&str; 4] = [
    "write | new | adr | Cache strategy | --owner | o | --description | first",
    "write | new | adr | Cache write-through | --owner | o | --description | second",
    "write | new | issue | Flaky upload | --description | linked | --body | {body}",
    "write | new | view | Rings | --kind | component | --description | rings",
];

fn scaffold(docs: &Path, body: &Path) {
    for row in SCAFFOLD {
        let args = row.split(" | ").map(|word| {
            if word == "{body}" {
                body.to_str().unwrap()
            } else {
                word
            }
        });
        let out = living_docs()
            .args(["--docs-dir", docs.to_str().unwrap()])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "{row}: {out:?}");
    }
}

pub fn make_bundle(kind: Bundle, docs: &Path, body: &Path) {
    match kind {
        Bundle::Canonical => copy_tree(&common::fixture("09-okf-canonical"), docs),
        Bundle::Scaffolded | Bundle::ReadOnlyIndex => scaffold(docs, body),
        Bundle::StaleIndexes => {
            scaffold(docs, body);
            for index in ["adr/index.md", "architecture/index.md"] {
                fs::remove_file(docs.join(index)).unwrap();
            }
        }
        Bundle::Nested => {
            scaffold(docs, body);
            rewrite_first_record(
                docs,
                "## Decision\n",
                "### Detail\n\nkept\n\n## Decision\n",
                "the fixture record must carry a Decision section",
            );
        }
        Bundle::NonCanonical => {
            scaffold(docs, body);
            rewrite_first_record(
                docs,
                "owner: o\nstatus: Proposed\n",
                "status: Proposed\nowner: o\n",
                "the fixture record must start canonical",
            );
        }
    }
}

fn rewrite_first_record(docs: &Path, from: &str, to: &str, expectation: &str) {
    let record = docs.join("adr/0001-cache-strategy.md");
    let text = fs::read_to_string(&record).unwrap();
    let rewritten = text.replace(from, to);
    assert_ne!(text, rewritten, "{expectation}");
    fs::write(record, rewritten).unwrap();
}

/// Makes the decision index of each bundle copy read-only or writable again,
/// so a refresh of it fails while the record write itself succeeds.
#[cfg(unix)]
pub fn set_index_writable(copies: &[&Path], writable: bool) {
    use std::os::unix::fs::PermissionsExt;
    let mode = if writable { 0o644 } else { 0o444 };
    for docs in copies {
        fs::set_permissions(docs.join("adr/index.md"), fs::Permissions::from_mode(mode)).unwrap();
    }
}

#[cfg(not(unix))]
pub fn set_index_writable(_copies: &[&Path], _writable: bool) {}
