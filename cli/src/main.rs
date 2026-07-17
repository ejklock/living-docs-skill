use clap::{Parser, Subcommand, ValueEnum};
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
    /// Operate on the derived read-model (SQLite/FTS5 by default, or ParadeDB
    /// with `--engine paradedb`).
    Db {
        #[command(subcommand)]
        cmd: DbCmd,
    },
    /// Full-text search the derived read-model, ranked best-match-first.
    Search {
        query: String,
        /// Which database backend to search: the local SQLite/FTS5 file, or
        /// ParadeDB via `$DATABASE_URL`.
        #[arg(long, value_enum, default_value = "sqlite")]
        engine: Engine,
    },
}

#[derive(Subcommand)]
enum DbCmd {
    /// Rebuild the read-model from every doc `--docs-dir` lists.
    Sync {
        /// Which database backend to sync into: the local SQLite/FTS5 file,
        /// or ParadeDB via `$DATABASE_URL`.
        #[arg(long, value_enum, default_value = "sqlite")]
        engine: Engine,
    },
}

/// The database backend to connect to, selectable per invocation (ADR 0004,
/// issue 0004).
#[derive(Clone, Copy, Debug, ValueEnum)]
enum Engine {
    Sqlite,
    Paradedb,
}

const SQLITE_READ_MODEL_PATH: &str = ".living-docs/index.db";
const DATABASE_URL_VAR: &str = "DATABASE_URL";

impl Engine {
    fn resolve_url(self) -> Result<String, String> {
        self.resolve_url_with(|name| std::env::var(name))
    }

    fn resolve_url_with(
        self,
        lookup_env: impl Fn(&str) -> Result<String, std::env::VarError>,
    ) -> Result<String, String> {
        match self {
            Engine::Sqlite => Ok(format!("sqlite://{SQLITE_READ_MODEL_PATH}?mode=rwc")),
            Engine::Paradedb => lookup_env(DATABASE_URL_VAR).map_err(|_| {
                format!(
                    "the paradedb engine requires ${DATABASE_URL_VAR} to be set to a Postgres connection string"
                )
            }),
        }
    }
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
        Command::Db {
            cmd: DbCmd::Sync { engine },
        } => run_db_sync(&cli.docs_dir, engine),
        Command::Search { query, engine } => run_search(&query, engine),
    }
}

fn run_db_sync(docs_dir: &Path, engine: Engine) -> ExitCode {
    let url = match engine.resolve_url() {
        Ok(url) => url,
        Err(err) => return report_failure(&err),
    };
    let runtime = match build_runtime() {
        Ok(runtime) => runtime,
        Err(err) => return report_failure(&err.to_string()),
    };
    match runtime.block_on(sync_read_model(docs_dir, &url)) {
        Ok(count) => {
            println!("Indexed {count} records.");
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err.to_string()),
    }
}

async fn sync_read_model(docs_dir: &Path, url: &str) -> db_store::Result<usize> {
    let conn = db_store::connect(url).await?;
    db_store::migrate(&conn).await?;
    db_store::sync(&conn, &fs_store::FsStore::new(), docs_dir).await
}

fn run_search(query: &str, engine: Engine) -> ExitCode {
    if matches!(engine, Engine::Sqlite) && !Path::new(SQLITE_READ_MODEL_PATH).exists() {
        eprintln!("no index found at {SQLITE_READ_MODEL_PATH}; run: living-docs db sync");
        return ExitCode::FAILURE;
    }

    let url = match engine.resolve_url() {
        Ok(url) => url,
        Err(err) => return report_failure(&err),
    };
    let runtime = match build_runtime() {
        Ok(runtime) => runtime,
        Err(err) => return report_failure(&err.to_string()),
    };
    match runtime.block_on(search_read_model(query, &url)) {
        Ok(hits) => {
            print_hits(&hits);
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err.to_string()),
    }
}

async fn search_read_model(query: &str, url: &str) -> db_store::Result<Vec<db_store::SearchHit>> {
    let conn = db_store::connect(url).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_sqlite_resolves_to_the_local_read_model_url() {
        let url = Engine::Sqlite
            .resolve_url_with(|_| Ok("unused".to_owned()))
            .expect("sqlite url always resolves");
        assert_eq!(url, format!("sqlite://{SQLITE_READ_MODEL_PATH}?mode=rwc"));
    }

    #[test]
    fn engine_paradedb_resolves_the_configured_database_url() {
        let url = Engine::Paradedb
            .resolve_url_with(|_| Ok("postgres://user:pass@localhost/db".to_owned()))
            .expect("paradedb url resolves when DATABASE_URL is set");
        assert_eq!(url, "postgres://user:pass@localhost/db");
    }

    #[test]
    fn engine_paradedb_errors_clearly_when_database_url_is_unset() {
        let err = Engine::Paradedb
            .resolve_url_with(|_| Err(std::env::VarError::NotPresent))
            .expect_err("paradedb url resolution fails without DATABASE_URL");
        assert!(err.contains(DATABASE_URL_VAR), "got: {err}");
    }
}
