use clap::{Parser, Subcommand};
use living_docs_core::{check, commands};
use std::path::{Path, PathBuf};
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
    /// Operate on the derived SQLite/FTS5 read-model at `.living-docs/index.db`.
    Db {
        #[command(subcommand)]
        cmd: DbCmd,
    },
    /// Full-text search the derived read-model, ranked best-match-first.
    Search {
        query: String,
    },
}

#[derive(Subcommand)]
enum DbCmd {
    /// Rebuild the read-model from every doc `--docs-dir` lists.
    Sync,
}

const READ_MODEL_PATH: &str = ".living-docs/index.db";

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
        Command::Db { cmd: DbCmd::Sync } => run_db_sync(&cli.docs_dir),
        Command::Search { query } => run_search(&query),
    }
}

fn run_db_sync(docs_dir: &Path) -> ExitCode {
    let runtime = match build_runtime() {
        Ok(runtime) => runtime,
        Err(err) => return report_failure(&err.to_string()),
    };
    match runtime.block_on(sync_read_model(docs_dir)) {
        Ok(count) => {
            println!("Indexed {count} records.");
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err.to_string()),
    }
}

async fn sync_read_model(docs_dir: &Path) -> db_store::Result<usize> {
    let conn = db_store::connect(Path::new(READ_MODEL_PATH)).await?;
    db_store::migrate(&conn).await?;
    db_store::sync(&conn, &fs_store::FsStore::new(), docs_dir).await
}

fn run_search(query: &str) -> ExitCode {
    if !Path::new(READ_MODEL_PATH).exists() {
        eprintln!("no index found at {READ_MODEL_PATH}; run: living-docs db sync");
        return ExitCode::FAILURE;
    }

    let runtime = match build_runtime() {
        Ok(runtime) => runtime,
        Err(err) => return report_failure(&err.to_string()),
    };
    match runtime.block_on(search_read_model(query)) {
        Ok(hits) => {
            print_hits(&hits);
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err.to_string()),
    }
}

async fn search_read_model(query: &str) -> db_store::Result<Vec<db_store::SearchHit>> {
    let conn = db_store::connect(Path::new(READ_MODEL_PATH)).await?;
    db_store::search(&conn, query).await
}

fn print_hits(hits: &[db_store::SearchHit]) {
    for hit in hits {
        println!("{} — {}", hit.path, hit.title);
        println!("{}", hit.snippet);
    }
}

fn build_runtime() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}

fn report_failure(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::FAILURE
}
