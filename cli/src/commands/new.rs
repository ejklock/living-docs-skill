//! `new` verb wrapper: delegates to `living_docs_core::commands::new::write`,
//! rendering the created path as colored text or JSON.

use super::prose;
use super::report::report_indexed;
use crate::output::{self, OutputMode};
use crate::store::build_store;
use living_docs_core::commands;
use living_docs_core::commands::new::{NewOptions, BODY_ONLY_INSTRUCTION};
use living_docs_core::doc_type::spec_for;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The `new` verb's optional CLI arguments, bundled so the wrapper's
/// signature stays within the argument budget as flags accrue.
pub(crate) struct NewArgs<'a> {
    pub(crate) description: Option<&'a str>,
    pub(crate) kind: Option<&'a str>,
    pub(crate) owner: Option<&'a str>,
    pub(crate) body: Option<&'a str>,
}

#[derive(Serialize)]
struct NewJson<'a> {
    path: &'a Path,
    instruction: &'a str,
    indexed: &'a [PathBuf],
}

/// The owner for a type whose registry row requires one and the caller gave
/// none: the git `user.name`, else `user.email`.
fn seeded_owner(doc_type: &str, given: Option<&str>) -> Option<String> {
    if given.is_some() || !spec_for(doc_type).is_some_and(|spec| spec.requires_owner) {
        return None;
    }
    crate::git::identity()
}

fn print_created(created: &commands::new::Created, mode: OutputMode) {
    if mode.is_json() {
        output::outln!(
            "{}",
            output::to_json(&NewJson {
                path: &created.path,
                instruction: BODY_ONLY_INSTRUCTION,
                indexed: &created.indexed.written,
            })
        );
    } else {
        output::outln!("{}", created.path.display());
        output::outln!("{BODY_ONLY_INSTRUCTION}");
    }
}

pub(crate) fn run_new(
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    args: &NewArgs,
    mode: OutputMode,
    quiet: bool,
) -> ExitCode {
    let seeded = seeded_owner(doc_type, args.owner);
    if let (Some(owner), false) = (&seeded, quiet) {
        eprintln!("living-docs new: owner seeded from git config: {owner}");
    }
    let body = match args.body.map(prose::read_body).transpose() {
        Ok(body) => body,
        Err(message) => {
            eprintln!("living-docs new: {message}");
            return ExitCode::from(2);
        }
    };
    let opts = NewOptions {
        description: args.description,
        kind: args.kind,
        owner: args.owner.or(seeded.as_deref()),
        body: body.as_deref(),
    };
    let store = build_store(docs_dir);
    match commands::new::write(store.as_ref(), docs_dir, doc_type, title, &opts) {
        Ok(created) => {
            print_created(&created, mode);
            report_indexed("new", &created.indexed, mode, quiet)
        }
        Err(message) => {
            eprintln!("living-docs new: {message}");
            ExitCode::from(2)
        }
    }
}
