//! The findings of `check` and their scoping: violations, advisories, and the
//! full report, narrowed to the records a caller touched.

use super::kinds;
use crate::inputs_hash::{matches_any, Glob};
use serde::Serialize;
use std::path::PathBuf;

/// A hard invariant failure: `check`'s exit code goes non-zero when any
/// exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Violation {
    pub file: String,
    pub message: String,
    /// Every record a group violation names, so scoping keeps it when any
    /// member is in scope. Empty for a violation of one file; never serialized.
    #[serde(skip)]
    pub anchors: Vec<String>,
}

/// A soft finding: printed alongside violations but never
/// affects the exit code. `kind` is the advisory's category,
/// one of [`super::kinds::ALL`] — so a JSON consumer can filter by it without
/// parsing `message`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Advisory {
    pub file: String,
    pub kind: String,
    pub message: String,
    /// Every record a group advisory names, so scoping keeps it when any
    /// member is in scope. Empty for an advisory of one file; never serialized.
    #[serde(skip)]
    pub anchors: Vec<String>,
}

/// `check`'s full report: every invariant's violations and
/// advisories, the bundle enumerated and its doc count, and `ok` (no
/// violations) precomputed so a renderer never has to re-derive it.
#[derive(Debug, Serialize)]
pub struct Report {
    pub bundle: String,
    pub docs: usize,
    pub violations: Vec<Violation>,
    pub advisories: Vec<Advisory>,
    pub ok: bool,
}

impl Report {
    /// The same report with every finding not anchored to one of `paths`
    /// dropped, and `ok` recomputed from what remains. Every invariant still
    /// ran over the whole bundle — this narrows what is reported, never what
    /// was computed — so a brownfield bundle can gate a commit on the records
    /// it touched while legacy debt elsewhere is paid down. An empty `paths`
    /// narrows to nothing and reports nothing.
    #[must_use]
    pub fn scoped_to(self, paths: &[PathBuf]) -> Self {
        self.scoped_to_changes(paths, &[])
    }

    /// [`Report::scoped_to`] that also keeps a `stale-view` advisory when one
    /// of `inputs`, repository-relative paths with `/` separators, matches an
    /// input glob stamped in its view. `inputs` reach no other finding, so a
    /// path outside the bundle never brings a record's violation into scope.
    #[must_use]
    pub fn scoped_to_changes(self, paths: &[PathBuf], inputs: &[String]) -> Self {
        let scope: Vec<String> = paths
            .iter()
            .map(|path| path.display().to_string())
            .collect();
        let violations: Vec<Violation> = self
            .violations
            .into_iter()
            .filter(|violation| violation.is_anchored_in(&scope))
            .collect();
        Self {
            ok: violations.is_empty(),
            violations,
            advisories: self
                .advisories
                .into_iter()
                .filter(|advisory| advisory.is_anchored_in(&scope) || advisory.reads_any(inputs))
                .collect(),
            ..self
        }
    }
}

/// Whether `file` names the same record as one of `scope`. The two come from
/// different hands — the report's paths are the store's enumeration, the
/// scope's are whatever git or the caller printed — so one is allowed to be
/// the other's suffix: `a/b/0001-x.md` matches an absolute
/// `/repo/a/b/0001-x.md` and vice versa.
fn anchors_in(file: &str, scope: &[String]) -> bool {
    scope.iter().any(|candidate| {
        file == candidate
            || file.ends_with(&format!("/{candidate}"))
            || candidate.ends_with(&format!("/{file}"))
    })
}

impl Advisory {
    /// The records the advisory is anchored on. A `stale-view` advisory is
    /// anchored on its view alone: the members after it are input globs.
    fn record_anchors(&self) -> &[String] {
        if self.kind == kinds::STALE_VIEW {
            &self.anchors[..self.anchors.len().min(1)]
        } else {
            &self.anchors
        }
    }

    /// Whether the advisory's file or any member of its group is in `scope`.
    fn is_anchored_in(&self, scope: &[String]) -> bool {
        anchors_in(&self.file, scope) || self.record_anchors().iter().any(|a| anchors_in(a, scope))
    }

    /// Whether the advisory is a `stale-view` whose stamped input globs match
    /// one of `inputs`, compared whole against repository-relative paths. A
    /// glob that does not compile matches nothing.
    fn reads_any(&self, inputs: &[String]) -> bool {
        if self.kind != kinds::STALE_VIEW || inputs.is_empty() {
            return false;
        }
        let globs: Vec<Glob> = self
            .anchors
            .iter()
            .skip(1)
            .filter_map(|glob| Glob::new(glob).ok())
            .collect();
        inputs.iter().any(|path| matches_any(&globs, path))
    }
}

impl Violation {
    /// Whether the violation's file or any member of its group is in `scope`.
    fn is_anchored_in(&self, scope: &[String]) -> bool {
        anchors_in(&self.file, scope) || self.anchors.iter().any(|a| anchors_in(a, scope))
    }
}
