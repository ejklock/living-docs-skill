//! The typed arguments of each tool; the serde tag `operation` selects the variant.

use schemars::JsonSchema;
use serde::Deserialize;

/// Create or change a record: `new`, `set` or `supersede`.
#[derive(Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "snake_case")]
#[schemars(extend("type" = "object"))]
pub enum WriteArgs {
    /// Scaffold a record of a type and refresh its index.
    New(NewArgs),
    /// Set one field of a record, or replace one body section of it.
    Set(SetArgs),
    /// Retire one record in favor of another and link both.
    Supersede(SupersedeArgs),
}

#[derive(Deserialize, JsonSchema)]
pub struct NewArgs {
    /// The record type token, such as `adr` or `issue`.
    #[serde(rename = "type")]
    pub doc_type: String,
    pub title: String,
    /// The one-sentence description for the frontmatter.
    pub description: Option<String>,
    /// The kind, for the types that take one.
    pub kind: Option<String>,
    /// The owner for the frontmatter. When omitted on a type that requires an owner, the server seeds the git user.name, else user.email, of the bundle's repository.
    pub owner: Option<String>,
    /// The prose below the title heading; frontmatter is never part of it.
    pub body: Option<String>,
}

/// Field form: `reference`, `key`, `value`. Section form: `reference`, `heading`, `text`.
#[derive(Deserialize, JsonSchema)]
pub struct SetArgs {
    /// A record reference: `TYPE/NNNN`, a bare `NNNN`, or a path inside the bundle.
    pub reference: String,
    /// The frontmatter field to set, such as `status` or `description`.
    pub key: Option<String>,
    /// The new value of the field.
    pub value: Option<String>,
    /// The body section heading to replace or append.
    pub heading: Option<String>,
    /// The section text, as prose.
    pub text: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct SupersedeArgs {
    /// The record being retired.
    pub old: String,
    /// The record that replaces it.
    pub new: String,
}

/// Read the bundle: `records`.
#[derive(Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "snake_case")]
#[schemars(extend("type" = "object"))]
pub enum ReadArgs {
    /// The in-force records: active only, supersede chains collapsed to the head.
    Records(RecordsArgs),
}

#[derive(Deserialize, JsonSchema)]
pub struct RecordsArgs {
    /// Keep records where every term starts a word of the title, description or body.
    pub topic: Option<String>,
    /// Return each record's full body.
    #[serde(default)]
    pub full: bool,
    /// Return each body without its detail-tier sections.
    #[serde(default)]
    pub contract: bool,
    /// Only the live records whose body links this record (`TYPE/NNNN`).
    pub links_to: Option<String>,
    /// Every live record that reaches this record (`TYPE/NNNN`) through links, nearest first.
    pub impact: Option<String>,
}

/// Maintain the bundle: `index` or `fmt`.
#[derive(Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "snake_case")]
#[schemars(extend("type" = "object"))]
pub enum ManageArgs {
    /// Regenerate the index of one type, or of every type.
    Index(IndexArgs),
    /// Canonicalize record frontmatter, or only report what would change.
    Fmt(FmtArgs),
}

#[derive(Deserialize, JsonSchema)]
pub struct IndexArgs {
    /// The record type token; omitted regenerates every type.
    #[serde(rename = "type")]
    pub doc_type: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct FmtArgs {
    /// Report the records that would change without writing any.
    #[serde(default)]
    pub check_only: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct CheckArgs {
    /// Also require an owner on the types that carry one.
    #[serde(default)]
    pub require_owner: bool,
}
