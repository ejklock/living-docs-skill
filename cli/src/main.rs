use clap::{Parser, Subcommand, ValueEnum};
use living_docs_core::store::DocStore;
use living_docs_core::{check, commands, paths};
use std::io;
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

    /// Which persistence backend `new`/`check`/`export`/`index`/`supersede`
    /// operate against: the local `.md` tree (`fs`, default) or the
    /// SQLite/ParadeDB read-model (`db`), scoped to a project derived from
    /// `--docs-dir` (ADR 0007, issue 0006 slices 0006-D2/0006-E). `index`'s
    /// output artifact (`index.md`) is always written to the filesystem
    /// regardless of this flag — only the records feeding it move through
    /// the active backend (ADR 0007: `index.md` is fs-only).
    #[arg(long, global = true, value_enum, default_value = "fs")]
    backend: Backend,

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
        /// Restrict the rendered index to records whose effective visibility
        /// (frontmatter `visibility`, or `private` when absent — default-deny,
        /// ADR 0009) is in this comma-separated set. Omitted: every record
        /// renders, unchanged from today's dev view.
        #[arg(long, value_delimiter = ',')]
        visibility: Option<Vec<String>>,
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
    /// Materializes every record the active `--backend` lists back into
    /// conformant `.md` files under `out_dir` — the lossless round-trip
    /// fitness function (ADR 0007, issue 0006 slice 0006-D2).
    Export {
        out_dir: PathBuf,
        /// Restrict the exported set to records whose effective visibility
        /// (frontmatter `visibility`, or `private` when absent —
        /// default-deny, ADR 0010) is in this comma-separated set. Omitted:
        /// every record exports, unchanged from today's behavior.
        #[arg(long, value_delimiter = ',')]
        visibility: Option<Vec<String>>,
    },
    /// Operate on the derived read-model (SQLite/FTS5 by default, or ParadeDB
    /// with `--engine paradedb`).
    Db {
        #[command(subcommand)]
        cmd: DbCmd,
    },
    /// Fails closed when an exported bundle leaks a private doc, or a
    /// dangling link to a doc withheld from the bundle (ADR 0010 leak gate,
    /// part 1 — always inspects a materialized filesystem bundle, regardless
    /// of `--backend`).
    LeakGate {
        bundle: PathBuf,
        /// Additionally runs the Tier-3 PII detectors (ADR 0012) — the
        /// highest-false-positive class, so they stay opt-in rather than
        /// running by default.
        #[arg(long)]
        check_tier3: bool,
    },
    /// Full-text search the derived read-model, ranked best-match-first.
    Search {
        query: String,
        /// Which database backend to search: the local SQLite/FTS5 file, or
        /// ParadeDB via `$DATABASE_URL`.
        #[arg(long, value_enum, default_value = "sqlite")]
        engine: Engine,
        /// Narrow results to one project's slug. Omitted spans every
        /// project, labeling each hit by the project it belongs to (ADR
        /// 0005, issue 0005 slice 0005-C1).
        #[arg(long)]
        project: Option<String>,
    },
}

#[derive(Subcommand)]
enum DbCmd {
    /// Rebuild the read-model from every doc `--docs-dir` lists, scoped to
    /// one named project (ADR 0005, issue 0005 slice 0005-B).
    Sync {
        /// Which database backend to sync into: the local SQLite/FTS5 file,
        /// or ParadeDB via `$DATABASE_URL`.
        #[arg(long, value_enum, default_value = "sqlite")]
        engine: Engine,
        /// The project slug to sync into. Defaults to a slug derived from
        /// `--docs-dir`'s own directory name, keeping single-project usage
        /// working without naming a project explicitly.
        #[arg(long)]
        project: Option<String>,
    },
}

/// The database backend to connect to, selectable per invocation (ADR 0004,
/// issue 0004).
#[derive(Clone, Copy, Debug, ValueEnum)]
enum Engine {
    Sqlite,
    Paradedb,
}

/// The persistence backend `new`/`check`/`export` operate against (ADR
/// 0007, issue 0006 slice 0006-D2).
#[derive(Clone, Copy, Debug, ValueEnum)]
enum Backend {
    Fs,
    Db,
}

const SQLITE_READ_MODEL_PATH: &str = ".living-docs/index.db";
const DATABASE_URL_VAR: &str = "DATABASE_URL";

impl Engine {
    fn resolve_url(self) -> Result<String, String> {
        self.resolve_url_with(|name| std::env::var(name))
    }

    /// `Sqlite` honors `$DATABASE_URL` when set (accepting a full
    /// `sqlite://…` value, e.g. a hermetic per-test database), falling back
    /// to the local read-model path otherwise; `Paradedb` requires
    /// `$DATABASE_URL` unconditionally.
    fn resolve_url_with(
        self,
        lookup_env: impl Fn(&str) -> Result<String, std::env::VarError>,
    ) -> Result<String, String> {
        match self {
            Engine::Sqlite => {
                Ok(lookup_env(DATABASE_URL_VAR).unwrap_or_else(|_| default_sqlite_url()))
            }
            Engine::Paradedb => lookup_env(DATABASE_URL_VAR).map_err(|_| {
                format!(
                    "the paradedb engine requires ${DATABASE_URL_VAR} to be set to a Postgres connection string"
                )
            }),
        }
    }
}

/// The connection string `Engine::Sqlite` resolves to when `$DATABASE_URL`
/// is unset — the single source of truth for what "the default local
/// SQLite backend" means, shared by [`Engine::resolve_url_with`] and
/// [`is_default_local_sqlite`].
fn default_sqlite_url() -> String {
    format!("sqlite://{SQLITE_READ_MODEL_PATH}?mode=rwc")
}

/// True only when `engine`/`url` is the default local SQLite backend
/// (`Engine::Sqlite` with `$DATABASE_URL` unset), the one case where the
/// `.living-docs/index.db` file existence check in [`run_search`] is a
/// reliable signal — a `Sqlite` engine pointed at an overridden URL, or
/// `Paradedb`, may have no local file at all yet still have a valid index.
fn is_default_local_sqlite(engine: Engine, url: &str) -> bool {
    matches!(engine, Engine::Sqlite) && url == default_sqlite_url()
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Next { doc_type } => commands::next::run(&cli.docs_dir, &doc_type),
        Command::New { doc_type, title } => run_new(cli.backend, &cli.docs_dir, &doc_type, &title),
        Command::Index {
            doc_type,
            visibility,
        } => run_index(cli.backend, &cli.docs_dir, doc_type, visibility),
        Command::Supersede { old, new } => run_supersede(cli.backend, &cli.docs_dir, &old, &new),
        Command::Check {
            paths,
            mermaid_only,
        } if mermaid_only => check::run_mermaid_only(&paths),
        Command::Check { paths, .. } => run_check(cli.backend, &cli.docs_dir, paths),
        Command::Export {
            out_dir,
            visibility,
        } => run_export(cli.backend, &cli.docs_dir, &out_dir, visibility),
        Command::LeakGate {
            bundle,
            check_tier3,
        } => run_leak_gate(&bundle, check_tier3),
        Command::Db {
            cmd: DbCmd::Sync { engine, project },
        } => run_db_sync(&cli.docs_dir, engine, project),
        Command::Search {
            query,
            engine,
            project,
        } => run_search(&query, engine, project),
    }
}

fn run_new(backend: Backend, docs_dir: &Path, doc_type: &str, title: &str) -> ExitCode {
    match build_backend_store(backend, docs_dir) {
        Ok(store) => commands::new::run(store.as_ref(), docs_dir, doc_type, title),
        Err(err) => report_failure(&err),
    }
}

fn run_index(
    backend: Backend,
    docs_dir: &Path,
    doc_type: Option<String>,
    visibility: Option<Vec<String>>,
) -> ExitCode {
    match build_backend_store(backend, docs_dir) {
        Ok(store) => commands::index::run(store.as_ref(), docs_dir, doc_type, visibility),
        Err(err) => report_failure(&err),
    }
}

fn run_supersede(backend: Backend, docs_dir: &Path, old: &str, new: &str) -> ExitCode {
    match build_backend_store(backend, docs_dir) {
        Ok(store) => commands::supersede::run(store.as_ref(), docs_dir, old, new),
        Err(err) => report_failure(&err),
    }
}

fn run_check(backend: Backend, docs_dir: &Path, paths: Vec<PathBuf>) -> ExitCode {
    let bundle = check_bundle(backend, docs_dir, paths);
    match build_backend_store(backend, &bundle) {
        Ok(store) => check::run(store.as_ref(), &bundle),
        Err(err) => report_failure(&err),
    }
}

/// The db backend has no notion of `check`'s `[BUNDLE_ROOT]` positional
/// argument — its `DocStore` is scoped to `--docs-dir` at construction — so
/// it always checks `docs_dir`, ignoring `paths`; the fs backend keeps its
/// existing `lint-docs.sh`-compatible behavior unchanged.
fn check_bundle(backend: Backend, docs_dir: &Path, paths: Vec<PathBuf>) -> PathBuf {
    match backend {
        Backend::Db => docs_dir.to_path_buf(),
        Backend::Fs => paths
            .into_iter()
            .next()
            .unwrap_or_else(|| PathBuf::from("docs")),
    }
}

fn run_export(
    backend: Backend,
    docs_dir: &Path,
    out_dir: &Path,
    visibility: Option<Vec<String>>,
) -> ExitCode {
    match build_backend_store(backend, docs_dir) {
        Ok(store) => commands::export::export(store.as_ref(), docs_dir, out_dir, visibility),
        Err(err) => report_failure(&err),
    }
}

/// Always inspects a materialized filesystem bundle — a bundle is a directory
/// tree `export` already wrote, not a `--backend`-selectable projection.
/// `check_tier3` threads `--check-tier3` down to the opt-in Tier-3 PII scan.
fn run_leak_gate(bundle: &Path, check_tier3: bool) -> ExitCode {
    commands::leak_gate::run(&fs_store::FsStore::new(), bundle, check_tier3)
}

fn build_backend_store(backend: Backend, root: &Path) -> Result<Box<dyn DocStore>, String> {
    match backend {
        Backend::Fs => Ok(Box::new(fs_store::FsStore::new())),
        Backend::Db => build_db_doc_store(root).map(|store| Box::new(store) as Box<dyn DocStore>),
    }
}

/// Opens (migrating if needed) the db backend's connection, bootstraps
/// `root`'s project if this is its first use, then hands back a
/// [`db_store::DbDocStore`] scoped to it — the `--backend db` counterpart
/// of [`fs_store::FsStore::new`].
fn build_db_doc_store(root: &Path) -> Result<db_store::DbDocStore, String> {
    let url = Engine::Sqlite.resolve_url()?;
    let project_slug = derive_project_slug(root);
    let runtime = build_runtime().map_err(|e| e.to_string())?;
    runtime
        .block_on(prepare_db_project(&url, root, &project_slug))
        .map_err(|e| e.to_string())?;
    db_store::DbDocStore::for_project(&url, root.to_path_buf(), &project_slug)
        .map_err(|e| e.to_string())
}

/// Ensures `project_slug` exists before a [`db_store::DbDocStore`] is
/// constructed over it — its constructor only looks an existing project up,
/// it never creates one. Bootstraps via an [`EmptyStore`] rather than
/// [`fs_store::FsStore`] so a first `--backend db` call never silently
/// ingests whatever `.md` files happen to sit under `root`; only ever
/// creates the project shell (never clears an existing one, since it is
/// skipped entirely once found).
async fn prepare_db_project(url: &str, root: &Path, project_slug: &str) -> db_store::Result<()> {
    let conn = db_store::connect(url).await?;
    db_store::migrate(&conn).await?;
    let existing = db_store::list_projects(&conn).await?;
    if existing.iter().any(|project| project.slug == project_slug) {
        return Ok(());
    }
    db_store::sync_project(&conn, &EmptyStore, root, project_slug)
        .await
        .map(|_| ())
}

/// A [`DocStore`] with no records, used only to bootstrap a fresh project
/// row for `--backend db` (via [`db_store::sync_project`]) without
/// ingesting anything from disk.
struct EmptyStore;

impl DocStore for EmptyStore {
    fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    fn read(&self, _path: &Path) -> io::Result<String> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "empty store carries no records",
        ))
    }

    fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
        Ok(())
    }
}

fn run_db_sync(docs_dir: &Path, engine: Engine, project: Option<String>) -> ExitCode {
    let url = match engine.resolve_url() {
        Ok(url) => url,
        Err(err) => return report_failure(&err),
    };
    let project_slug = project.unwrap_or_else(|| derive_project_slug(docs_dir));
    let runtime = match build_runtime() {
        Ok(runtime) => runtime,
        Err(err) => return report_failure(&err.to_string()),
    };
    match runtime.block_on(sync_read_model(docs_dir, &url, &project_slug)) {
        Ok(count) => {
            println!("Indexed {count} records. (project: {project_slug})");
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err.to_string()),
    }
}

async fn sync_read_model(
    docs_dir: &Path,
    url: &str,
    project_slug: &str,
) -> db_store::Result<usize> {
    let conn = db_store::connect(url).await?;
    db_store::migrate(&conn).await?;
    db_store::sync_project(&conn, &fs_store::FsStore::new(), docs_dir, project_slug).await
}

const DEFAULT_PROJECT_SLUG: &str = "default";

/// Derives a stable project slug from `docs_dir`'s own final path
/// component when `--project` is omitted, so repeated syncs of the same
/// bundle land in the same project. Falls back to `"default"` when
/// `docs_dir` has no usable final component (e.g. `.` or `/`).
fn derive_project_slug(docs_dir: &Path) -> String {
    docs_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(paths::slugify)
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| DEFAULT_PROJECT_SLUG.to_owned())
}

fn run_search(query: &str, engine: Engine, project: Option<String>) -> ExitCode {
    let url = match engine.resolve_url() {
        Ok(url) => url,
        Err(err) => return report_failure(&err),
    };
    if is_default_local_sqlite(engine, &url) && !Path::new(SQLITE_READ_MODEL_PATH).exists() {
        eprintln!("no index found at {SQLITE_READ_MODEL_PATH}; run: living-docs db sync");
        return ExitCode::FAILURE;
    }

    let runtime = match build_runtime() {
        Ok(runtime) => runtime,
        Err(err) => return report_failure(&err.to_string()),
    };
    match runtime.block_on(search_read_model(query, &url, project.as_deref())) {
        Ok(hits) => {
            print_hits(&hits);
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err.to_string()),
    }
}

async fn search_read_model(
    query: &str,
    url: &str,
    project: Option<&str>,
) -> db_store::Result<Vec<db_store::SearchHit>> {
    let conn = db_store::connect(url).await?;
    match project {
        Some(slug) => db_store::search_in_project(&conn, query, slug).await,
        None => db_store::search(&conn, query).await,
    }
}

fn print_hits(hits: &[db_store::SearchHit]) {
    for hit in hits {
        println!("[{}] {} — {}", hit.project, hit.path, hit.title);
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
    fn engine_sqlite_resolves_to_the_local_read_model_url_when_database_url_is_unset() {
        let url = Engine::Sqlite
            .resolve_url_with(|_| Err(std::env::VarError::NotPresent))
            .expect("sqlite url always resolves");
        assert_eq!(url, format!("sqlite://{SQLITE_READ_MODEL_PATH}?mode=rwc"));
    }

    #[test]
    fn engine_sqlite_honors_database_url_when_set() {
        let url = Engine::Sqlite
            .resolve_url_with(|_| Ok("sqlite:///tmp/hermetic.db?mode=rwc".to_owned()))
            .expect("sqlite url resolves from the override");
        assert_eq!(url, "sqlite:///tmp/hermetic.db?mode=rwc");
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

    #[test]
    fn derive_project_slug_uses_the_docs_dir_final_component() {
        assert_eq!(derive_project_slug(Path::new("/repo/docs")), "docs");
        assert_eq!(
            derive_project_slug(Path::new("/repo/client-docs")),
            "client-docs"
        );
    }

    #[test]
    fn derive_project_slug_is_stable_across_repeated_calls_on_the_same_dir() {
        let docs_dir = Path::new("/repo/docs");
        assert_eq!(derive_project_slug(docs_dir), derive_project_slug(docs_dir));
    }

    #[test]
    fn derive_project_slug_falls_back_to_default_when_docs_dir_has_no_final_component() {
        assert_eq!(derive_project_slug(Path::new("/")), DEFAULT_PROJECT_SLUG);
        assert_eq!(derive_project_slug(Path::new("")), DEFAULT_PROJECT_SLUG);
    }

    #[test]
    fn check_bundle_uses_docs_dir_for_the_db_backend_ignoring_paths() {
        let bundle = check_bundle(
            Backend::Db,
            Path::new("/repo/docs"),
            vec![PathBuf::from("/ignored")],
        );
        assert_eq!(bundle, PathBuf::from("/repo/docs"));
    }

    #[test]
    fn check_bundle_uses_the_first_path_argument_for_the_fs_backend() {
        let bundle = check_bundle(
            Backend::Fs,
            Path::new("/repo/docs"),
            vec![PathBuf::from("/bundle")],
        );
        assert_eq!(bundle, PathBuf::from("/bundle"));
    }

    #[test]
    fn check_bundle_defaults_to_docs_for_the_fs_backend_when_no_paths_are_given() {
        let bundle = check_bundle(Backend::Fs, Path::new("/repo/docs"), Vec::new());
        assert_eq!(bundle, PathBuf::from("docs"));
    }

    #[test]
    fn is_default_local_sqlite_is_true_for_sqlite_with_the_default_url() {
        assert!(is_default_local_sqlite(
            Engine::Sqlite,
            &default_sqlite_url()
        ));
    }

    #[test]
    fn is_default_local_sqlite_is_false_for_sqlite_with_an_overridden_url() {
        assert!(!is_default_local_sqlite(
            Engine::Sqlite,
            "sqlite:///tmp/hermetic.db?mode=rwc"
        ));
    }

    #[test]
    fn is_default_local_sqlite_is_false_for_paradedb_even_with_the_default_sqlite_url_string() {
        assert!(!is_default_local_sqlite(
            Engine::Paradedb,
            &default_sqlite_url()
        ));
    }

    #[test]
    fn empty_store_lists_no_records_and_refuses_every_read() {
        let store = EmptyStore;
        assert!(store
            .list(Path::new("/bundle"))
            .expect("empty store lists successfully")
            .is_empty());
        assert!(store.read(Path::new("/bundle/adr/0001-x.md")).is_err());
        assert!(store.write(Path::new("/bundle/adr/0001-x.md"), "x").is_ok());
    }
}
