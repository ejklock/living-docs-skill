//! The findings of `check` and their scoping: violations, advisories, and the
//! full report, narrowed to the records a caller touched.

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
                .filter(|advisory| anchors_in(&advisory.file, &scope))
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

impl Violation {
    /// Whether the violation's file or any member of its group is in `scope`.
    fn is_anchored_in(&self, scope: &[String]) -> bool {
        anchors_in(&self.file, scope) || self.anchors.iter().any(|a| anchors_in(a, scope))
    }
}
