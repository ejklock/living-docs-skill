use args::{Command, InstallCmd, UninstallCmd};
use output::{ColorMode, OutputMode};
use std::process::ExitCode;

mod args;
mod commands;
mod hooks;
mod output;
mod skill;
mod skill_install;
mod store;

#[allow(clippy::too_many_lines)]
fn main() -> ExitCode {
    let cli = args::parse();
    let mode = OutputMode::from_flags(cli.json, cli.plain, cli.color);
    let color = ColorMode::from_choice(cli.color);
    let quiet = cli.quiet;
    match cli.command {
        Command::New {
            doc_type,
            title,
            description,
            kind,
            owner,
            body,
        } => commands::new::run_new(
            &cli.docs_dir,
            &doc_type,
            &title,
            &commands::new::NewArgs {
                description: description.as_deref(),
                kind: kind.as_deref(),
                owner: owner.as_deref(),
                body: body.as_deref(),
            },
            mode,
            quiet,
        ),
        Command::Index { doc_type } => commands::index::run_index(&cli.docs_dir, doc_type, mode),
        Command::Supersede { old, new } => {
            commands::supersede::run_supersede(&cli.docs_dir, &old, &new, mode, quiet)
        }
        Command::Set {
            reference,
            key,
            value,
            from,
        } => commands::set::run_set(
            &cli.docs_dir,
            &commands::set::SetArgs {
                reference: &reference,
                key: &key,
                value: value.as_deref(),
                from: from.as_deref(),
            },
            mode,
            quiet,
        ),
        Command::Check {
            paths,
            mermaid_only,
            ..
        } if mermaid_only => commands::check::run_mermaid_only(&paths, mode, color),
        Command::Check {
            paths,
            require_owner,
            changed_files,
            advisories,
            ..
        } => commands::check::run_check(
            &cli.docs_dir,
            paths,
            require_owner,
            &changed_files,
            mode,
            color,
            advisories,
        ),
        Command::Fmt { paths, check } => commands::fmt::run_fmt(&cli.docs_dir, paths, check, mode),
        Command::Read(args) => commands::read::run_read(&cli.docs_dir, args, mode, quiet),
        Command::Guide(args) => commands::guide::run_guide(args, mode),
        Command::Install {
            action:
                InstallCmd::Skills {
                    harness,
                    project,
                    dir,
                    dry_run,
                },
        } => commands::install::run_install_skills(harness, project, dir, dry_run),
        Command::Install {
            action: InstallCmd::Hooks { dir, dry_run },
        } => commands::install::run_install_hooks(dir, dry_run, &cli.docs_dir, quiet),
        Command::Uninstall {
            action: UninstallCmd::Hooks { dir, dry_run },
        } => commands::uninstall::run_uninstall_hooks(dir, dry_run),
        Command::Completions { shell } => commands::completions::run_completions(shell),
    }
}
