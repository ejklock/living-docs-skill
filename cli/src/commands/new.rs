//! `new` verb wrapper: delegates to `living_docs_core::commands::new::write`,
//! rendering the created path as colored text or JSON.

use super::prose;
use super::report::report_indexed;
use crate::output::{self, OutputMode};
use crate::store::build_store;
use fs_store::git::seeded_owner;
use living_docs_core::commands;
use living_docs_core::commands::effective::{self, Asked, Candidate, Query};
use living_docs_core::commands::new::{NewOptions, BODY_ONLY_INSTRUCTION};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The `new` verb's optional CLI arguments, bundled so the wrapper's
/// signature stays within the argument budget as flags accrue.
pub(crate) struct NewArgs<'a> {
    pub(crate) description: Option<&'a str>,
    pub(crate) kind: Option<&'a str>,
    pub(crate) owner: Option<&'a str>,
    pub(crate) body: Option<&'a str>,
    pub(crate) dir: Option<&'a str>,
}

#[derive(Serialize)]
struct NewJson<'a> {
    path: &'a Path,
    instruction: &'a str,
    indexed: &'a [PathBuf],
    #[serde(skip_serializing_if = "Option::is_none")]
    in_force: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ask: Option<&'a str>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    candidates: &'a [Candidate],
}

/// Prints the created path as JSON or text; `asked` is the count and the
/// candidates when the new record is an ADR, which adds the question to both:
/// JSON fields, or stderr lines that `quiet` silences.
fn print_created(
    created: &commands::new::Created,
    mode: OutputMode,
    asked: Option<&Asked>,
    quiet: bool,
) {
    let in_force = asked.map(|asked| asked.in_force);
    let candidates = asked.map_or(&[][..], |asked| asked.candidates.as_slice());
    if mode.is_json() {
        output::outln!(
            "{}",
            output::to_json(&NewJson {
                path: &created.path,
                instruction: BODY_ONLY_INSTRUCTION,
                indexed: &created.indexed.written,
                in_force,
                ask: in_force.map(|_| effective::adr_ask()),
                candidates,
            })
        );
    } else {
        output::outln!("{}", created.path.display());
        output::outln!("{BODY_ONLY_INSTRUCTION}");
        if let (Some(count), false) = (in_force, quiet) {
            eprintln!("{}", effective::adr_ask_line(count));
            for candidate in candidates {
                eprintln!("{}", effective::candidate_line(candidate));
            }
        }
    }
}

pub(crate) fn run_new(
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    args: &NewArgs,
    mode: OutputMode,
    quiet: bool,
) -> ExitCode {
    let seeded = seeded_owner(docs_dir, doc_type, args.owner);
    if let (Some(owner), false) = (&seeded, quiet) {
        eprintln!("living-docs write new: owner seeded from git config: {owner}");
    }
    let body = match args.body.map(prose::read_body).transpose() {
        Ok(body) => body,
        Err(message) => {
            eprintln!("living-docs write new: {message}");
            return ExitCode::from(2);
        }
    };
    let opts = NewOptions {
        description: args.description,
        kind: args.kind,
        owner: args.owner.or(seeded.as_deref()),
        body: body.as_deref(),
        dir: args.dir,
    };
    create(
        docs_dir,
        &Record { doc_type, title },
        &opts,
        Report { mode, quiet },
    )
}

/// The type and title of the record to scaffold.
struct Record<'a> {
    doc_type: &'a str,
    title: &'a str,
}

/// How the created record is reported.
#[derive(Clone, Copy)]
struct Report {
    mode: OutputMode,
    quiet: bool,
}

/// Writes the record and reports it; an ADR's count of records in force and
/// its candidates are read before the write, so the new record is neither
/// counted nor listed.
fn create(docs_dir: &Path, record: &Record, opts: &NewOptions, report: Report) -> ExitCode {
    let Record { doc_type, title } = *record;
    let Report { mode, quiet } = report;
    let store = build_store(docs_dir);
    let query = Query::new(title, opts.description, opts.body);
    let asked = effective::adr_asked(store.as_ref(), docs_dir, doc_type, &query);
    match commands::new::write(store.as_ref(), docs_dir, doc_type, title, opts) {
        Ok(created) => {
            print_created(&created, mode, asked.as_ref(), quiet);
            report_indexed("write new", &created.indexed, mode, quiet)
        }
        Err(message) => {
            eprintln!("living-docs write new: {message}");
            ExitCode::from(2)
        }
    }
}
