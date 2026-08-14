//! `living-docs migrate` (ADR 0037): a read-only advisor that scans the
//! bundle — or detects its absence — and prints one ordered adaptation
//! plan. `RUN` steps name exact mechanical CLI commands; `AUTHOR` steps
//! name judgment work the authoring model owns (governed by the skill
//! topic `rules/migration.md`); `ADOPT` steps bootstrap a project with no
//! bundle at all. The verb never writes or edits anything.

use crate::check::traceability::requirement_ids;
use crate::commands::index::is_table_listing_row;
use crate::doc_type::{self, Identity, VIEW_KIND_ORDER};
use crate::frontmatter;
use crate::store::DocStore;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn run(store: &dyn DocStore, bundle: &Path) -> ExitCode {
    println!("Living Docs migration plan — bundle: {}", bundle.display());
    println!();
    let steps = plan(store, bundle);
    if steps.is_empty() {
        println!("Bundle is current — nothing to adapt.");
        return ExitCode::SUCCESS;
    }
    for step in &steps {
        println!("  {step}");
    }
    ExitCode::SUCCESS
}

/// The ordered adaptation steps for `bundle`, empty when it is already
/// current. A bundle whose root `index.md` the store does not list is
/// treated as absent and gets the `ADOPT` bootstrap sequence instead of a
/// per-record scan.
pub(crate) fn plan(store: &dyn DocStore, bundle: &Path) -> Vec<String> {
    let all_md = store.list(bundle).unwrap_or_default();
    if !all_md.contains(&bundle.join("index.md")) {
        return adoption_steps(bundle);
    }

    let mut steps = Vec::new();
    single_architecture_file(&all_md, bundle, &mut steps);
    views_missing_kind(store, &all_md, bundle, &mut steps);
    prds_missing_requirement_ids(store, &all_md, &mut steps);
    legacy_table_indexes(store, &all_md, &mut steps);
    if !steps.is_empty() {
        steps.push("RUN living-docs fmt".to_string());
        steps.push(format!("RUN living-docs check {}", bundle.display()));
    }
    steps
}

fn adoption_steps(bundle: &Path) -> Vec<String> {
    let at = bundle.display();
    vec![
        format!("ADOPT no docs bundle at {at} — bootstrap in this order:"),
        format!("ADOPT 1. create {at}/index.md — the bundle-root index every record hangs off (invariant 3)"),
        "ADOPT 2. living-docs new constitution \"<product>\" — scope, non-negotiables (confirm content with the user)".to_string(),
        "ADOPT 3. living-docs hooks install — doc-gate pre-commit + CLI-owned authoring hooks".to_string(),
        "ADOPT 4. living-docs seal init — baseline provenance sealing so check catches out-of-CLI writes (ADR 0039)".to_string(),
        "ADOPT 5. back-fill standing decisions with living-docs new adr — confirm each with the user, never infer (skill topic: procedure)".to_string(),
        format!("ADOPT 6. living-docs new view \"Context\" --kind context — first architecture view, then more kinds as they earn their place (one of {})", VIEW_KIND_ORDER.join("|")),
        format!("ADOPT 7. living-docs index && living-docs check {at}"),
    ]
}

fn single_architecture_file(all_md: &[PathBuf], bundle: &Path, steps: &mut Vec<String>) {
    let legacy = bundle.join("architecture.md");
    if all_md.contains(&legacy) {
        steps.push(format!(
            "AUTHOR {} — split into one view per concern: for each diagram run `living-docs new view \"<name>\" --kind <kind>`, move the fence and prose, then delete this file and relink the root index (ADR 0036)",
            legacy.display()
        ));
    }
}

fn views_missing_kind(
    store: &dyn DocStore,
    all_md: &[PathBuf],
    bundle: &Path,
    steps: &mut Vec<String>,
) {
    let view_dir = named_view_dir(bundle);
    for path in all_md {
        if path.parent() != Some(view_dir.as_path()) || ends_with_index(path) {
            continue;
        }
        let Ok(contents) = store.read(path) else {
            continue;
        };
        if frontmatter::read_scalar_from_str(&contents, "kind").is_none() {
            steps.push(format!(
                "AUTHOR {} — missing `kind:` frontmatter; set one of {} so the generated index can rank it (ADR 0036)",
                path.display(),
                VIEW_KIND_ORDER.join("|")
            ));
        }
    }
}

/// The bundle's Named-identity (architecture view) directory, resolved from
/// the registry rather than a literal so a renamed row keeps this advisor
/// honest.
fn named_view_dir(bundle: &Path) -> PathBuf {
    let dir = doc_type::DOC_TYPES
        .iter()
        .find_map(|spec| match spec.identity {
            Identity::Named { dir } => Some(dir),
            Identity::Numbered { .. } | Identity::Singleton { .. } => None,
        })
        .unwrap_or("architecture");
    bundle.join(dir)
}

fn prds_missing_requirement_ids(store: &dyn DocStore, all_md: &[PathBuf], steps: &mut Vec<String>) {
    for path in all_md {
        let Ok(contents) = store.read(path) else {
            continue;
        };
        if frontmatter::read_scalar_from_str(&contents, "type").as_deref() != Some("PRD") {
            continue;
        }
        let status = frontmatter::read_scalar_from_str(&contents, "status");
        let past_draft = matches!(status.as_deref(), Some("Accepted") | Some("Implemented"));
        if past_draft && requirement_ids(&contents).is_empty() {
            steps.push(format!(
                "AUTHOR {} — requirements carry no FR-N/NFR-N IDs; rewrite them as EARS statements under stable IDs so BDRs can cite what they prove (ADR 0035)",
                path.display()
            ));
        }
    }
}

fn legacy_table_indexes(store: &dyn DocStore, all_md: &[PathBuf], steps: &mut Vec<String>) {
    for path in all_md {
        if !ends_with_index(path) || !in_registry_dir(path) {
            continue;
        }
        let Ok(contents) = store.read(path) else {
            continue;
        };
        if contents.lines().any(is_table_listing_row) {
            steps.push(format!(
                "RUN living-docs index — {} is a hand-maintained table listing; the generator migrates it in place",
                path.display()
            ));
        }
    }
}

fn ends_with_index(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "index.md")
}

fn in_registry_dir(path: &Path) -> bool {
    path.parent()
        .and_then(Path::file_name)
        .and_then(|dir| dir.to_str())
        .is_some_and(|dir| doc_type::spec_for_dir(dir).is_some())
}

#[cfg(test)]
mod tests;
