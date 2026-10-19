//! `supersede` verb wrapper: delegates to
//! `living_docs_core::commands::supersede::supersede`, rendering the two
//! resolved paths as colored text or JSON (ADR 0060).

use super::report::report_indexed;
use crate::output::{self, OutputMode};
use crate::store::build_store;
use living_docs_core::commands;
use serde::Serialize;
use std::path::Path;
use std::process::ExitCode;

#[derive(Serialize)]
struct SupersedeJson<'a> {
    old: &'a Path,
    new: &'a Path,
    indexed: &'a [std::path::PathBuf],
}

pub(crate) fn run_supersede(
    docs_dir: &Path,
    old: &str,
    new: &str,
    mode: OutputMode,
    quiet: bool,
) -> ExitCode {
    match commands::supersede::supersede(build_store().as_ref(), docs_dir, old, new) {
        Ok(superseded) => {
            if mode.is_json() {
                println!(
                    "{}",
                    output::to_json(&SupersedeJson {
                        old: &superseded.old,
                        new: &superseded.new,
                        indexed: &superseded.indexed.written,
                    })
                );
            } else {
                println!("{}", superseded.old.display());
                println!("{}", superseded.new.display());
            }
            report_indexed("supersede", &superseded.indexed, mode, quiet)
        }
        Err(message) => {
            eprintln!("living-docs supersede: {message}");
            ExitCode::from(2)
        }
    }
}
