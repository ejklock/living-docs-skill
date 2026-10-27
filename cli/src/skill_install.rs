//! Materializes the embedded skills corpus (ADR 0014/0017) into a harness's
//! skills directory (ADR 0028) — no working tree involved, ever. Mirrors
//! `crate::hooks`'s embedded-asset materialization: the corpus is the only
//! source, placement is idempotent, and `--dry-run` writes nothing.

use crate::{output::outln, skill};
use clap::ValueEnum;
use std::borrow::Cow;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The AI harnesses `install skills` places the corpus for natively — every
/// one auto-discovers `SKILL.md` files under its own skills directory, so
/// placement never needs a generated pointer file (unlike `cursor`/`copilot`,
/// slice S2 of ADR 0028).
#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum Harness {
    Claude,
    Opencode,
    Codex,
    Pi,
}

struct SkillAssetSpec {
    skill: &'static str,
    relative_paths: &'static [&'static str],
}

const SKILL_ASSET_SPECS: [SkillAssetSpec; 3] = [
    SkillAssetSpec {
        skill: "living-docs",
        relative_paths: &["SKILL.md"],
    },
    SkillAssetSpec {
        skill: "okf-knowledge-format",
        relative_paths: &["SKILL.md", "reference/SPEC.md", "reference/SPEC.source.md"],
    },
    SkillAssetSpec {
        skill: "research-artifacts",
        relative_paths: &["SKILL.md"],
    },
];

struct Placement {
    dest: PathBuf,
    bytes: Cow<'static, [u8]>,
}

/// Places every [`SKILL_ASSET_SPECS`] asset under the harness's skills
/// directory: `--dir` wins outright over the harness's own layout, `--project`
/// selects the project-relative form (resolved against the current
/// directory), and the default is the harness's global, `$HOME`-rooted form.
/// Under `dry_run`, reports the same plan on stdout and changes nothing on
/// disk. A missing embedded asset or an unresolvable `$HOME` (only reachable
/// on the global-destination path) is a hard error — named on stderr,
/// `ExitCode::from(2)`.
pub(crate) fn install(
    harness: Harness,
    project: bool,
    dir: Option<PathBuf>,
    dry_run: bool,
) -> ExitCode {
    let root = match destination_root(harness, project, dir.as_deref()) {
        Ok(root) => root,
        Err(message) => return report_failure(&message),
    };
    let placements = match resolve_placements(&root) {
        Ok(placements) => placements,
        Err(message) => return report_failure(&message),
    };
    if dry_run {
        announce_dry_run(&placements);
        return ExitCode::SUCCESS;
    }
    match write_placements(&placements) {
        Ok(()) => {
            for placement in &placements {
                outln!("wrote {}", placement.dest.display());
            }
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err.to_string()),
    }
}

/// The harness's own skills directory, relative to `$HOME` (used when
/// neither `--project` nor `--dir` is given) and relative to the project root
/// (used with `--project`) — the source of truth for install.sh's former
/// harness destination table.
fn harness_paths(harness: Harness) -> (&'static str, &'static str) {
    match harness {
        Harness::Claude => (".claude/skills", ".claude/skills"),
        Harness::Opencode => (".config/opencode/skills", ".opencode/skills"),
        Harness::Codex => (".codex/skills", ".codex/skills"),
        Harness::Pi => (".pi/agent/skills", ".pi/skills"),
    }
}

fn destination_root(
    harness: Harness,
    project: bool,
    dir: Option<&Path>,
) -> Result<PathBuf, String> {
    if let Some(dir) = dir {
        return Ok(dir.to_path_buf());
    }
    if project {
        return Ok(PathBuf::from(harness_paths(harness).1));
    }
    resolve_home().map(|home| resolved_global(harness, &home))
}

fn resolved_global(harness: Harness, home: &Path) -> PathBuf {
    home.join(harness_paths(harness).0)
}

fn resolve_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
        "cannot resolve the global skills directory: $HOME is not set (pass --dir or --project)"
            .to_owned()
    })
}

fn resolve_placements(root: &Path) -> Result<Vec<Placement>, String> {
    SKILL_ASSET_SPECS
        .iter()
        .flat_map(|spec| {
            spec.relative_paths
                .iter()
                .map(move |path| (spec.skill, *path))
        })
        .map(|(skill_name, relative_path)| resolve_one(root, skill_name, relative_path))
        .collect()
}

fn resolve_one(root: &Path, skill_name: &str, relative_path: &str) -> Result<Placement, String> {
    let asset_path = format!("{skill_name}/{relative_path}");
    let bytes =
        skill::asset(&asset_path).ok_or_else(|| format!("missing embedded asset: {asset_path}"))?;
    Ok(Placement {
        dest: root.join(skill_name).join(relative_path),
        bytes,
    })
}

fn announce_dry_run(placements: &[Placement]) {
    for placement in placements {
        outln!("[dry-run] would write {}", placement.dest.display());
    }
}

fn write_placements(placements: &[Placement]) -> io::Result<()> {
    placements.iter().try_for_each(write_one)
}

fn write_one(placement: &Placement) -> io::Result<()> {
    if let Some(parent) = placement.dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&placement.dest, placement.bytes.as_ref())
}

fn report_failure(message: &str) -> ExitCode {
    eprintln!("living-docs install skills: {message}");
    ExitCode::from(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[derive(rust_embed::RustEmbed)]
    #[folder = "../skills/"]
    struct SkillCorpusAssets;

    /// Whether `relative_path` (relative to a skill's own directory, e.g.
    /// `reference/SPEC.md`) is one `install skills` places on a harness's
    /// filesystem, versus one served through `living-docs guide <topic>
    /// --skill <name>`. Only the slim-stub `SKILL.md` (ADR 0014) and
    /// anything a stub links to directly by relative path — vendored
    /// external material such as `okf-knowledge-format`'s `reference/` spec
    /// — has to exist on disk; `rules/`, `templates/`, `hooks/`, `scripts/`,
    /// and `tests/` are progressive-disclosure or dev-only content the CLI
    /// (or, for `hooks/`, `install hooks`) serves instead.
    /// [`SKILL_ASSET_SPECS`] must declare exactly the paths this predicate
    /// accepts, proven below.
    fn is_skill_install_asset(relative_path: &str) -> bool {
        relative_path == "SKILL.md" || relative_path.starts_with("reference/")
    }

    fn declared_paths(skill_name: &str) -> BTreeSet<String> {
        SKILL_ASSET_SPECS
            .iter()
            .find(|spec| spec.skill == skill_name)
            .into_iter()
            .flat_map(|spec| spec.relative_paths.iter().map(|path| (*path).to_owned()))
            .collect()
    }

    fn corpus_installable_paths(skill_name: &str) -> BTreeSet<String> {
        let prefix = format!("{skill_name}/");
        SkillCorpusAssets::iter()
            .filter_map(|path| path.strip_prefix(prefix.as_str()).map(str::to_owned))
            .filter(|relative| is_skill_install_asset(relative))
            .collect()
    }

    #[test]
    fn skill_asset_specs_matches_every_installable_asset_the_corpus_holds() {
        for spec in &SKILL_ASSET_SPECS {
            let corpus = corpus_installable_paths(spec.skill);
            assert!(
                !corpus.is_empty(),
                "{} has no installable assets in the embedded corpus \
                 — the corpus is empty or is_skill_install_asset is wrong",
                spec.skill
            );
            assert_eq!(
                corpus,
                declared_paths(spec.skill),
                "SKILL_ASSET_SPECS for {} has drifted from the embedded corpus",
                spec.skill
            );
        }
    }

    #[test]
    fn resolve_placements_covers_every_skill_md() {
        let placements = resolve_placements(Path::new("/dest")).expect("assets are embedded");
        let dests: Vec<PathBuf> = placements.iter().map(|p| p.dest.clone()).collect();
        for skill_name in ["living-docs", "okf-knowledge-format", "research-artifacts"] {
            assert!(
                dests.contains(&PathBuf::from(format!("/dest/{skill_name}/SKILL.md"))),
                "missing {skill_name}/SKILL.md, got: {dests:?}"
            );
        }
    }

    #[test]
    fn resolve_placements_includes_the_okf_reference_subdirectory() {
        let placements = resolve_placements(Path::new("/dest")).expect("assets are embedded");
        let dests: Vec<PathBuf> = placements.iter().map(|p| p.dest.clone()).collect();
        assert!(
            dests.contains(&PathBuf::from(
                "/dest/okf-knowledge-format/reference/SPEC.md"
            )),
            "got: {dests:?}"
        );
        assert!(
            dests.contains(&PathBuf::from(
                "/dest/okf-knowledge-format/reference/SPEC.source.md"
            )),
            "got: {dests:?}"
        );
    }

    #[test]
    fn resolved_global_maps_each_harness_to_its_own_directory_under_home() {
        let home = Path::new("/home/u");
        assert_eq!(
            resolved_global(Harness::Claude, home),
            PathBuf::from("/home/u/.claude/skills"),
            "claude"
        );
        assert_eq!(
            resolved_global(Harness::Opencode, home),
            PathBuf::from("/home/u/.config/opencode/skills"),
            "opencode"
        );
        assert_eq!(
            resolved_global(Harness::Codex, home),
            PathBuf::from("/home/u/.codex/skills"),
            "codex"
        );
        assert_eq!(
            resolved_global(Harness::Pi, home),
            PathBuf::from("/home/u/.pi/agent/skills"),
            "pi"
        );
    }

    #[test]
    fn destination_root_project_scopes_each_harness_relative_to_the_current_directory() {
        assert_eq!(
            destination_root(Harness::Claude, true, None).unwrap(),
            PathBuf::from(".claude/skills"),
            "claude"
        );
        assert_eq!(
            destination_root(Harness::Opencode, true, None).unwrap(),
            PathBuf::from(".opencode/skills"),
            "opencode"
        );
        assert_eq!(
            destination_root(Harness::Codex, true, None).unwrap(),
            PathBuf::from(".codex/skills"),
            "codex"
        );
        assert_eq!(
            destination_root(Harness::Pi, true, None).unwrap(),
            PathBuf::from(".pi/skills"),
            "pi"
        );
    }

    #[test]
    fn destination_root_dir_override_wins_over_project_and_harness() {
        let dir = Path::new("/custom/dest");
        let resolved = destination_root(Harness::Codex, true, Some(dir)).unwrap();
        assert_eq!(resolved, dir.to_path_buf());
    }

    #[test]
    fn resolve_placements_errors_never_touch_the_filesystem_for_a_missing_asset() {
        let placements = resolve_one(Path::new("/dest"), "no-such-skill", "SKILL.md");
        assert!(placements.is_err());
    }
}
