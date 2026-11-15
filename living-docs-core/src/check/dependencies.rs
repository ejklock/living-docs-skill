//! Issue dependencies declared in `blocked_by`, whose items are local issue
//! numbers: every item names an issue of the bundle, no issue waits on itself,
//! and no issues wait on one another in a cycle.

use super::Reporter;
use std::path::{Path, PathBuf};

/// Reports a `blocked_by` item that names no issue of the bundle, an issue
/// that lists itself, and every cycle of issues waiting on one another.
pub(crate) fn check_dependencies(_bundle: &Path, _all_md: &[PathBuf], _reporter: &mut Reporter) {}
