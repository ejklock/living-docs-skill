use std::path::Path;

/// The guidance a client receives at discovery: what the server serves, how to
/// read before deciding, and who owns the record files.
pub fn instructions(docs_dir: &Path) -> String {
    format!(
        "This server serves one living-docs bundle: {bundle}\n\
\n\
Four tools, each grouping operations by an `operation` argument:\n\
- `read` (records): the in-force view of the bundle. Filter with `topic`, or ask `links_to` / `impact` for the records that depend on one.\n\
- `write` (new, set, supersede): the only way to create or change a record.\n\
- `manage` (index, fmt): regenerate indexes and canonicalize frontmatter.\n\
- `check`: the gate. It reports violations as data; run it after writing.\n\
\n\
Before deciding anything, read what is already decided: call `read` with `topic` for the subject, and with `links_to` or `impact` on a record before you change or supersede it.\n\
\n\
Never write a record file directly. `write` owns numbering, frontmatter and indexes; give `new` and `set` only the prose.\n",
        bundle = docs_dir.display()
    )
}
