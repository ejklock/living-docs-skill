//! SeaORM entities for the multi-project schema (ADR 0004 issue 0002 slice
//! S2a; ADR 0005 issue 0005 slice 0005-A). `projects` is the schema root;
//! every other table carries a `project_id` foreign key into it. `records`
//! keeps `path` as the bundle-relative identity, now unique per project
//! rather than globally. `relations` and `tags`/`record_tags` are the
//! normalized link and tagging tables added in slice 0005-A; slice 0005-B
//! populates them from frontmatter. `records` carries the typed
//! `number`/`concept_id`/`identity_kind` identity columns and
//! `frontmatter_fields` is the ordered EAV tail (ADR 0007, issue 0006 slice
//! 0006-A).

pub mod projects {
    use sea_orm::entity::prelude::*;

    /// A single project: the schema root every record, relation, and tag
    /// belongs to.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "projects")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        #[sea_orm(unique)]
        pub slug: String,
        pub name: String,
        pub root_path: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod records {
    use sea_orm::entity::prelude::*;

    /// A single doc record's read-model row: the bundle-relative `path` is
    /// the stable identity for idempotent rebuild, unique within its
    /// project (`UNIQUE(project_id, path)`, not globally). `identity_kind`
    /// discriminates which of `number`/`concept_id` is the record's
    /// identity (ADR 0007 decision 2); this slice populates both fields
    /// from `doc_type` without yet enforcing the XOR. `status` is the
    /// frontmatter `status:` value, `None` when the doc carries no such key
    /// (issue 0008, ADR 0015, S1). `revision` is the optimistic-concurrency
    /// counter that starts at 1 for every record and is bumped on each
    /// authoring write (ADR 0016; the bumping write path lands in issue
    /// 0010 slice 2 — this slice only adds the column and its default).
    /// `deleted_at` is `None` for a live record; `Some(unix_seconds)` marks
    /// it soft-deleted (ADR 0018, issue 0013 slice A) — excluded from the
    /// nav tree, search, and every regenerated `index.md`, but still
    /// readable by a direct query. Stored as a plain Unix-epoch-seconds
    /// `i64` rather than `sea_orm::entity::prelude::DateTimeUtc`: the
    /// latter requires sea-orm's `with-chrono` feature, which this
    /// workspace's `sea-orm` dependency does not enable, so introducing it
    /// would mean a Cargo.toml/Cargo.lock change outside this column's
    /// scope — `i64` mirrors the same crate-local-primitive precedent
    /// `revision` already sets on this struct.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "records")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub project_id: i32,
        pub path: String,
        pub doc_type: String,
        pub number: Option<i32>,
        pub concept_id: Option<String>,
        pub identity_kind: String,
        pub title: String,
        pub description: String,
        pub body: String,
        pub status: Option<String>,
        pub revision: i64,
        pub deleted_at: Option<i64>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod frontmatter_fields {
    use sea_orm::entity::prelude::*;

    /// One frontmatter key/value pair with no typed column, scoped to its
    /// owning record via `record_id` (`ON DELETE CASCADE`). `ordinal`
    /// preserves the field's source encounter order so the tail
    /// reconstructs byte-stably by ascending `ordinal` (ADR 0007 decision
    /// 1, issue 0006 slice 0006-A).
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "frontmatter_fields")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub record_id: i32,
        pub key: String,
        pub value: String,
        pub ordinal: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod relations {
    use sea_orm::entity::prelude::*;

    /// A directed link between two records within the same project (e.g. a
    /// supersede edge), resolved from frontmatter in slice 0005-B.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "relations")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub project_id: i32,
        pub from_record_id: i32,
        pub to_record_id: i32,
        pub kind: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod tags {
    use sea_orm::entity::prelude::*;

    /// A tag name scoped to a project (`UNIQUE(project_id, name)`), linked
    /// to records through `record_tags`.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "tags")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub project_id: i32,
        pub name: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod record_tags {
    use sea_orm::entity::prelude::*;

    /// The many-to-many join between `records` and `tags`, keyed on the
    /// pair itself.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "record_tags")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub record_id: i32,
        #[sea_orm(primary_key, auto_increment = false)]
        pub tag_id: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub use records::{ActiveModel, Column, Entity, Model, Relation};
