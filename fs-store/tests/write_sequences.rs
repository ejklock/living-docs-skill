//! Bounded exhaustive exploration of write sequences over a small bundle:
//! after every step of every sequence up to a fixed length the bundle must
//! keep the contract in `contract`, whether the step succeeded or was refused.

#[path = "write_sequences/contract.rs"]
mod contract;
#[path = "write_sequences/ops.rs"]
mod ops;
#[path = "write_sequences/rejections.rs"]
mod rejections;
#[path = "write_sequences/snapshot.rs"]
mod snapshot;

use contract::Breach;
use ops::Op;
use snapshot::Snapshot;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_LENGTH: usize = 3;
const ROOT_INDEX: &str = "---\nokf_version: \"0.1\"\n---\n\n# Documentation\n\n## Trail\n";

/// A scratch directory under the temporary directory, removed on drop.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(label: &str) -> Self {
        let base =
            std::env::temp_dir().join(format!("write-sequences-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).expect("create scratch directory");
        let path = base.canonicalize().expect("canonicalize scratch directory");
        Self { path }
    }

    fn bundle(&self, name: &str) -> PathBuf {
        let bundle = self.path.join(name);
        fs::create_dir_all(&bundle).expect("create bundle directory");
        fs::write(bundle.join("index.md"), ROOT_INDEX).expect("write root index");
        bundle
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// A bundle every exploration starts from, built with the same operations a
/// sequence uses.
#[derive(Debug, Clone, Copy)]
struct Start {
    name: &'static str,
    steps: &'static [Op],
}

const SUPERSEDE_FIRST_BY_SECOND: Op = Op::Supersede {
    old: "adr/0001",
    new: "adr/0002",
};

const STARTS: [Start; 3] = [
    Start {
        name: "empty bundle",
        steps: &[],
    },
    Start {
        name: "two ADRs and an issue",
        steps: &[Op::NewAdr, Op::NewAdr, Op::NewIssue],
    },
    Start {
        name: "two ADRs, the first superseded by the second, and an issue",
        steps: &[
            Op::NewAdr,
            Op::NewAdr,
            SUPERSEDE_FIRST_BY_SECOND,
            Op::NewIssue,
        ],
    },
];

/// One exploration: where it runs, the sequence being extended, the bundles
/// already reached and how far it went.
struct Run {
    scratch: PathBuf,
    start: Start,
    sequence: Vec<Op>,
    reached: HashSet<(usize, Snapshot)>,
    nodes: usize,
    operations_run: BTreeSet<String>,
    judgments_run: BTreeSet<Vec<&'static str>>,
    sequences: usize,
    states: usize,
}

impl Run {
    fn new(scratch: &Path, start: Start) -> Self {
        Self {
            scratch: scratch.to_path_buf(),
            start,
            sequence: Vec::new(),
            reached: HashSet::new(),
            nodes: 0,
            operations_run: BTreeSet::new(),
            judgments_run: BTreeSet::new(),
            sequences: 0,
            states: 0,
        }
    }
}

/// Runs one step over a copy of `parent`, judges the copy, then explores
/// every longer sequence from it. A bundle already reached at the same
/// length has the same judgment and the same continuations, so it is judged
/// and explored once and its sequences are counted from the closed form.
fn explore(parent: &Path, run: &mut Run) {
    for op in ops::alphabet() {
        run.nodes += 1;
        run.operations_run.insert(format!("{op:?}"));
        let child = run.scratch.join(format!("node-{}", run.nodes));
        snapshot::copy_tree(parent, &child);
        let before = snapshot::take(&child);
        run.sequence.push(op);
        let refused = ops::apply(op, &child, run.start.steps.len() + run.sequence.len()).is_err();
        let after = snapshot::take(&child);
        run.sequences += 1;
        if let Err(breach) = judge(refused, &before, after, &child, run) {
            panic!("{}", failure_message(&run.start, &run.sequence, &breach));
        }
        run.sequence.pop();
        let _ = fs::remove_dir_all(&child);
    }
}

/// Judges one step's result and, when its bundle is new at this length,
/// explores the sequences that continue from it.
fn judge(
    refused: bool,
    before: &Snapshot,
    after: Snapshot,
    child: &Path,
    run: &mut Run,
) -> Result<(), Breach> {
    if refused {
        contract::refused_step_left_bundle_alone(before, &after)?;
    }
    if !run.reached.insert((run.sequence.len(), after)) {
        run.sequences += descendants(MAX_LENGTH - run.sequence.len());
        return Ok(());
    }
    run.states += 1;
    let ran = contract::judge_state(child)?;
    run.judgments_run.insert(ran);
    if run.sequence.len() < MAX_LENGTH {
        explore(child, run);
    }
    Ok(())
}

/// The sequences that extend a sequence by up to `remaining` more steps.
fn descendants(remaining: usize) -> usize {
    let width = ops::alphabet().len();
    (1..=remaining).map(|steps| width.pow(steps as u32)).sum()
}

fn failure_message(start: &Start, sequence: &[Op], breach: &Breach) -> String {
    format!(
        "from the {} the sequence {sequence:?} broke the contract at {:?}: {}",
        start.name, breach.part, breach.detail
    )
}

fn explore_from(start: Start, label: &str) {
    let scratch = Scratch::new(label);
    let root = scratch.bundle("root");
    for (index, op) in start.steps.iter().enumerate() {
        ops::apply(*op, &root, index + 1).expect("starting step is accepted");
    }
    contract::judge_state(&root).expect("the starting bundle keeps the contract");
    let mut run = Run::new(&scratch.path, start);
    explore(&root, &mut run);
    println!(
        "from the {}: enumerated {} sequences of length 1 to {MAX_LENGTH} over {} operations ({} steps run, {} distinct bundles judged)",
        start.name,
        run.sequences,
        ops::alphabet().len(),
        run.nodes,
        run.states
    );
    assert_eq!(run.operations_run.len(), ops::alphabet().len());
    assert!(run.nodes >= ops::alphabet().len());
    assert_every_judged_state_ran_every_judgment(&run);
}

/// Every bundle the search judged ran the whole ordered list of state
/// judgments, so a judgment the search skips fails here instead of passing
/// unnoticed.
fn assert_every_judged_state_ran_every_judgment(run: &Run) {
    let expected: Vec<&str> = contract::STATE_PARTS
        .iter()
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(
        run.judgments_run,
        BTreeSet::from([expected.clone()]),
        "from the {} every judged bundle must run {expected:?}",
        run.start.name
    );
}

#[test]
fn every_sequence_from_the_empty_bundle_keeps_the_contract() {
    explore_from(STARTS[0], "explore-empty");
}

#[test]
fn every_sequence_from_two_adrs_and_an_issue_keeps_the_contract() {
    explore_from(STARTS[1], "explore-two-adrs");
}

#[test]
fn every_sequence_from_a_superseded_pair_keeps_the_contract() {
    explore_from(STARTS[2], "explore-superseded-pair");
}

#[test]
fn the_search_bound_matches_the_alphabet() {
    assert_eq!(descendants(1), ops::alphabet().len());
    assert_eq!(descendants(MAX_LENGTH), 17 + 17 * 17 + 17 * 17 * 17);
}
