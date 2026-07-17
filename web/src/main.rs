//! `living-docs-web`: connects to the SQLite/FTS5 read-model and serves the
//! read-only search page (ADR 0006, issue 0003 slice S3a). The read-model
//! itself is built by `living-docs db sync`; this binary never migrates or
//! writes it.

use std::env;
use std::net::SocketAddr;

const DEFAULT_DB_PATH: &str = ".living-docs/index.db";
const DEFAULT_PORT: u16 = 3000;

#[tokio::main]
async fn main() {
    let conn = db_store::connect(&sqlite_url(&db_path()))
        .await
        .expect("connect to the read-model database");
    let app = web::build_router(conn);

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
