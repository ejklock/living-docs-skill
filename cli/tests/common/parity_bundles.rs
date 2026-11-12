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
        Bundle::Scaffolded => scaffold(docs, body),
        Bundle::StaleIndexes => {
            scaffold(docs, body);
            for index in ["adr/index.md", "architecture/index.md"] {
                fs::remove_file(docs.join(index)).unwrap();
            }
        }
        Bundle::NonCanonical => {
            scaffold(docs, body);
            let record = docs.join("adr/0001-cache-strategy.md");
            let text = fs::read_to_string(&record).unwrap();
            let moved = text.replace(
                "owner: o\nstatus: Proposed\n",
                "status: Proposed\nowner: o\n",
            );
            assert_ne!(text, moved, "the fixture record must start canonical");
            fs::write(record, moved).unwrap();
        }
    }
}
