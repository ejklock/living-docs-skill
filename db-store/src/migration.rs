//! Schema migration for the SQLite/FTS5 read-model (ADR 0004, issue 0002
//! slice S2a): the `records` table and the `records_fts` FTS5 external-content
//! virtual table over it.

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

        manager
            .get_connection()
            .execute_unprepared(
                "CREATE VIRTUAL TABLE records_fts USING fts5(title, description, body, \
                 content='records', content_rowid='id')",
            )
            .await
            .map(|_| ())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS records_fts")
            .await?;
        manager
            .drop_table(Table::drop().table(Records::Table).to_owned())
            .await
    }
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
