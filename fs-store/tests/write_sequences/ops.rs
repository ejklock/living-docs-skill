//! The write operations a sequence is built from, each calling the same
//! public function the command line front calls.

use fs_store::FsStore;
use living_docs_core::commands::fmt::{self, Seeds};
use living_docs_core::commands::new::{self, NewOptions};
use living_docs_core::commands::{index, set, supersede};
use std::path::Path;

const FIRST_ADR: &str = "adr/0001";
const SECOND_ADR: &str = "adr/0002";
const FIRST_ISSUE: &str = "issue/0001";
const ADR_BODY: &str = "## Context\n\nThe cache grows without a bound.\n\n## Decision\n\nWe evict the oldest entry first.\n\n## Consequences\n\nMemory stays flat under load.\n";
const ISSUE_BODY: &str =
    "## Scope\n\nBound the cache.\n\n## Acceptance\n\nMemory stays flat under load.\n";

/// One write operation over a bundle.
#[derive(Debug, Clone, Copy)]
pub enum Op {
    NewAdr,
    NewIssue,
    AdrStatus {
        number: u8,
        value: &'static str,
    },
    IssueStatus(&'static str),
    Supersede {
        old: &'static str,
        new: &'static str,
    },
    TitleFirstAdr,
    NumberNextFirstAdr,
    ReplaceContext,
    Fmt,
    Index,
}

/// Every operation a sequence step may pick.
pub fn alphabet() -> Vec<Op> {
    let mut ops = vec![Op::NewAdr, Op::NewIssue];
    for number in [1, 2] {
        for value in ["Proposed", "Accepted", "Deprecated"] {
            ops.push(Op::AdrStatus { number, value });
        }
    }
    ops.push(Op::IssueStatus("closed"));
    for (old, new) in [
        (FIRST_ADR, SECOND_ADR),
        (SECOND_ADR, FIRST_ADR),
        (FIRST_ADR, FIRST_ISSUE),
    ] {
        ops.push(Op::Supersede { old, new });
    }
    ops.extend([
        Op::TitleFirstAdr,
        Op::NumberNextFirstAdr,
        Op::ReplaceContext,
        Op::Fmt,
        Op::Index,
    ]);
    ops
}

/// Runs `op` over the bundle at `bundle` as the `step`-th of its sequence;
/// `Err` is a refused step.
pub fn apply(op: Op, bundle: &Path, step: usize) -> Result<(), String> {
    let store = FsStore::rooted(bundle);
    match op {
        Op::NewAdr => create("adr", ADR_BODY, step, &store, bundle),
        Op::NewIssue => create("issue", ISSUE_BODY, step, &store, bundle),
        Op::AdrStatus { number, value } => {
            let reference = format!("adr/{number:04}");
            set_key(&store, bundle, &reference, "status", value)
        }
        Op::IssueStatus(value) => set_key(&store, bundle, FIRST_ISSUE, "status", value),
        Op::Supersede { old, new } => supersede::supersede(&store, bundle, old, new).map(drop),
        Op::TitleFirstAdr | Op::NumberNextFirstAdr | Op::ReplaceContext => {
            edit_first_adr(op, &store, bundle)
        }
        Op::Fmt => format_bundle(&store, bundle),
        Op::Index => index::write(&store, bundle, None).map(drop),
    }
}

fn edit_first_adr(op: Op, store: &FsStore, bundle: &Path) -> Result<(), String> {
    match op {
        Op::TitleFirstAdr => set_key(store, bundle, FIRST_ADR, "title", "Renamed decision"),
        Op::NumberNextFirstAdr => set_key(store, bundle, FIRST_ADR, "number", "next"),
        _ => {
            let text = "The context was rewritten.\n";
            set::apply_section(store, bundle, FIRST_ADR, "Context", text).map(drop)
        }
    }
}

fn create(
    doc_type: &str,
    body: &str,
    step: usize,
    store: &FsStore,
    bundle: &Path,
) -> Result<(), String> {
    let title = format!("Decision made at step {step}");
    let opts = NewOptions {
        description: Some("Bounds the cache with oldest-first eviction."),
        owner: Some("tester"),
        body: Some(body),
        ..NewOptions::default()
    };
    new::write(store, bundle, doc_type, &title, &opts).map(drop)
}

fn set_key(
    store: &FsStore,
    bundle: &Path,
    reference: &str,
    key: &str,
    value: &str,
) -> Result<(), String> {
    set::apply(store, bundle, reference, key, value).map(drop)
}

/// Runs `fmt` over the whole bundle the way the front does, with an owner
/// seed and no git-derived timestamp.
pub fn format_bundle(store: &FsStore, bundle: &Path) -> Result<(), String> {
    let seeds = Seeds {
        owner: Some("tester".to_string()),
        added_at: None,
    };
    fmt::outcome(store, bundle, false, &seeds).map(drop)
}
