//! Schema migrations for the multi-project read-model, branched per backend
//! (ADR 0004, issue 0002 slice S2a; ParadeDB branch issue 0004 slice
//! 0004-B; multi-project schema issue 0005 slice 0005-A): SQLite gets the
//! `records_fts` FTS5 external-content virtual table, Postgres gets a
//! `pg_search` `records_bm25` BM25 index.

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

/// The crate's migration source, applied in order by [`crate::migrate`].
pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(CreateRecords),
            Box::new(CreateMultiProjectSchema),
            Box::new(CreateAuthoringSchema),
        ]
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

/// Recreates `records` with a `project_id` foreign key into a new `projects`
/// root table, replacing the old global-unique `path` with
/// `UNIQUE(project_id, path)`, and adds the `relations`/`tags`/`record_tags`
/// tables with foreign-key constraints (ADR 0005, issue 0005 slice 0005-A).
/// The `records` recreation is destructive by design: the table is a derived
/// read-model, rebuilt in full by [`crate::sync::sync`], so there is no data
/// to preserve across the shape change.
struct CreateMultiProjectSchema;

impl MigrationName for CreateMultiProjectSchema {
    fn name(&self) -> &str {
        "m20260717_000002_create_multi_project_schema"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateMultiProjectSchema {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_projects_table(manager).await?;
        drop_search_index(manager).await?;
        manager
            .drop_table(Table::drop().table(Records::Table).to_owned())
            .await?;
        create_records_table(manager).await?;
        create_search_index(manager).await?;
        create_relations_table(manager).await?;
        create_tags_table(manager).await?;
        create_record_tags_table(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RecordTags::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Tags::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Relations::Table).to_owned())
            .await?;
        drop_search_index(manager).await?;
        manager
            .drop_table(Table::drop().table(Records::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Projects::Table).to_owned())
            .await?;
        CreateRecords.up(manager).await
    }
}

/// Replaces `records.identity` with typed `number`/`concept_id` columns
/// plus a non-null `identity_kind` discriminator, and adds the ordered
/// `frontmatter_fields` EAV tail table (ADR 0007, issue 0006 slice 0006-A).
/// Like [`CreateMultiProjectSchema`], the `records` recreation is
/// destructive by design: the table is a derived read-model, rebuilt in
/// full by [`crate::sync::sync`].
struct CreateAuthoringSchema;

impl MigrationName for CreateAuthoringSchema {
    fn name(&self) -> &str {
        "m20260717_000003_create_authoring_schema"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateAuthoringSchema {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_search_index(manager).await?;
        manager
            .drop_table(Table::drop().table(Records::Table).to_owned())
            .await?;
        create_authoring_records_table(manager).await?;
        create_search_index(manager).await?;
        create_frontmatter_fields_table(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FrontmatterFields::Table).to_owned())
            .await?;
        drop_search_index(manager).await?;
        manager
            .drop_table(Table::drop().table(Records::Table).to_owned())
            .await?;
        create_records_table(manager).await?;
        create_search_index(manager).await
    }
}

/// Creates `records` with the typed `number`/`concept_id`/`identity_kind`
/// identity columns in place of the single polymorphic `identity` column,
/// keeping the same `project_id` foreign key and `UNIQUE(project_id, path)`
/// index as [`create_records_table`].
async fn create_authoring_records_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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
                .col(ColumnDef::new(Records::ProjectId).integer().not_null())
                .col(ColumnDef::new(Records::Path).text().not_null())
                .col(
                    ColumnDef::new(Records::DocType)
                        .text()
                        .not_null()
                        .default(""),
                )
                .col(ColumnDef::new(Records::Number).integer())
                .col(ColumnDef::new(Records::ConceptId).text())
                .col(
                    ColumnDef::new(Records::IdentityKind)
                        .text()
                        .not_null()
                        .default(""),
                )
                .col(ColumnDef::new(Records::Title).text().not_null().default(""))
                .col(
                    ColumnDef::new(Records::Description)
                        .text()
                        .not_null()
                        .default(""),
                )
                .col(ColumnDef::new(Records::Body).text().not_null().default(""))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_records_project")
                        .from(Records::Table, Records::ProjectId)
                        .to(Projects::Table, Projects::Id),
                )
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("idx_records_project_path")
                .table(Records::Table)
                .col(Records::ProjectId)
                .col(Records::Path)
                .unique()
                .to_owned(),
        )
        .await
}

/// Creates the ordered EAV frontmatter tail: one row per non-typed
/// frontmatter key, scoped to its record via `record_id` and cascaded on
/// the record's delete so a record's tail never outlives it.
async fn create_frontmatter_fields_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(FrontmatterFields::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(FrontmatterFields::Id)
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(FrontmatterFields::RecordId)
                        .integer()
                        .not_null(),
                )
                .col(ColumnDef::new(FrontmatterFields::Key).text().not_null())
                .col(ColumnDef::new(FrontmatterFields::Value).text().not_null())
                .col(
                    ColumnDef::new(FrontmatterFields::Ordinal)
                        .integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_frontmatter_fields_record")
                        .from(FrontmatterFields::Table, FrontmatterFields::RecordId)
                        .to(Records::Table, Records::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
}

async fn create_projects_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Projects::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Projects::Id)
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(Projects::Slug)
                        .text()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(Projects::Name).text().not_null().default(""))
                .col(ColumnDef::new(Projects::RootPath).text())
                .to_owned(),
        )
        .await
}

/// Creates `records` with a `project_id` foreign key into `projects` and a
/// `UNIQUE(project_id, path)` index, replacing the single-project global
/// unique on `path`.
async fn create_records_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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
                .col(ColumnDef::new(Records::ProjectId).integer().not_null())
                .col(ColumnDef::new(Records::Path).text().not_null())
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
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_records_project")
                        .from(Records::Table, Records::ProjectId)
                        .to(Projects::Table, Projects::Id),
                )
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("idx_records_project_path")
                .table(Records::Table)
                .col(Records::ProjectId)
                .col(Records::Path)
                .unique()
                .to_owned(),
        )
        .await
}

async fn create_relations_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Relations::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Relations::Id)
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(ColumnDef::new(Relations::ProjectId).integer().not_null())
                .col(ColumnDef::new(Relations::FromRecordId).integer().not_null())
                .col(ColumnDef::new(Relations::ToRecordId).integer().not_null())
                .col(ColumnDef::new(Relations::Kind).text().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_relations_project")
                        .from(Relations::Table, Relations::ProjectId)
                        .to(Projects::Table, Projects::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_relations_from_record")
                        .from(Relations::Table, Relations::FromRecordId)
                        .to(Records::Table, Records::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_relations_to_record")
                        .from(Relations::Table, Relations::ToRecordId)
                        .to(Records::Table, Records::Id),
                )
                .to_owned(),
        )
        .await
}

async fn create_tags_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Tags::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Tags::Id)
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(ColumnDef::new(Tags::ProjectId).integer().not_null())
                .col(ColumnDef::new(Tags::Name).text().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_tags_project")
                        .from(Tags::Table, Tags::ProjectId)
                        .to(Projects::Table, Projects::Id),
                )
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("idx_tags_project_name")
                .table(Tags::Table)
                .col(Tags::ProjectId)
                .col(Tags::Name)
                .unique()
                .to_owned(),
        )
        .await
}

async fn create_record_tags_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(RecordTags::Table)
                .if_not_exists()
                .col(ColumnDef::new(RecordTags::RecordId).integer().not_null())
                .col(ColumnDef::new(RecordTags::TagId).integer().not_null())
                .primary_key(
                    Index::create()
                        .col(RecordTags::RecordId)
                        .col(RecordTags::TagId),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_record_tags_record")
                        .from(RecordTags::Table, RecordTags::RecordId)
                        .to(Records::Table, Records::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_record_tags_tag")
                        .from(RecordTags::Table, RecordTags::TagId)
                        .to(Tags::Table, Tags::Id),
                )
                .to_owned(),
        )
        .await
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
    ProjectId,
    Path,
    DocType,
    Identity,
    Number,
    ConceptId,
    IdentityKind,
    Title,
    Description,
    Body,
}

#[derive(DeriveIden)]
enum FrontmatterFields {
    Table,
    Id,
    RecordId,
    Key,
    Value,
    Ordinal,
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
    Slug,
    Name,
    RootPath,
}

#[derive(DeriveIden)]
enum Relations {
    Table,
    Id,
    ProjectId,
    FromRecordId,
    ToRecordId,
    Kind,
}

#[derive(DeriveIden)]
enum Tags {
    Table,
    Id,
    ProjectId,
    Name,
}

#[derive(DeriveIden)]
enum RecordTags {
    Table,
    RecordId,
    TagId,
}
