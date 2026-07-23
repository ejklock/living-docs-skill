use clap::{Parser, Subcommand, ValueEnum};
use living_docs_core::store::DocStore;
use living_docs_core::{check, commands, paths};
use std::io;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod skill;

#[derive(Parser)]
#[command(
    name = "living-docs",
    version,
    about = "Deterministic layer of Living Docs authoring. Write ONLY the body below the closing ---. Frontmatter and indexes are CLI-owned: `living-docs status` / `supersede` / `index`."
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

    /// Which database engine `db sync`/`search`, and any `--backend db`
    /// authoring command, connects to: ParadeDB via `$DATABASE_URL` (the
    /// default, ADR 0004) or the local embedded SQLite/FTS5 file (`sqlite`,
    /// opt-in, falling back to `.living-docs/index.db` when `$DATABASE_URL`
    /// is unset).
    #[arg(long, global = true, value_enum, default_value = "paradedb")]
    engine: Engine,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    New {
        doc_type: String,
        title: String,
    },
    /// `new` plus deterministic pre-fill (issue 0008): frontmatter title,
    /// numbered title heading, a trail comment, and every judgment section
    /// collapsed to a marked empty `<!-- judgment: ... -->` slot the
    /// authoring model fills.
    Brief {
        doc_type: String,
        title: String,
        /// Git range (e.g. HEAD~3..HEAD) whose touched files are listed —
        /// verbatim from `git diff --name-only` — under the context slot.
        #[arg(long)]
        from_diff: Option<String>,
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
    /// Sets a record's `status:` frontmatter field directly — for the
    /// `Proposed`/`Accepted`/`Deprecated` lifecycle only. `Superseded` is
    /// rejected; use `supersede`, which also wires the
    /// `supersedes`/`superseded_by` links.
    Status {
        number: String,
        new_status: String,
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
    /// Canonicalizes every concept record's frontmatter in place — the
    /// remediation verb for `check`'s canonical-frontmatter invariant (ADR
    /// 0019). Matches `check`'s own `[BUNDLE_ROOT]` argument rather than the
    /// global `--docs-dir`; fs-backend only, since db-mode is canonical by
    /// construction on export.
    Fmt {
        paths: Vec<PathBuf>,
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
    /// Operate on the derived read-model — ParadeDB via `$DATABASE_URL` by
    /// default (ADR 0004), or the local embedded SQLite/FTS5 file with
    /// `--engine sqlite`.
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
        /// Narrow results to one project's slug. Omitted spans every
        /// project, labeling each hit by the project it belongs to (ADR
        /// 0005, issue 0005 slice 0005-C1).
        #[arg(long)]
        project: Option<String>,
    },
    /// Serves skill content embedded in the binary at compile time (ADR
    /// 0014): list embedded skills and their topics, print a skill's full
    /// `SKILL.md` body, or print one topic's detail.
    Skill {
        /// The skill to query, e.g. `living-docs`. Required unless `--list`.
        name: Option<String>,
        /// Print only this topic's detail instead of the full `SKILL.md`
        /// body; maps to a `rules/`/`templates/` basename.
        #[arg(long)]
        topic: Option<String>,
        /// List every embedded skill and its available topics instead of
        /// printing a single skill's content.
        #[arg(long)]
        list: bool,
        /// Emit minified single-line JSON instead of plain text, for
        /// consumption by other agents. Only changes the success-output
        /// shape; errors still print to stderr as plain text. Overrides TTY
        /// autodetection; mutually exclusive with `--plain`.
        #[arg(long)]
        json: bool,
        /// Force human-readable plain text, overriding TTY autodetection.
        /// Mutually exclusive with `--json`.
        #[arg(long, conflicts_with = "json")]
        plain: bool,
    },
}

#[derive(Subcommand)]
enum DbCmd {
    /// Rebuild the read-model from every doc `--docs-dir` lists, scoped to
    /// one named project (ADR 0005, issue 0005 slice 0005-B).
    Sync {
        /// The project slug to sync into. Defaults to a slug derived from
        /// `--docs-dir`'s own directory name, keeping single-project usage
        /// working without naming a project explicitly.
        #[arg(long)]
        project: Option<String>,
    },
}

/// The database backend to connect to, selectable via the global `--engine`
/// flag (ADR 0004, issue 0004). `Paradedb` is the default, requiring
/// `$DATABASE_URL`; `Sqlite` is opt-in and falls back to the local embedded
/// read-model when `$DATABASE_URL` is unset.
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
        Command::New { doc_type, title } => {
            run_new(cli.backend, cli.engine, &cli.docs_dir, &doc_type, &title)
        }
        Command::Brief {
            doc_type,
            title,
            from_diff,
        } => run_brief(
            cli.backend,
            cli.engine,
            &cli.docs_dir,
            &doc_type,
            &title,
            from_diff,
        ),
        Command::Index {
            doc_type,
            visibility,
        } => run_index(cli.backend, cli.engine, &cli.docs_dir, doc_type, visibility),
        Command::Supersede { old, new } => {
            run_supersede(cli.backend, cli.engine, &cli.docs_dir, &old, &new)
        }
        Command::Status { number, new_status } => {
            run_status(cli.backend, cli.engine, &cli.docs_dir, &number, &new_status)
        }
        Command::Check {
            paths,
            mermaid_only,
        } if mermaid_only => check::run_mermaid_only(&paths),
        Command::Check { paths, .. } => run_check(cli.backend, cli.engine, &cli.docs_dir, paths),
        Command::Fmt { paths } => run_fmt(&cli.docs_dir, paths),
        Command::Export {
            out_dir,
            visibility,
        } => run_export(cli.backend, cli.engine, &cli.docs_dir, &out_dir, visibility),
        Command::LeakGate {
            bundle,
            check_tier3,
        } => run_leak_gate(&bundle, check_tier3),
        Command::Db {
            cmd: DbCmd::Sync { project },
        } => run_db_sync(&cli.docs_dir, cli.engine, project),
        Command::Search { query, project } => run_search(&query, cli.engine, project),
        Command::Skill {
            name,
            topic,
            list,
            json,
            plain,
        } => run_skill(name, topic, list, json, plain),
    }
}

fn run_new(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
) -> ExitCode {
    match backend {
        Backend::Fs => match build_backend_store(backend, engine, docs_dir) {
            Ok(store) => commands::new::run(store.as_ref(), docs_dir, doc_type, title),
            Err(err) => report_failure(&err),
        },
        Backend::Db => run_new_db(engine, docs_dir, doc_type, title),
    }
}

/// `--backend db new`'s own path: unlike `Backend::Fs` (which delegates
/// straight to [`commands::new::run`]'s plain [`living_docs_core::store::DocStore::write`]),
/// db-mode plans the target path with [`commands::new::plan`] and commits it
/// through [`db_store::DbDocStore::write_checked`], so an invalid record is
/// rejected before it is ever visible (ADR 0016, issue 0010 slice 2).
fn run_new_db(engine: Engine, docs_dir: &Path, doc_type: &str, title: &str) -> ExitCode {
    let store = match build_db_doc_store(engine, docs_dir) {
        Ok(store) => store,
        Err(err) => return report_failure(&err),
    };
    match commands::new::plan(&store, docs_dir, doc_type, title) {
        Ok((target_path, filled)) => commit_new_db(&store, &target_path, &filled),
        Err(err) => report_new_db_failure(&err),
    }
}

fn commit_new_db(store: &db_store::DbDocStore, target_path: &Path, filled: &str) -> ExitCode {
    match store.write_checked(target_path, filled) {
        Ok(_) => {
            println!("{}", target_path.display());
            println!("{}", commands::new::BODY_ONLY_INSTRUCTION);
            ExitCode::SUCCESS
        }
        Err(err) => report_new_db_failure(&err.to_string()),
    }
}

/// Mirrors [`commands::new::run`]'s own failure wording exactly, so
/// db-mode's `plan`/`write_checked` errors print and exit identically to
/// fs-mode's `scaffold` errors — the only new outcome db-mode can now reach
/// that fs-mode never could is a failing `check` from `write_checked`.
fn report_new_db_failure(message: &str) -> ExitCode {
    eprintln!("living-docs new: {message}");
    ExitCode::from(2)
}

fn run_brief(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    from_diff: Option<String>,
) -> ExitCode {
    let diff = match from_diff.map(|range| resolve_diff(&range)).transpose() {
        Ok(diff) => diff,
        Err(err) => return report_failure(&err),
    };
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => commands::brief::run(store.as_ref(), docs_dir, doc_type, title, diff.as_ref()),
        Err(err) => report_failure(&err),
    }
}

/// Resolves `--from-diff` in the front so `living-docs-core` stays I/O-free:
/// the touched-file list is exactly `git diff --name-only <range>` against
/// the current working directory's repository.
fn resolve_diff(range: &str) -> Result<commands::brief::DiffContext, String> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", range])
        .output()
        .map_err(|e| format!("failed to run git diff --name-only {range}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git diff --name-only {range} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    Ok(commands::brief::DiffContext {
        range: range.to_string(),
        files,
    })
}

fn run_index(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    doc_type: Option<String>,
    visibility: Option<Vec<String>>,
) -> ExitCode {
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => commands::index::run(store.as_ref(), docs_dir, doc_type, visibility),
        Err(err) => report_failure(&err),
    }
}

fn run_supersede(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    old: &str,
    new: &str,
) -> ExitCode {
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => commands::supersede::run(store.as_ref(), docs_dir, old, new),
        Err(err) => report_failure(&err),
    }
}

fn run_status(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    number: &str,
    new_status: &str,
) -> ExitCode {
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => commands::status::run(store.as_ref(), docs_dir, number, new_status),
        Err(err) => report_failure(&err),
    }
}

fn run_check(backend: Backend, engine: Engine, docs_dir: &Path, paths: Vec<PathBuf>) -> ExitCode {
    let bundle = check_bundle(backend, docs_dir, paths);
    match build_backend_store(backend, engine, &bundle) {
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

/// `fmt` is fs-backend only (db-mode is canonical by construction on
/// export), so it needs no `build_backend_store`/`Engine` plumbing — it
/// reuses [`check_bundle`]'s `[BUNDLE_ROOT]` resolution against a fixed
/// [`fs_store::FsStore`], the same way [`run_leak_gate`] always inspects a
/// materialized filesystem bundle regardless of `--backend`.
fn run_fmt(docs_dir: &Path, paths: Vec<PathBuf>) -> ExitCode {
    let bundle = check_bundle(Backend::Fs, docs_dir, paths);
    commands::fmt::run(&fs_store::FsStore::new(), &bundle)
}

fn run_export(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    out_dir: &Path,
    visibility: Option<Vec<String>>,
) -> ExitCode {
    match build_backend_store(backend, engine, docs_dir) {
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

fn build_backend_store(
    backend: Backend,
    engine: Engine,
    root: &Path,
) -> Result<Box<dyn DocStore>, String> {
    match backend {
        Backend::Fs => Ok(Box::new(fs_store::FsStore::new())),
        Backend::Db => {
            build_db_doc_store(engine, root).map(|store| Box::new(store) as Box<dyn DocStore>)
        }
    }
}

/// Opens (migrating if needed) the db backend's connection, bootstraps
/// `root`'s project if this is its first use, then hands back a
/// [`db_store::DbDocStore`] scoped to it — the `--backend db` counterpart
/// of [`fs_store::FsStore::new`]. `engine` resolves the connection string
/// exactly as `db sync`/`search` do (ADR 0004: ParadeDB default, SQLite
/// opt-in), so `--backend db` authoring honors the same `--engine` choice.
fn build_db_doc_store(engine: Engine, root: &Path) -> Result<db_store::DbDocStore, String> {
    let url = engine.resolve_url()?;
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

/// The resolved output shape for `skill`'s success path (ADR 0014, "output
/// format is context-aware"). Errors are unaffected — they always print
/// plain text to stderr regardless of mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputMode {
    Plain,
    Json,
}

/// Resolves `skill`'s effective [`OutputMode`]: `json`/`plain` are explicit
/// overrides and always win over autodetection, `json` taking precedence if
/// both were somehow set (clap's `conflicts_with` already rejects that
/// combination before this runs). With neither flag given, `is_tty` decides
/// — a TTY (interactive human) defaults to plain text, anything else
/// (piped, an agent consuming the output) defaults to JSON. `is_tty` is a
/// plain parameter, not a live syscall, so this stays unit-testable without
/// a real terminal.
fn resolve_skill_output(json: bool, plain: bool, is_tty: bool) -> OutputMode {
    if json {
        return OutputMode::Json;
    }
    if plain {
        return OutputMode::Plain;
    }
    if is_tty {
        OutputMode::Plain
    } else {
        OutputMode::Json
    }
}

/// `--list` takes priority over `name`/`topic`; otherwise `name` is
/// required and `topic`, when given, narrows the body to one topic's
/// detail (ADR 0014). The resolved [`OutputMode`] swaps every branch's
/// plain-text renderer for its minified-JSON counterpart without changing
/// the selection logic or error handling (errors always stay plain text on
/// stderr).
fn run_skill(
    name: Option<String>,
    topic: Option<String>,
    list: bool,
    json: bool,
    plain: bool,
) -> ExitCode {
    let mode = resolve_skill_output(json, plain, std::io::stdout().is_terminal());
    let as_json = mode == OutputMode::Json;
    if list {
        return print_skill_result(if as_json {
            skill::list_json()
        } else {
            skill::list()
        });
    }
    let Some(name) = name else {
        return report_failure("skill: NAME is required unless --list is given");
    };
    match topic {
        Some(topic) => print_skill_result(if as_json {
            skill::topic_json(&name, &topic)
        } else {
            skill::topic(&name, &topic)
        }),
        None => print_skill_result(if as_json {
            skill::body_json(&name)
        } else {
            skill::body(&name)
        }),
    }
}

fn print_skill_result(result: Result<String, String>) -> ExitCode {
    match result {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err),
    }
}

fn report_failure(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn root_help_about_carries_the_same_body_only_instruction_new_prints() {
        let about = Cli::command()
            .get_about()
            .expect("the root command carries an about string")
            .to_string();
        assert!(
            about.contains(commands::new::BODY_ONLY_INSTRUCTION),
            "got: {about}"
        );
    }

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
    fn resolve_skill_output_json_flag_wins_regardless_of_tty() {
        assert_eq!(resolve_skill_output(true, false, true), OutputMode::Json);
        assert_eq!(resolve_skill_output(true, false, false), OutputMode::Json);
    }

    #[test]
    fn resolve_skill_output_plain_flag_wins_regardless_of_tty() {
        assert_eq!(resolve_skill_output(false, true, true), OutputMode::Plain);
        assert_eq!(resolve_skill_output(false, true, false), OutputMode::Plain);
    }

    #[test]
    fn resolve_skill_output_defaults_to_json_when_stdout_is_not_a_tty() {
        assert_eq!(resolve_skill_output(false, false, false), OutputMode::Json);
    }

    #[test]
    fn resolve_skill_output_defaults_to_plain_when_stdout_is_a_tty() {
        assert_eq!(resolve_skill_output(false, false, true), OutputMode::Plain);
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
