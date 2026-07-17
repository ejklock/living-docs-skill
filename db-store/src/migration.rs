//! Schema migration for the `records` read-model, branched per backend (ADR
//! 0004, issue 0002 slice S2a; ParadeDB branch issue 0004 slice 0004-B):
//! SQLite gets the `records_fts` FTS5 external-content virtual table,
//! Postgres gets a `pg_search` `records_bm25` BM25 index.

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

/// The crate's single migration source, applied by [`crate::migrate`].
pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(CreateRecords)]
    }
}

struct CreateRecords;

impl MigrationName for CreateRecords {
    fn name(&self) -> &str {
        "m20260716_000001_create_records"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateRecords {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Records::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Records::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Records::Path).text().not_null().unique_key())
                    .col(
                        ColumnDef::new(Records::DocType)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(Records::Identity).text())
                    .col(ColumnDef::new(Records::Title).text().not_null().default(""))
                    .col(
                        ColumnDef::new(Records::Description)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(Records::Body).text().not_null().default(""))
                    .to_owned(),
            )
            .await?;

        create_search_index(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_search_index(manager).await?;
        manager
            .drop_table(Table::drop().table(Records::Table).to_owned())
            .await
    }
}

/// Creates the backend-native full-text index over `records`: an FTS5
/// external-content virtual table on SQLite, a `pg_search` BM25 index on
/// Postgres.
async fn create_search_index(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let connection = manager.get_connection();
    match manager.get_database_backend() {
        DbBackend::Postgres => {
            connection
                .execute_unprepared("CREATE EXTENSION IF NOT EXISTS pg_search")
                .await?;
            connection
                .execute_unprepared(
                    "CREATE INDEX records_bm25 ON records USING bm25 (id, title, description, \
                     body) WITH (key_field='id')",
                )
                .await
                .map(|_| ())
        }
        DbBackend::Sqlite => connection
            .execute_unprepared(
                "CREATE VIRTUAL TABLE records_fts USING fts5(title, description, body, \
                 content='records', content_rowid='id')",
            )
            .await
            .map(|_| ()),
        DbBackend::MySql => Err(unsupported_backend_err()),
    }
}

/// Reverses [`create_search_index`] per backend.
async fn drop_search_index(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let connection = manager.get_connection();
    match manager.get_database_backend() {
        DbBackend::Postgres => connection
            .execute_unprepared("DROP INDEX IF EXISTS records_bm25")
            .await
            .map(|_| ()),
        DbBackend::Sqlite => connection
            .execute_unprepared("DROP TABLE IF EXISTS records_fts")
            .await
            .map(|_| ()),
        DbBackend::MySql => Err(unsupported_backend_err()),
    }
}

/// The error returned when a migration runs against a backend this crate
/// does not support (only SQLite and Postgres are compiled in).
fn unsupported_backend_err() -> DbErr {
    DbErr::Custom("db-store only supports the sqlite and postgres backends".to_owned())
}

#[derive(DeriveIden)]
enum Records {
    Table,
    Id,
    Path,
    DocType,
    Identity,
    Title,
    Description,
    Body,
}
