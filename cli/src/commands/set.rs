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
        output::outln!("{}", path.display());
    }
    eprintln!(
        "living-docs write set: rewrote {} in-bundle reference(s) to {previous}; references outside the bundle are not rewritten — run `grep -rn {previous} .`",
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
            return Err("give the value or --from, not both: `write set <ref> <key> <value>` or `write set <ref> <key> --from PATH|-`".to_string())
        }
        (None, None) => {
            return Err("write set needs a value or --from: `write set <ref> <key> <value>` or `write set <ref> <key> --from PATH|-`".to_string())
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
            "section needs a heading: `write set <ref> section \"<heading>\" --from PATH|-`".to_string(),
        ),
        (Some(_), None) => Err(
            "section takes its text from --from: `write set <ref> section \"<heading>\" --from PATH|-`"
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
    let store = build_store(docs_dir);
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
    let result = resolve_change(args)
        .and_then(|change| apply_change(docs_dir, args, &change).map(|applied| (change, applied)));
    match result {
        Ok((change, applied)) => {
            print_applied(&applied, args.key, &change.value, mode);
            report_indexed("write set", &applied.indexed, mode, quiet)
        }
        Err(message) => {
            eprintln!("living-docs write set: {message}");
            let argv: Vec<_> = std::env::args_os().collect();
            if let Some(hint) = split_reference::hint(&argv) {
                eprintln!("{hint}");
            }
            ExitCode::from(2)
        }
    }
}

fn print_applied(applied: &commands::set::Applied, key: &str, value: &str, mode: OutputMode) {
    if mode.is_json() {
        output::outln!(
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
        output::outln!("{}", applied.path.display());
        report_rewritten(applied);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args<'a>(key: &'a str, value: Option<&'a str>, from: Option<&'a str>) -> SetArgs<'a> {
        SetArgs {
            reference: "0001",
            key,
            value,
            from,
        }
    }

    fn error_of(result: Result<Change, String>) -> String {
        match result {
            Ok(_) => panic!("expected a usage error"),
            Err(message) => message,
        }
    }

    fn assert_field_forms(message: &str) {
        assert!(
            message.contains("`write set <ref> <key> <value>`"),
            "{message}"
        );
        assert!(
            message.contains("`write set <ref> <key> --from PATH|-`"),
            "{message}"
        );
    }

    fn assert_section_form(message: &str) {
        assert!(
            message.contains("`write set <ref> section \"<heading>\" --from PATH|-`"),
            "{message}"
        );
    }

    #[test]
    fn a_field_with_neither_value_nor_from_teaches_the_write_set_forms() {
        assert_field_forms(&error_of(resolve_change(&args("status", None, None))));
    }

    #[test]
    fn a_field_given_both_a_value_and_from_teaches_the_write_set_forms() {
        let both = args("status", Some("x"), Some("-"));
        assert_field_forms(&error_of(resolve_change(&both)));
    }

    #[test]
    fn a_section_without_from_teaches_the_write_set_form() {
        let no_from = args("section", Some("Notes"), None);
        assert_section_form(&error_of(resolve_change(&no_from)));
    }

    #[test]
    fn a_section_without_a_heading_teaches_the_write_set_form() {
        let no_heading = args("section", None, Some("-"));
        assert_section_form(&error_of(resolve_change(&no_heading)));
    }
}
