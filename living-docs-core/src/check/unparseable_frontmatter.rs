//! A record whose frontmatter block exists but does not parse as a YAML
//! mapping. It is reported as that one violation and judged no further, so it
//! never also reads as a record missing its `type`, `title` or `status`.

use super::Reporter;
use crate::frontmatter::{has_unparseable_frontmatter, UNPARSEABLE_MESSAGE};
use std::path::Path;

/// Reports `path` when its frontmatter does not parse and returns whether it
/// did, so the caller leaves the record out of every later check.
pub(super) fn report_unparseable(path: &Path, contents: &str, reporter: &mut Reporter) -> bool {
    let unparseable = has_unparseable_frontmatter(contents);
    if unparseable {
        reporter.report(path, UNPARSEABLE_MESSAGE);
    }
    unparseable
}
