//! The records in force, served as `record://` resources.
//!
//! A record's URI is its reference as `write set` accepts it (`record://adr/0108`,
//! `record://view/crates`, `record://constitution`); a record with no number
//! or slug identity uses its bundle-relative path, each segment
//! percent-encoded. A requested URI is matched against the URIs of the
//! records that exist, never turned into a path, so a `..` segment, an
//! encoded escape or an unknown record can only miss. Files are read through
//! the rooted store, which refuses a link leaving the bundle.

use fs_store::FsStore;
use living_docs_core::commands::in_force::in_force_records;
use living_docs_core::doc_type::{self, DocTypeSpec, Identity};
use living_docs_core::record::ExtractedRecord;
use living_docs_core::store::DocStore;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use rmcp::model::{ReadResourceResult, Resource, ResourceContents};
use rmcp::ErrorData;
use std::path::Path;

pub(crate) const SCHEME: &str = "record://";
const MIME: &str = "text/markdown";

/// One record in force: where it is addressed and what it holds.
pub(crate) struct Entry {
    pub(crate) uri: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) contents: String,
}

/// What a path segment must percent-encode: controls and the characters a
/// URI reserves or a client may mangle; every non-ASCII byte is encoded too.
const SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'/')
    .add(b'\\')
    .add(b'^')
    .add(b'[')
    .add(b']')
    .add(b'|');

fn path_uri(relative: &Path) -> String {
    let parts: Vec<_> = relative
        .iter()
        .map(|part| utf8_percent_encode(&part.to_string_lossy(), SEGMENT).to_string())
        .collect();
    format!("{SCHEME}{}", parts.join("/"))
}

fn named_uri(spec: &DocTypeSpec, relative: &Path) -> Option<String> {
    let stem = relative.file_stem()?.to_str()?;
    Some(format!("{SCHEME}{}/{stem}", spec.token))
}

fn uri_of(relative: &Path, record: &ExtractedRecord) -> String {
    let spec = doc_type::spec_for_frontmatter(&record.doc_type);
    let by_identity = spec.and_then(|spec| match spec.identity {
        Identity::Numbered { .. } => record
            .number
            .map(|number| format!("{SCHEME}{}/{number:04}", spec.token)),
        Identity::Singleton { .. } => Some(format!("{SCHEME}{}", spec.token)),
        Identity::Named { .. } => named_uri(spec, relative),
        Identity::Nested { .. } => None,
    });
    by_identity.unwrap_or_else(|| path_uri(relative))
}

fn entry_of(
    store: &FsStore,
    docs_dir: &Path,
    path: &Path,
    record: ExtractedRecord,
) -> Option<Entry> {
    let relative = path.strip_prefix(docs_dir).ok()?;
    let contents = store.read(path).ok()?;
    Some(Entry {
        uri: uri_of(relative, &record),
        title: record.title,
        description: record.description,
        contents,
    })
}

/// Every record in force under `docs_dir`, ordered by URI; none when the
/// bundle does not exist.
pub(crate) fn entries(docs_dir: &Path) -> Vec<Entry> {
    let store = FsStore::rooted(docs_dir);
    let mut found: Vec<Entry> = in_force_records(&store, docs_dir)
        .into_iter()
        .filter_map(|(path, record)| entry_of(&store, docs_dir, &path, record))
        .collect();
    found.sort_by(|a, b| a.uri.cmp(&b.uri));
    found
}

/// One resource per record in force.
pub(crate) fn resources(docs_dir: &Path) -> Vec<Resource> {
    entries(docs_dir)
        .into_iter()
        .map(|entry| {
            Resource::new(entry.uri, entry.title)
                .with_description(entry.description)
                .with_mime_type(MIME)
        })
        .collect()
}

/// The file of the record in force at `uri`; a resource-not-found error for
/// any other URI.
pub(crate) fn read(docs_dir: &Path, uri: &str) -> Result<ReadResourceResult, ErrorData> {
    let entry = entries(docs_dir)
        .into_iter()
        .find(|entry| entry.uri == uri)
        .ok_or_else(|| ErrorData::resource_not_found(format!("no record at {uri}"), None))?;
    Ok(ReadResourceResult::new(vec![ResourceContents::text(
        entry.contents,
        uri,
    )
    .with_mime_type(MIME)]))
}
