//! `mcp` command wrapper: serves the bundle to an MCP client over stdio until
//! the client closes its end. The bundle root need not exist at start; each
//! tool call resolves it, and only `write new` creates it.

use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_mcp(docs_dir: &Path) -> ExitCode {
    match living_docs_mcp::serve_stdio_blocking(docs_dir) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("living-docs mcp: {error}");
            ExitCode::from(1)
        }
    }
}
