//! `check`'s pass sequence and the entry point that stops between passes.
//!
//! The passes run in one fixed order, each over the records the earlier
//! passes left in force. A cancel is looked at before every pass, never
//! inside one, because every pass reads through a cache that keeps serving
//! what it already read.

use super::Report;
use super::{
    callout, canonical, decision_table, dependencies, duplicate_numbers, frontmatter_advice, graph,
    index_drift, line_endings, link_style, links, liveness, mermaid, moved_source, path_missing,
    placeholder, read_once::ReadOnceStore, records, sections, size, stale_view, state_machine,
    supersede_cycle, supersede_links, Reporter,
};
use crate::line_ending::LineEndingStore;
use crate::store::DocStore;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// What a cancellable `check` run ends with: the finished report, or a stop
/// at a pass boundary that leaves no partial report.
pub enum Checked {
    Report(Report),
    Cancelled,
}

struct Stopped;

/// What the passes share: the two views of the store, the records still in
/// force, and the run's options.
struct Run<'a> {
    raw: &'a dyn DocStore,
    store: &'a dyn DocStore,
    bundle: &'a Path,
    root_index: PathBuf,
    all_md: Vec<PathBuf>,
    require_owner: bool,
    check_placeholders: bool,
}

type Pass = fn(&mut Run, &mut Reporter);

/// Every invariant `check` validates, in the order it validates them. The
/// first three read the whole listing; `judge_records` then drops the records
/// the store refused, which every later pass leaves out.
const PASSES: &[Pass] = &[
    |r, rep| line_endings::check_line_endings(r.raw, &r.all_md, rep),
    |r, rep| {
        if !r.root_index.is_file() {
            rep.report(&r.root_index, "missing bundle-root index.md (invariant 3)");
        }
    },
    judge_records,
    |r, rep| graph::check_directory_membership(r.raw, r.bundle, &r.all_md, rep),
    |r, rep| graph::check_reachability(r.raw, r.bundle, &r.root_index, &r.all_md, rep),
    |r, rep| index_drift::check_index_drift(r.raw, r.bundle, rep),
    |r, rep| links::check_links(r.store, r.bundle, &r.all_md, rep),
    |r, rep| link_style::check_link_style(r.store, r.bundle, &r.all_md, rep),
    |r, rep| duplicate_numbers::check_duplicate_numbers(r.bundle, &r.all_md, rep),
    |r, rep| dependencies::check_dependencies(r.store, r.bundle, &r.all_md, rep),
    |r, rep| path_missing::check_path_missing(r.store, &r.all_md, rep),
    |r, rep| stale_view::check_stale_view(r.store, &r.all_md, rep),
    |r, rep| state_machine::check_state_machine(r.store, &r.all_md, rep),
    |r, rep| decision_table::check_decision_table(r.store, r.bundle, &r.all_md, rep),
    |r, rep| supersede_cycle::check_supersede_cycle(r.store, r.bundle, &r.all_md, rep),
    |r, rep| records::check_supersede_chain(r.store, &r.all_md, rep),
    |r, rep| supersede_links::check_supersede_links(r.store, r.bundle, &r.all_md, rep),
    |r, rep| callout::check_callouts(r.store, &r.all_md, rep),
    |r, rep| moved_source::check_moved_source(r.store, r.bundle, &r.all_md, rep),
    |r, rep| records::check_owner_requirement(r.store, &r.all_md, r.require_owner, rep),
    |r, rep| records::check_heading_matches_title(r.store, &r.all_md, rep),
    |r, rep| frontmatter_advice::check_frontmatter_advice(r.store, r.bundle, &r.all_md, rep),
    |r, rep| canonical::check_canonical_frontmatter(r.store, r.bundle, &r.all_md, rep),
    |r, rep| mermaid::check_bundle(&r.all_md, rep),
    |r, rep| size::check_body_size(r.store, &r.all_md, rep),
    |r, rep| sections::check_required_sections(r.store, &r.all_md, rep),
    |r, rep| {
        if r.check_placeholders {
            placeholder::check_placeholders(r.store, r.bundle, &r.all_md, rep);
        }
    },
    |r, rep| liveness::check_liveness(r.store, r.bundle, &r.all_md, rep),
];

fn judge_records(run: &mut Run, reporter: &mut Reporter) {
    let refused =
        records::check_frontmatter_and_format(run.store, &run.all_md, &run.root_index, reporter);
    run.all_md.retain(|f| !refused.contains(f));
}

/// Runs every pass, stopping before the next one once `cancel` is raised.
/// Records are judged in LF form, mixed endings flagged from the raw view.
/// The count is the listing's, before any record is refused.
fn run_passes(
    backing: &dyn DocStore,
    bundle: &Path,
    reporter: &mut Reporter,
    options: (bool, bool),
    cancel: &AtomicBool,
    passes: &[Pass],
) -> Result<usize, Stopped> {
    if cancel.load(Ordering::SeqCst) {
        return Err(Stopped);
    }
    let raw: &dyn DocStore = &ReadOnceStore::new(backing);
    let store: &dyn DocStore = &LineEndingStore::new(raw, true);
    let all_md = store.list(bundle).unwrap_or_default();
    let doc_count = all_md.len();
    let (require_owner, check_placeholders) = options;
    let mut run = Run {
        raw,
        store,
        bundle,
        root_index: bundle.join("index.md"),
        all_md,
        require_owner,
        check_placeholders,
    };
    for pass in passes {
        if cancel.load(Ordering::SeqCst) {
            return Err(Stopped);
        }
        pass(&mut run, reporter);
    }
    Ok(doc_count)
}

/// Every invariant `check` validates, without the `bundle.is_dir()` guard,
/// header, or verdict rendering — shared by `compile` (the full report) and
/// `check_violations` (the raw list, for a caller that gates a write on the
/// same invariants). Nothing cancels it.
pub(super) fn run_all_checks(
    backing: &dyn DocStore,
    bundle: &Path,
    reporter: &mut Reporter,
    require_owner: bool,
    check_placeholders: bool,
) -> usize {
    let never = AtomicBool::new(false);
    let options = (require_owner, check_placeholders);
    run_passes(backing, bundle, reporter, options, &never, PASSES).unwrap_or_default()
}

/// [`super::compile`], stopping between passes once `cancel` is raised.
/// `None` when `bundle` is not a directory.
pub fn compile_cancellable(
    store: &dyn DocStore,
    bundle: &Path,
    require_owner: bool,
    cancel: &AtomicBool,
) -> Option<Checked> {
    if !bundle.is_dir() {
        return None;
    }
    let mut reporter = Reporter::new();
    let options = (require_owner, true);
    Some(
        match run_passes(store, bundle, &mut reporter, options, cancel, PASSES) {
            Ok(doc_count) => Checked::Report(reporter.into_report(bundle, doc_count)),
            Err(Stopped) => Checked::Cancelled,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::super::compile;
    use super::*;
    use crate::test_support::MapStore;
    use std::cell::RefCell;
    use std::fs;

    const ROOT_INDEX: &str = "# Index\n";

    struct Bundle(PathBuf);

    impl Bundle {
        fn new(label: &str, with_root_index: bool) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock after the epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!("living-docs-cancel-{label}-{nanos}"));
            fs::create_dir_all(&root).expect("create scratch bundle");
            if with_root_index {
                fs::write(root.join("index.md"), ROOT_INDEX).expect("write root index");
            }
            Self(root)
        }

        fn store(&self) -> MapStore {
            let files = [(self.0.join("index.md"), ROOT_INDEX)];
            MapStore {
                files: files.map(|(p, t)| (p, t.to_owned())).into(),
            }
        }

        fn run(&self, cancel: &AtomicBool, passes: &[Pass]) -> Result<usize, Stopped> {
            let mut reporter = Reporter::new();
            let options = (false, true);
            run_passes(&self.store(), &self.0, &mut reporter, options, cancel, passes)
        }
    }

    impl Drop for Bundle {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    thread_local! {
        static STOP: AtomicBool = const { AtomicBool::new(false) };
        static RAN: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    }

    fn first_pass(_: &mut Run, _: &mut Reporter) {
        RAN.with_borrow_mut(|ran| ran.push("first"));
    }

    fn raising_pass(_: &mut Run, _: &mut Reporter) {
        RAN.with_borrow_mut(|ran| ran.push("raising"));
        STOP.with(|stop| stop.store(true, Ordering::SeqCst));
    }

    fn last_pass(_: &mut Run, _: &mut Reporter) {
        RAN.with_borrow_mut(|ran| ran.push("last"));
    }

    #[test]
    fn a_flag_raised_before_the_run_starts_no_pass() {
        let bundle = Bundle::new("before", true);
        let flag = AtomicBool::new(true);

        let outcome = bundle.run(&flag, &[first_pass]);

        assert!(matches!(outcome, Err(Stopped)));
        assert!(RAN.with_borrow(Vec::is_empty));
    }

    #[test]
    fn a_flag_raised_during_a_pass_runs_no_later_pass() {
        let bundle = Bundle::new("passes", true);

        let outcome = STOP.with(|stop| bundle.run(stop, &[first_pass, raising_pass, last_pass]));

        assert!(matches!(outcome, Err(Stopped)));
        assert_eq!(RAN.with_borrow(Clone::clone), ["first", "raising"]);
    }

    #[test]
    fn a_flag_never_raised_runs_every_pass_in_order() {
        let bundle = Bundle::new("every", true);

        let outcome = bundle.run(&AtomicBool::new(false), &[first_pass, last_pass]);

        assert!(matches!(outcome, Ok(1)));
        assert_eq!(RAN.with_borrow(Clone::clone), ["first", "last"]);
    }

    #[test]
    fn a_flag_never_raised_returns_the_report_compile_returns() {
        for with_root_index in [true, false] {
            let bundle = Bundle::new("never", with_root_index);
            let flag = AtomicBool::new(false);
            let store = bundle.store();

            let expected = compile(&store, &bundle.0, false).expect("bundle exists");
            let outcome = compile_cancellable(&store, &bundle.0, false, &flag);

            let Some(Checked::Report(report)) = outcome else {
                panic!("a run nothing cancelled must finish with a report");
            };
            assert_eq!(
                serde_json::to_value(&report).expect("report serializes"),
                serde_json::to_value(&expected).expect("report serializes")
            );
            assert_eq!(report.ok, with_root_index);
        }
    }

    #[test]
    fn a_missing_bundle_returns_none() {
        let flag = AtomicBool::new(false);
        let store = MapStore {
            files: Default::default(),
        };
        let missing = std::env::temp_dir().join("living-docs-cancel-absent-bundle");

        assert!(compile_cancellable(&store, &missing, false, &flag).is_none());
    }
}
