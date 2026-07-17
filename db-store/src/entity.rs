//! SeaORM entity for the `records` table (ADR 0004, issue 0002 slice S2a
//! subset of the ADR 0005 normalized schema). `path` is the bundle-relative
//! path and the stable identity used for idempotent rebuild in slice S2b.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
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
