//! `manage view --splice` wrapper: reads the Mermaid text from standard input,
//! delegates to `living_docs_core::commands::splice::apply` and renders the
//! stamped view as text or JSON.

use crate::commands::prose;
use crate::commands::report::report_indexed;
use crate::output::{self, OutputMode};
use crate::store::build_store;
use living_docs_core::commands::splice::{self, Request, Spliced};
use living_docs_core::tla_export;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Serialize)]
struct SpliceJson<'a> {
    path: &'a Path,
    created: bool,
    warnings: &'a [String],
    indexed: &'a [PathBuf],
}

/// Splices standard input into the generated region of `view`; a refusal is a
/// usage error (exit 2) and writes nothing.
pub(crate) fn run_splice(
    docs_dir: &Path,
    view: &str,
    inputs: &[String],
    command: Option<&str>,
    mode: OutputMode,
    quiet: bool,
) -> ExitCode {
    let done = prose::read_source("-").and_then(|mermaid| {
        let request = Request {
            view,
            inputs,
            command,
            mermaid: &mermaid,
        };
        splice::apply(build_store(docs_dir).as_ref(), docs_dir, &request)
    });
    match done {
        Ok(spliced) => {
            print_spliced(&spliced, mode);
            report_indexed("manage view", &spliced.indexed, mode, quiet)
        }
        Err(message) => {
            eprintln!("living-docs manage view: {message}");
            ExitCode::from(2)
        }
    }
}

/// Prints the TLA+ module of the state view `view` to standard output; a
/// refusal is a usage error (exit 2) and prints nothing to standard output.
pub(crate) fn run_export_tla(docs_dir: &Path, view: &str) -> ExitCode {
    match tla_export::export_view(build_store(docs_dir).as_ref(), docs_dir, view) {
        Ok(module) => {
            output::write_stdout_bytes(module.as_bytes());
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs manage view: {message}");
            ExitCode::from(2)
        }
    }
}

fn print_spliced(spliced: &Spliced, mode: OutputMode) {
    if mode.is_json() {
        output::outln!(
            "{}",
            output::to_json(&SpliceJson {
                path: &spliced.path,
                created: spliced.created,
                warnings: &spliced.warnings,
                indexed: &spliced.indexed.written,
            })
        );
        return;
    }
    output::outln!("{}", spliced.path.display());
    for warning in &spliced.warnings {
        eprintln!("living-docs manage view: warning: {warning}");
    }
}
