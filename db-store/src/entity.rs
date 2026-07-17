//! SeaORM entities for the multi-project schema (ADR 0004 issue 0002 slice
//! S2a; ADR 0005 issue 0005 slice 0005-A). `projects` is the schema root;
//! every other table carries a `project_id` foreign key into it. `records`
//! keeps `path` as the bundle-relative identity, now unique per project
//! rather than globally. `relations` and `tags`/`record_tags` are the
//! normalized link and tagging tables added in slice 0005-A; slice 0005-B
//! populates them from frontmatter.

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
    /// project (`UNIQUE(project_id, path)`, not globally).
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "records")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub project_id: i32,
        pub path: String,
        pub doc_type: String,
        pub identity: Option<String>,
        pub title: String,
        pub description: String,
        pub body: String,
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
