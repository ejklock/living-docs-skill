use args::{Cli, Command, DbCmd, HooksCmd, SkillCmd};
use clap::Parser;
use living_docs_core::check;
use std::process::ExitCode;

mod args;
mod commands;
mod config;
mod hooks;
mod skill;
mod skill_install;
mod store;

#[allow(clippy::too_many_lines)]
fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Next { doc_type } => commands::next::run_next(&cli.docs_dir, &doc_type),
        Command::New {
            doc_type,
            title,
            description,
            kind,
        } => commands::new::run_new(
            cli.backend,
            cli.engine,
            &cli.docs_dir,
            &doc_type,
            &title,
            description.as_deref(),
            kind.as_deref(),
        ),
        Command::Brief {
            doc_type,
            title,
            from_diff,
        } => commands::brief::run_brief(
            cli.backend,
            cli.engine,
            &cli.docs_dir,
            &doc_type,
            &title,
            from_diff,
        ),
        Command::Index {
            doc_type,
            visibility,
        } => {
            commands::index::run_index(cli.backend, cli.engine, &cli.docs_dir, doc_type, visibility)
        }
        Command::Supersede { old, new } => {
            commands::supersede::run_supersede(cli.backend, cli.engine, &cli.docs_dir, &old, &new)
        }
        Command::Status { number, new_status } => commands::status::run_status(
            cli.backend,
            cli.engine,
            &cli.docs_dir,
            &number,
            &new_status,
        ),
        Command::Describe {
            number,
            description,
        } => commands::describe::run_describe(
            cli.backend,
            cli.engine,
            &cli.docs_dir,
            &number,
            &description,
        ),
        Command::Check {
            paths,
            mermaid_only,
        } if mermaid_only => check::run_mermaid_only(&paths),
        Command::Check { paths, .. } => {
            commands::check::run_check(cli.backend, cli.engine, &cli.docs_dir, paths)
        }
        Command::Fmt { paths } => commands::fmt::run_fmt(&cli.docs_dir, paths),
        Command::Export {
            out_dir,
            visibility,
        } => commands::export::run_export(
            cli.backend,
            cli.engine,
            &cli.docs_dir,
            &out_dir,
            visibility,
        ),
        Command::LeakGate {
            bundle,
            check_tier3,
        } => commands::leak_gate::run_leak_gate(&bundle, check_tier3),
        Command::Db {
            cmd: DbCmd::Sync { project },
        } => commands::db::run_db_sync(&cli.docs_dir, cli.engine, project),
        Command::Search { query, project } => {
            commands::search::run_search(&query, cli.engine, project)
        }
        Command::Skill {
            action:
                Some(SkillCmd::Install {
                    harness,
                    project,
                    dir,
                    dry_run,
                }),
            ..
        } => skill_install::install(harness, project, dir, dry_run),
        Command::Skill {
            name,
            topic,
            list,
            json,
            plain,
            ..
        } => commands::skill_cmd::run_skill(name, topic, list, json, plain),
        Command::Hooks {
            cmd: HooksCmd::Install { dir, dry_run },
        } => commands::hooks_cmd::run_hooks_install(dir, dry_run, &cli.docs_dir),
        Command::Hooks {
            cmd: HooksCmd::Uninstall { dir, dry_run },
        } => commands::hooks_cmd::run_hooks_uninstall(dir, dry_run),
    }
}
