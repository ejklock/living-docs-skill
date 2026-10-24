//! `fmt` verb wrapper: reuses `check::check_bundle` for bundle resolution
//! and renders `living_docs_core::commands::fmt::outcome` as colored text
//! or JSON, with the git-derived timestamp and owner seeds it computes.

use crate::commands::check::check_bundle;
use crate::git;
use crate::output::{self, OutputMode};
use living_docs_core::commands::fmt::{self, Outcome, Seeds};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Serialize)]
struct WrittenJson<'a> {
    files: Vec<&'a Path>,
}

#[derive(Serialize)]
struct PendingJson<'a> {
    pending: Vec<&'a Path>,
}

pub(crate) fn run_fmt(
    docs_dir: &Path,
    paths: Vec<PathBuf>,
    check_only: bool,
    mode: OutputMode,
) -> ExitCode {
    let target = check_bundle(docs_dir, paths);
    let seeds = Seeds {
        owner: git::identity(),
        added_at: Some(git::added_at),
    };
    match fmt::outcome(&fs_store::FsStore::new(), &target, check_only, &seeds) {
        Ok(outcome) => render(&outcome, mode),
        Err(message) => {
            eprintln!("living-docs fmt: {message}");
            ExitCode::from(2)
        }
    }
}

fn render(outcome: &Outcome, mode: OutputMode) -> ExitCode {
    match outcome {
        Outcome::Rewritten(paths) => {
            render_paths(paths, "record(s) rewritten.", mode, |paths| WrittenJson {
                files: paths,
            });
            ExitCode::SUCCESS
        }
        Outcome::Pending(paths) => {
            render_paths(paths, "record(s) would change.", mode, |paths| {
                PendingJson { pending: paths }
            });
            if paths.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
    }
}

fn render_paths<'a, T: Serialize>(
    paths: &'a [PathBuf],
    summary_suffix: &str,
    mode: OutputMode,
    to_json: impl FnOnce(Vec<&'a Path>) -> T,
) {
    if mode.is_json() {
        let borrowed = paths.iter().map(PathBuf::as_path).collect();
        println!("{}", output::to_json(&to_json(borrowed)));
        return;
    }
    for path in paths {
        println!("{}", path.display());
    }
    println!("{} {summary_suffix}", paths.len());
}
