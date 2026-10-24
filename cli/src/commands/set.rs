//! `set` verb wrapper: delegates to `living_docs_core::commands::set::apply`,
//! rendering the written record as colored text or JSON.

use crate::args::split_reference;
use crate::commands::prose;
use crate::commands::report::report_indexed;
use crate::output::{self, OutputMode};
use crate::store::build_store;
use living_docs_core::commands;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Serialize)]
struct SetJson<'a> {
    path: &'a Path,
    key: &'a str,
    value: &'a str,
    /// The other bundle records whose reference to the renamed file was
    /// rewritten by a retitle; empty for every other key.
    rewritten: &'a [PathBuf],
    indexed: &'a [PathBuf],
}

/// Names what a retitle rewrote and the search for what it could not reach:
/// a reference outside the bundle — another repo's README, a code comment, a
/// published URL — is beyond any scan the tool owns.
fn report_rewritten(applied: &commands::set::Applied) {
    let Some(previous) = applied.previous.as_ref().and_then(|path| path.file_name()) else {
        return;
    };
    let previous = previous.to_string_lossy();
    for path in &applied.rewritten {
        println!("{}", path.display());
    }
    eprintln!(
        "living-docs set: rewrote {} in-bundle reference(s) to {previous}; references outside the bundle are not rewritten — run `grep -rn {previous} .`",
        applied.rewritten.len()
    );
}

/// The `set` verb's positional and `--from` arguments as typed.
pub(crate) struct SetArgs<'a> {
    pub(crate) reference: &'a str,
    pub(crate) key: &'a str,
    pub(crate) value: Option<&'a str>,
    pub(crate) from: Option<&'a str>,
}

/// What `set` writes: a field value, or for the `section` key the heading
/// (in `value`) and the text that goes under it.
struct Change {
    value: String,
    text: Option<String>,
}

fn field_change(args: &SetArgs) -> Result<Change, String> {
    let value = match (args.value, args.from) {
        (Some(value), None) => value.to_string(),
        (None, Some(spec)) => prose::read_value(spec)?,
        (Some(_), Some(_)) => {
            return Err("give the value or --from, not both: `set <ref> <key> <value>` or `set <ref> <key> --from PATH|-`".to_string())
        }
        (None, None) => {
            return Err("set needs a value or --from: `set <ref> <key> <value>` or `set <ref> <key> --from PATH|-`".to_string())
        }
    };
    Ok(Change { value, text: None })
}

fn section_change(args: &SetArgs) -> Result<Change, String> {
    match (args.value, args.from) {
        (Some(heading), Some(spec)) => Ok(Change {
            value: heading.to_string(),
            text: Some(prose::read_body(spec)?),
        }),
        (None, _) => Err(
            "section needs a heading: `set <ref> section \"<heading>\" --from PATH|-`".to_string(),
        ),
        (Some(_), None) => Err(
            "section takes its text from --from: `set <ref> section \"<heading>\" --from PATH|-`"
                .to_string(),
        ),
    }
}

fn resolve_change(args: &SetArgs) -> Result<Change, String> {
    if args.key == "section" {
        section_change(args)
    } else {
        field_change(args)
    }
}

fn apply_change(
    docs_dir: &Path,
    args: &SetArgs,
    change: &Change,
) -> Result<commands::set::Applied, String> {
    let store = build_store();
    match &change.text {
        Some(text) => commands::set::apply_section(
            store.as_ref(),
            docs_dir,
            args.reference,
            &change.value,
            text,
        ),
        None => commands::set::apply(
            store.as_ref(),
            docs_dir,
            args.reference,
            args.key,
            &change.value,
        ),
    }
}

pub(crate) fn run_set(docs_dir: &Path, args: &SetArgs, mode: OutputMode, quiet: bool) -> ExitCode {
    let key = args.key;
    let result = resolve_change(args)
        .and_then(|change| apply_change(docs_dir, args, &change).map(|applied| (change, applied)));
    match result {
        Ok((change, applied)) => {
            let value = change.value.as_str();
            if mode.is_json() {
                println!(
                    "{}",
                    output::to_json(&SetJson {
                        path: &applied.path,
                        key,
                        value,
                        rewritten: &applied.rewritten,
                        indexed: &applied.indexed.written,
                    })
                );
            } else {
                println!("{}", applied.path.display());
                report_rewritten(&applied);
            }
            report_indexed("set", &applied.indexed, mode, quiet)
        }
        Err(message) => {
            eprintln!("living-docs set: {message}");
            let argv: Vec<_> = std::env::args_os().collect();
            if let Some(hint) = split_reference::hint(&argv) {
                eprintln!("{hint}");
            }
            ExitCode::from(2)
        }
    }
}
