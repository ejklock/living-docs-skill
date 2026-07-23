//! `living-docs-web`: connects to the SQLite/FTS5 read-model and serves the
//! read-only search page (ADR 0006, issue 0003 slice S3a), plus — with
//! `--backend db` — Atlas's authoring create route (ADR 0016, issue 0010
//! slice 3). The read-model itself is built by `living-docs db sync`; this
//! binary never migrates or writes it outside `POST /new`'s own
//! `write_checked` call.

use std::env;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

const DEFAULT_DB_PATH: &str = ".living-docs/index.db";
const DEFAULT_PORT: u16 = 3000;
const DEFAULT_DOCS_DIR: &str = "docs";

#[tokio::main]
async fn main() {
    let args = Args::parse(env::args().skip(1));
    let db_path = db_path();
    let conn = db_store::connect(&sqlite_url(&db_path))
        .await
        .expect("connect to the read-model database");
    let authoring = authoring_config(args.backend, &db_path, &args.docs_dir);
    let app = web::build_router(conn, authoring);

    let addr = SocketAddr::from(([127, 0, 0, 1], port()));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind server address");
    println!("living-docs-web listening on http://{addr}");
    axum::serve(listener, app)
        .await
        .expect("serve the axum app");
}

fn db_path() -> String {
    env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DB_PATH.to_owned())
}

fn sqlite_url(path: &str) -> String {
    format!("sqlite://{path}?mode=rwc")
}

fn port() -> u16 {
    env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

/// Which persistence backend gates whether `POST /new`/`GET /new` are
/// registered at all (ADR 0016, issue 0010 slice 3): `Fs` (default,
/// mirroring the CLI's own `--backend` default) never mounts them; `Db`
/// mounts them, scoped to the resolved connection URL and docs root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Backend {
    Fs,
    Db,
}

/// This binary's own parsed `--backend`/`--docs-dir` flags — a minimal
/// manual scan (this binary carries no `clap` dependency, unlike the CLI)
/// mirroring the CLI's own flag names and defaults (`fs`, `docs`).
struct Args {
    backend: Backend,
    docs_dir: PathBuf,
}

impl Args {
    fn parse<I: Iterator<Item = String>>(args: I) -> Self {
        let mut backend = Backend::Fs;
        let mut docs_dir = PathBuf::from(DEFAULT_DOCS_DIR);
        let mut iter = args;
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--backend" => {
                    backend = iter.next().as_deref().map(parse_backend).unwrap_or(backend);
                }
                "--docs-dir" => {
                    docs_dir = iter.next().map(PathBuf::from).unwrap_or(docs_dir);
                }
                _ => {}
            }
        }
        Self { backend, docs_dir }
    }
}

fn parse_backend(value: &str) -> Backend {
    match value {
        "db" => Backend::Db,
        _ => Backend::Fs,
    }
}

/// Builds Atlas's [`web::AuthoringConfig`] for `--backend db`, reusing the
/// same connection URL this binary already computes for its read-model
/// connection (`sqlite_url(db_path)`) — `None` for `--backend fs`, which is
/// what keeps `/new` unregistered in `web::build_router`.
fn authoring_config(
    backend: Backend,
    db_path: &str,
    docs_dir: &Path,
) -> Option<web::AuthoringConfig> {
    (backend == Backend::Db).then(|| web::AuthoringConfig {
        db_url: sqlite_url(db_path),
        docs_root: docs_dir.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_parse_defaults_to_fs_backend_and_the_docs_directory() {
        let args = Args::parse(std::iter::empty());

        assert_eq!(args.backend, Backend::Fs);
        assert_eq!(args.docs_dir, PathBuf::from(DEFAULT_DOCS_DIR));
    }

    #[test]
    fn args_parse_reads_backend_and_docs_dir_flags() {
        let args = Args::parse(
            ["--backend", "db", "--docs-dir", "client-docs"]
                .into_iter()
                .map(str::to_owned),
        );

        assert_eq!(args.backend, Backend::Db);
        assert_eq!(args.docs_dir, PathBuf::from("client-docs"));
    }

    #[test]
    fn args_parse_ignores_an_unknown_backend_value_keeping_the_fs_default() {
        let args = Args::parse(["--backend", "bogus"].into_iter().map(str::to_owned));

        assert_eq!(args.backend, Backend::Fs);
    }

    #[test]
    fn args_parse_ignores_a_dangling_flag_with_no_following_value() {
        let args = Args::parse(["--backend"].into_iter().map(str::to_owned));

        assert_eq!(args.backend, Backend::Fs);
    }

    #[test]
    fn authoring_config_is_none_for_the_fs_backend() {
        assert!(
            authoring_config(Backend::Fs, ".living-docs/index.db", Path::new("docs")).is_none()
        );
    }

    #[test]
    fn authoring_config_carries_the_resolved_db_url_and_docs_root_for_the_db_backend() {
        let config = authoring_config(
            Backend::Db,
            ".living-docs/index.db",
            Path::new("client-docs"),
        )
        .expect("db backend must configure authoring");

        assert_eq!(config.db_url, "sqlite://.living-docs/index.db?mode=rwc");
        assert_eq!(config.docs_root, PathBuf::from("client-docs"));
    }
}
