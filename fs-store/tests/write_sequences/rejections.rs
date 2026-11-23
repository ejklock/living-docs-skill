//! Cases proving each part of the contract can fail: a bundle broken in
//! exactly one way is rejected, naming the part.

use crate::contract::{self, Breach, Part};
use crate::ops::{self, Op};
use crate::snapshot;
use crate::Scratch;
use living_docs_core::commands::index;
use std::fs;
use std::path::{Path, PathBuf};

fn built(scratch: &Scratch, steps: &[Op]) -> PathBuf {
    let bundle = scratch.bundle("built");
    for (index, op) in steps.iter().enumerate() {
        ops::apply(*op, &bundle, index + 1).expect("setup step is accepted");
    }
    bundle
}

fn rewrite(path: &Path, edit: impl Fn(&str) -> String) {
    let text = fs::read_to_string(path).expect("read record");
    fs::write(path, edit(&text)).expect("write record");
}

fn two_adrs_one_superseding_the_other(scratch: &Scratch) -> PathBuf {
    let steps = [
        Op::NewAdr,
        Op::NewAdr,
        Op::Supersede {
            old: "adr/0001",
            new: "adr/0002",
        },
    ];
    built(scratch, &steps)
}

fn first_adr(bundle: &Path) -> PathBuf {
    let mut found: Vec<PathBuf> = fs::read_dir(bundle.join("adr"))
        .expect("list adr directory")
        .map(|entry| entry.expect("read entry").path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("0001"))
        })
        .collect();
    found.pop().expect("the first adr exists")
}

fn judged(bundle: &Path, refused: bool) -> Result<(), Breach> {
    let before = snapshot::take(bundle);
    contract::verify(bundle, &before, refused)
}

fn reindexed(bundle: &Path) {
    let store = fs_store::FsStore::rooted(bundle);
    index::write(&store, bundle, None).expect("regenerate the indexes");
}

fn part_broken(result: Result<(), Breach>) -> Part {
    result
        .expect_err("the contract checker must reject this bundle")
        .part
}

#[test]
fn contract_rejects_an_accepted_record_that_still_names_a_successor() {
    let scratch = Scratch::new("stale");
    let bundle = two_adrs_one_superseding_the_other(&scratch);
    assert!(judged(&bundle, false).is_ok());
    let old = first_adr(&bundle);
    rewrite(&old, |text| {
        let without_callout: Vec<&str> = text
            .lines()
            .filter(|line| !line.starts_with("> **SUPERSEDED"))
            .collect();
        without_callout
            .join("\n")
            .replace("status: Superseded", "status: Accepted")
            + "\n"
    });
    reindexed(&bundle);
    assert_eq!(part_broken(judged(&bundle, false)), Part::StaleSuccessor);
}

#[test]
fn contract_rejects_a_successor_link_pointing_at_a_record_of_another_type() {
    let scratch = Scratch::new("cross-type");
    let steps = [Op::NewAdr, Op::NewAdr, Op::NewIssue];
    let bundle = built(&scratch, &steps);
    let adr = first_adr(&bundle);
    rewrite(&adr, |text| {
        text.replacen(
            "status: Proposed",
            "status: Superseded\nsuperseded_by: 0001",
            1,
        )
    });
    reindexed(&bundle);
    let part = part_broken(judged(&bundle, false));
    assert_eq!(part, Part::UnmirroredSupersedeLink);
}

#[test]
fn contract_rejects_an_index_that_lost_a_row() {
    let scratch = Scratch::new("index-row");
    let bundle = built(&scratch, &[Op::NewAdr, Op::NewAdr]);
    let index = bundle.join("adr/index.md");
    rewrite(&index, |text| {
        let kept: Vec<&str> = text.lines().filter(|line| !line.contains("0002")).collect();
        kept.join("\n") + "\n"
    });
    assert_eq!(
        part_broken(judged(&bundle, false)),
        Part::RecordNotListedInIndex
    );
}

#[test]
fn contract_rejects_a_refused_step_that_changed_a_byte() {
    let scratch = Scratch::new("refused-byte");
    let bundle = built(&scratch, &[Op::NewAdr]);
    let before = snapshot::take(&bundle);
    let adr = first_adr(&bundle);
    rewrite(&adr, |text| format!("{text} "));
    let breach = contract::verify(&bundle, &before, true).expect_err("a changed byte is rejected");
    assert_eq!(breach.part, Part::RefusedStepChangedFile);
    assert!(breach.detail.contains("0001"), "{}", breach.detail);
}

fn bundle_with_record_in_force_beside_its_successor(scratch: &Scratch) -> PathBuf {
    let bundle = two_adrs_one_superseding_the_other(scratch);
    rewrite(&first_adr(&bundle), |text| {
        text.replace("status: Superseded", "status: Accepted")
    });
    bundle
}

fn bundle_with_frontmatter_fmt_would_rewrite(scratch: &Scratch) -> PathBuf {
    let bundle = built(scratch, &[Op::NewAdr]);
    rewrite(&first_adr(&bundle), |text| {
        text.replacen("title: ", "title:   ", 1)
    });
    bundle
}

#[test]
fn contract_rejects_a_record_in_force_together_with_its_successor() {
    let scratch = Scratch::new("in-force");
    let healthy = two_adrs_one_superseding_the_other(&scratch);
    assert!(judged(&healthy, false).is_ok());
    let other = Scratch::new("in-force-broken");
    let bundle = bundle_with_record_in_force_beside_its_successor(&other);
    let breach = contract::judge_from(&bundle, contract::IN_FORCE_WITHOUT_SUCCESSOR)
        .expect_err("a record in force with its successor is rejected");
    assert_eq!(breach.part, Part::InForceWithItsSuccessor);
    assert!(breach.detail.contains("0001"), "{}", breach.detail);
    assert!(breach.detail.contains("0002"), "{}", breach.detail);
}

#[test]
fn contract_rejects_frontmatter_that_fmt_would_rewrite() {
    let scratch = Scratch::new("not-canonical");
    let bundle = bundle_with_frontmatter_fmt_would_rewrite(&scratch);
    let breach = contract::judge_from(&bundle, contract::CANONICAL_AFTER_FMT_AND_INDEX)
        .expect_err("frontmatter fmt rewrites is rejected");
    assert_eq!(breach.part, Part::FmtThenIndexChangedBytes);
    assert!(breach.detail.contains("0001"), "{}", breach.detail);
}

#[test]
fn contract_rejects_a_refused_step_that_left_a_new_directory() {
    let scratch = Scratch::new("refused-directory");
    let bundle = built(&scratch, &[Op::NewAdr]);
    let before = snapshot::take(&bundle);
    fs::create_dir(bundle.join("leftover")).expect("create directory");
    let breach = contract::verify(&bundle, &before, true).expect_err("a new directory is rejected");
    assert_eq!(breach.part, Part::RefusedStepChangedFile);
    assert!(breach.detail.contains("leftover"), "{}", breach.detail);
}

#[test]
fn the_main_path_runs_every_state_judgment_in_order() {
    let scratch = Scratch::new("main-path");
    let bundle = two_adrs_one_superseding_the_other(&scratch);
    let ran = contract::judge_state(&bundle).expect("a healthy bundle keeps the contract");
    assert_eq!(
        ran,
        [
            "check_report",
            "in_force_without_successor",
            "successor_chains_are_acyclic",
            "canonical_after_fmt_and_index"
        ]
    );
}

fn second_adr(bundle: &Path) -> PathBuf {
    let mut found: Vec<PathBuf> = fs::read_dir(bundle.join("adr"))
        .expect("list adr directory")
        .map(|entry| entry.expect("read entry").path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("0002"))
        })
        .collect();
    found.pop().expect("the second adr exists")
}

#[test]
fn contract_rejects_a_supersede_chain_that_loops_back() {
    let scratch = Scratch::new("cycle");
    let bundle = two_adrs_one_superseding_the_other(&scratch);
    rewrite(&second_adr(&bundle), |text| {
        text.replacen(
            "status: Proposed",
            "status: Superseded\nsuperseded_by: 0001",
            1,
        )
    });
    let breach = contract::judge_from(&bundle, contract::SUCCESSOR_CHAINS_ARE_ACYCLIC)
        .expect_err("a supersede cycle is rejected");
    assert_eq!(breach.part, Part::SupersedeCycle);
    assert!(breach.detail.contains("0001"), "{}", breach.detail);
}

#[test]
fn contract_accepts_a_supersede_chain_that_ends_in_a_retired_record() {
    let scratch = Scratch::new("retired");
    let bundle = two_adrs_one_superseding_the_other(&scratch);
    ops::apply(
        Op::AdrStatus {
            number: 2,
            value: "Deprecated",
        },
        &bundle,
        4,
    )
    .expect("a record that superseded another may be deprecated");
    assert!(judged(&bundle, false).is_ok());
}

#[test]
fn contract_rejects_a_status_taken_by_a_superseded_record() {
    let scratch = Scratch::new("status-on-superseded");
    let bundle = two_adrs_one_superseding_the_other(&scratch);
    let before = snapshot::take(&bundle);
    let breach = contract::status_step_is_legal(&before, "ADR", 1)
        .expect_err("a status taken by a superseded record is rejected");
    assert_eq!(breach.part, Part::StatusChangedOnSuperseded);
    assert!(breach.detail.contains("0001"), "{}", breach.detail);
    assert!(contract::status_step_is_legal(&before, "ADR", 2).is_ok());
}

#[test]
fn contract_rejects_a_supersede_step_that_rewrote_an_existing_link() {
    let scratch = Scratch::new("rewrote-link");
    let bundle = built(&scratch, &[Op::NewAdr, Op::NewAdr, Op::NewAdr]);
    ops::apply(supersede("adr/0001", "adr/0002"), &bundle, 4).expect("first supersede");
    let before = snapshot::take(&bundle);
    rewrite(&first_adr(&bundle), |text| {
        text.replace("superseded_by: 0002", "superseded_by: 0003")
    });
    let breach = contract::supersede_step_is_legal(&before, &bundle, "adr/0003")
        .expect_err("a rewritten successor link is rejected");
    assert_eq!(breach.part, Part::SupersedeRewroteLink);
    assert!(breach.detail.contains("0001"), "{}", breach.detail);
}

fn supersede(old: &'static str, new: &'static str) -> Op {
    Op::Supersede { old, new }
}

#[test]
fn contract_rejects_a_supersede_step_whose_new_record_is_already_retired() {
    let scratch = Scratch::new("retired-new");
    let bundle = built(&scratch, &[Op::NewAdr, Op::NewAdr, Op::NewAdr]);
    ops::apply(supersede("adr/0001", "adr/0003"), &bundle, 4).expect("first supersede");
    let before = snapshot::take(&bundle);
    rewrite(&second_adr(&bundle), |text| {
        text.replacen(
            "status: Proposed",
            "status: Superseded\nsuperseded_by: 0001",
            1,
        )
    });
    let breach = contract::supersede_step_is_legal(&before, &bundle, "adr/0001")
        .expect_err("a retired new record is rejected");
    assert_eq!(breach.part, Part::SupersedeEndsRetired);
    assert!(breach.detail.contains("0001"), "{}", breach.detail);
}
