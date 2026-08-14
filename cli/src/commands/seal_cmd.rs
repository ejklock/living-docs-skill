//! `seal` verb wrapper (ADR 0039): `init` baselines the clone — fresh key
//! under `.git/living-docs/` plus a ledger sealing the current bundle.

use living_docs_core::seal;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_seal_init(docs_dir: &Path) -> ExitCode {
    match seal::init(&fs_store::FsStore::new(), docs_dir) {
        Ok(count) => {
            println!(
                "Sealed {count} record(s) under {} — trusted baseline set; `check` now verifies provenance on this clone.",
                docs_dir.display()
            );
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs seal init: {message}");
            ExitCode::from(2)
        }
    }
}
