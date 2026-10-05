//! `read` verb wrapper (renamed from `effective` by ADR 0060): delegates to
//! `living_docs_core::commands::effective` to compile and print the
//! agent-facing view, as colored text or JSON per the resolved output mode.

use crate::args::ReadArgs;
use crate::output::OutputMode;
use crate::store::build_store;
use living_docs_core::commands::effective::{self, Options};
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_read(docs_dir: &Path, args: ReadArgs, mode: OutputMode) -> ExitCode {
    let options = Options {
        topic: args.topic,
        full: args.full,
        contract: args.contract,
    };
    let store = build_store();
    let output = if mode.is_json() {
        effective::compile_json(store.as_ref(), docs_dir, &options)
    } else {
        effective::compile(store.as_ref(), docs_dir, &options)
    };
    print!("{output}");
    ExitCode::SUCCESS
}
