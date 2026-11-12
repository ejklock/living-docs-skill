use args::{
    CheckArgs, Command, InstallCmd, ManageCmd, NewRecordArgs, ReadCmd, UninstallCmd, WriteCmd,
};
use output::{ColorMode, OutputMode};
use std::path::Path;
use std::process::ExitCode;

mod args;
mod commands;
mod doc_gate;
mod hooks;
mod output;
mod skill_install;
mod store;

fn main() -> ExitCode {
    let cli = args::parse();
    let mode = OutputMode::from_flags(cli.json, cli.plain, cli.color);
    let color = ColorMode::from_choice(cli.color);
    let quiet = cli.quiet;
    match cli.command {
        Command::Write { operation } => run_write(&cli.docs_dir, operation, mode, quiet),
        Command::Read { operation } => run_read(&cli.docs_dir, operation, mode, quiet),
        Command::Manage { operation } => run_manage(&cli.docs_dir, operation, mode, quiet),
        Command::Check(args) => run_check(&cli.docs_dir, args, mode, color, quiet),
        Command::Mcp => commands::mcp::run_mcp(&cli.docs_dir),
    }
}

fn run_check(
    docs_dir: &Path,
    args: CheckArgs,
    mode: OutputMode,
    color: ColorMode,
    quiet: bool,
) -> ExitCode {
    if args.mermaid_only {
        return commands::check::run_mermaid_only(&args.paths, mode, color);
    }
    commands::check::run_check(
        docs_dir,
        args.paths,
        args.require_owner,
        &args.changed_files,
        mode,
        color,
        &commands::check::Verbosity {
            list_advisories: args.advisories,
            quiet,
        },
    )
}

fn run_write_new(docs_dir: &Path, args: NewRecordArgs, mode: OutputMode, quiet: bool) -> ExitCode {
    let fields = commands::new::NewArgs {
        description: args.description.as_deref(),
        kind: args.kind.as_deref(),
        owner: args.owner.as_deref(),
        body: args.body.as_deref(),
    };
    commands::new::run_new(docs_dir, &args.doc_type, &args.title, &fields, mode, quiet)
}

/// The operands of `write set`, as typed.
struct SetFields {
    reference: String,
    key: String,
    value: Option<String>,
    from: Option<String>,
}

fn run_write_set(docs_dir: &Path, fields: SetFields, mode: OutputMode, quiet: bool) -> ExitCode {
    let args = commands::set::SetArgs {
        reference: &fields.reference,
        key: &fields.key,
        value: fields.value.as_deref(),
        from: fields.from.as_deref(),
    };
    commands::set::run_set(docs_dir, &args, mode, quiet)
}

fn run_write(docs_dir: &Path, operation: WriteCmd, mode: OutputMode, quiet: bool) -> ExitCode {
    match operation {
        WriteCmd::New(args) => run_write_new(docs_dir, args, mode, quiet),
        WriteCmd::Supersede { old, new } => {
            commands::supersede::run_supersede(docs_dir, &old, &new, mode, quiet)
        }
        WriteCmd::Set {
            reference,
            key,
            value,
            from,
        } => run_write_set(
            docs_dir,
            SetFields {
                reference,
                key,
                value,
                from,
            },
            mode,
            quiet,
        ),
    }
}

fn run_read(docs_dir: &Path, operation: ReadCmd, mode: OutputMode, quiet: bool) -> ExitCode {
    match operation {
        ReadCmd::Records(args) => commands::read::run_read(docs_dir, args, mode, quiet),
        ReadCmd::Guide(args) => commands::guide::run_guide(args, mode),
    }
}

fn run_manage(docs_dir: &Path, operation: ManageCmd, mode: OutputMode, quiet: bool) -> ExitCode {
    match operation {
        ManageCmd::Index { doc_type } => commands::index::run_index(docs_dir, doc_type, mode),
        ManageCmd::Fmt { paths, check } => commands::fmt::run_fmt(docs_dir, paths, check, mode),
        ManageCmd::Install {
            action:
                InstallCmd::Skills {
                    harness,
                    project,
                    dir,
                    dry_run,
                },
        } => commands::install::run_install_skills(harness, project, dir, dry_run),
        ManageCmd::Install {
            action: InstallCmd::Hooks { dir, dry_run },
        } => commands::install::run_install_hooks(dir, dry_run, docs_dir, quiet),
        ManageCmd::Uninstall {
            action: UninstallCmd::Hooks { dir, dry_run },
        } => commands::uninstall::run_uninstall_hooks(dir, dry_run, docs_dir),
        ManageCmd::Completions { shell } => commands::completions::run_completions(shell),
    }
}
