//! `mcp` command wrapper: refuses a missing bundle root, then serves the
//! bundle to an MCP client over stdio until the client closes its end.

use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_mcp(docs_dir: &Path) -> ExitCode {
    if !docs_dir.is_dir() {
        eprintln!(
            "living-docs mcp: bundle root not found: {}",
            docs_dir.display()
        );
        return ExitCode::from(2);
    }
    match living_docs_mcp::serve_stdio_blocking(docs_dir) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("living-docs mcp: {error}");
            ExitCode::from(1)
        }
    }
}
