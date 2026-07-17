use clap::{Parser, Subcommand};
use living_docs_core::{check, commands};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "living-docs",
    version,
    about = "Deterministic layer of Living Docs authoring"
)]
struct Cli {
    /// Root of the docs bundle. Overridable so tests can point at a temp tree.
    #[arg(long, global = true, default_value = "docs")]
    docs_dir: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    New {
        doc_type: String,
        title: String,
    },
    Index {
        doc_type: Option<String>,
    },
    Supersede {
        old: String,
        new: String,
    },
    Next {
        doc_type: String,
    },
    /// Validate the mechanical Living Docs invariants on a docs bundle, matching
    /// `lint-docs.sh`'s `[BUNDLE_ROOT]` argument (default `docs`) rather than the
    /// global `--docs-dir`. With `--mermaid-only`, `paths` instead lists the
    /// file(s)/directory(ies) to sweep for ```mermaid``` fences (default:
    /// git-tracked `*.md`, fixtures dir excluded), matching `lint-mermaid.sh`.
    Check {
        paths: Vec<PathBuf>,
        /// Validate only ```mermaid``` fences over `paths`, skipping every other invariant.
        #[arg(long)]
        mermaid_only: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Next { doc_type } => commands::next::run(&cli.docs_dir, &doc_type),
        Command::New { doc_type, title } => commands::new::run(&cli.docs_dir, &doc_type, &title),
        Command::Index { doc_type } => commands::index::run(&cli.docs_dir, doc_type),
        Command::Supersede { old, new } => commands::supersede::run(&cli.docs_dir, &old, &new),
        Command::Check {
            paths,
            mermaid_only,
        } if mermaid_only => check::run_mermaid_only(&paths),
        Command::Check { paths, .. } => {
            let bundle = paths
                .into_iter()
                .next()
                .unwrap_or_else(|| PathBuf::from("docs"));
            let store = fs_store::FsStore::new();
            check::run(&store, &bundle)
        }
    }
}
