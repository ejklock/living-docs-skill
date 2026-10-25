//! `index` verb wrapper: delegates to `living_docs_core::commands::index::write`,
//! rendering the written paths as colored text or JSON (ADR 0060).

use crate::output::{self, OutputMode};
use crate::store::build_store;
use living_docs_core::commands;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Serialize)]
struct FilesJson<'a> {
    files: Vec<&'a Path>,
}

pub(crate) fn run_index(docs_dir: &Path, doc_type: Option<String>, mode: OutputMode) -> ExitCode {
    match commands::index::write(build_store(docs_dir).as_ref(), docs_dir, doc_type) {
        Ok(written) => render(&written, mode),
        Err(message) => {
            eprintln!("living-docs index: {message}");
            ExitCode::from(2)
        }
    }
}

fn render(written: &[PathBuf], mode: OutputMode) -> ExitCode {
    if mode.is_json() {
        let files = written.iter().map(PathBuf::as_path).collect();
        println!("{}", output::to_json(&FilesJson { files }));
    } else {
        for path in written {
            println!("{}", path.display());
        }
    }
    ExitCode::SUCCESS
}
