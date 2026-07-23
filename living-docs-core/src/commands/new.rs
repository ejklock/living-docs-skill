use crate::commands::next::next_number_from_store;
use crate::paths;
use crate::record::format_scalar;
use crate::store::DocStore;
use crate::templates;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

/// The point-of-use teaching line (ADR 0019, decision 3): printed after
/// `new`'s created path, and repeated verbatim in the root `--help` about
/// text and the `living-docs` SKILL.md stub, so an agent meets the
/// CLI-owns-the-mechanics rule at the moment it authors a record.
pub const BODY_ONLY_INSTRUCTION: &str = "Write ONLY the body below the closing ---. Frontmatter and indexes are CLI-owned: `living-docs status` / `supersede` / `index`.";

pub fn run(store: &dyn DocStore, docs_dir: &Path, doc_type: &str, title: &str) -> ExitCode {
    match scaffold(store, docs_dir, doc_type, title, &now_iso8601()) {
        Ok(path) => {
            println!("{}", path.display());
            println!("{BODY_ONLY_INSTRUCTION}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs new: {message}");
            ExitCode::from(2)
        }
    }
}

/// Computes `new`'s target path and filled content without writing it —
/// the pure planning half of [`scaffold`], reused by [`plan`] (with
/// today's timestamp) and by any caller (e.g.
/// `db_store::DbDocStore::write_checked`) that needs to run its own
/// transactional write instead of [`crate::store::DocStore::write`].
fn plan_at(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    timestamp: &str,
) -> Result<(PathBuf, String), String> {
    let dir_name = paths::dir_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    let frontmatter_type = paths::frontmatter_type_for(doc_type)
        .expect("dir_for and frontmatter_type_for cover the same doc types");
    let template = templates::template_for(doc_type)
        .expect("dir_for and template_for cover the same doc types");

    let type_dir = docs_dir.join(dir_name);
    let number = next_number_from_store(store, docs_dir, dir_name).map_err(|e| e.to_string())?;
    let target_path = type_dir.join(format!("{number:04}-{}.md", paths::slugify(title)));

    if store.read(&target_path).is_ok() {
        return Err(format!("{} already exists", target_path.display()));
    }

    let filled = fill_frontmatter(template, frontmatter_type, timestamp);
    let filled = fill_frontmatter_title(&filled, title);
    Ok((target_path, filled))
}

/// Plans `new`'s target path and filled content, timestamped now, without
/// writing it — the counterpart a caller uses when it owns its own write
/// (e.g. a transactional write+check verb) instead of going through
/// [`scaffold`]'s call to [`crate::store::DocStore::write`].
pub fn plan(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
) -> Result<(PathBuf, String), String> {
    plan_at(store, docs_dir, doc_type, title, &now_iso8601())
}

fn scaffold(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    timestamp: &str,
) -> Result<PathBuf, String> {
    let (target_path, filled) = plan_at(store, docs_dir, doc_type, title, timestamp)?;
    store
        .write(&target_path, &filled)
        .map_err(|e| e.to_string())?;
    Ok(target_path)
}

pub(crate) fn unsupported_type_message(doc_type: &str) -> String {
    format!("unsupported doc type '{doc_type}' (expected one of adr, bdr, prd, issue)")
}

/// Targeted line-edit fill of `type`/`status`/`timestamp` inside the leading
/// frontmatter block only — never a serde round-trip, so body placeholders
/// and frontmatter guidance comments outside those three keys survive
/// byte-for-byte.
pub(crate) fn fill_frontmatter(template: &str, type_value: &str, timestamp: &str) -> String {
    let lines: Vec<&str> = template.lines().collect();
    let Some(close) = frontmatter_close_index(&lines) else {
        return template.to_string();
    };

    let filled: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, &line)| {
            if i == 0 || i >= close {
                line.to_string()
            } else {
                fill_frontmatter_line(line, type_value, timestamp)
            }
        })
        .collect();

    filled.join("\n") + "\n"
}

pub(crate) fn frontmatter_close_index(lines: &[&str]) -> Option<usize> {
    lines
        .iter()
        .skip(1)
        .position(|&l| l == "---")
        .map(|i| i + 1)
}

/// Fills the frontmatter `title:` line with `title`, quoted exactly as
/// [`crate::record::to_canonical_markdown`] would (via
/// [`format_scalar`]) — never a local quoting rule — so a fresh scaffold's
/// frontmatter is already a canonical-check fixed point (ADR 0019). Shared
/// with [`crate::commands::brief::run`], which applies the same fill on top
/// of its own pre-filled sections.
pub(crate) fn fill_frontmatter_title(content: &str, title: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let Some(close) = frontmatter_close_index(&lines) else {
        return content.to_string();
    };

    let filled: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, &line)| {
            if i == 0 || i >= close {
                line.to_string()
            } else {
                replace_targeted_value(line, "title", &format_scalar(title))
                    .unwrap_or_else(|| line.to_string())
            }
        })
        .collect();

    filled.join("\n") + "\n"
}

fn fill_frontmatter_line(line: &str, type_value: &str, timestamp: &str) -> String {
    replace_targeted_value(line, "type", type_value)
        .or_else(|| replace_targeted_value(line, "status", "Proposed"))
        .or_else(|| replace_targeted_value(line, "timestamp", timestamp))
        .unwrap_or_else(|| line.to_string())
}

/// Replaces the value of a `key: value` frontmatter line, preserving any
/// trailing `# guidance comment` verbatim.
pub(crate) fn replace_targeted_value(line: &str, key: &str, new_value: &str) -> Option<String> {
    let prefix = format!("{key}:");
    let rest = line.strip_prefix(&prefix)?;
    match rest.find('#') {
        Some(hash_idx) => Some(format!("{prefix} {new_value} {}", &rest[hash_idx..])),
        None => Some(format!("{prefix} {new_value}")),
    }
}

pub(crate) fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the unix epoch")
        .as_secs();
    let (year, month, day) = civil_date_from_unix_days((secs / 86_400) as i64);
    let time_of_day = secs % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        time_of_day / 3_600,
        (time_of_day % 3_600) / 60,
        time_of_day % 60
    )
}

/// Days-since-epoch to (year, month, day) via Howard Hinnant's
/// `civil_from_days` (proleptic Gregorian) — the only way to produce an
/// ISO-8601 date from `std` alone, since this slice adds no chrono
/// dependency.
fn civil_date_from_unix_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::io;

    #[test]
    fn fill_frontmatter_sets_type_status_and_timestamp() {
        let template = "---\ntype: ADR\nstatus: Proposed            # Proposed | Accepted\ntimestamp: <ISO 8601 datetime>\n---\n\n# Body\n<placeholder>\n";
        let filled = fill_frontmatter(template, "ADR", "2026-07-14T00:00:00Z");

        assert!(filled.contains("type: ADR"));
        assert!(filled.contains("status: Proposed"));
        assert!(filled.contains("timestamp: 2026-07-14T00:00:00Z"));
    }

    #[test]
    fn fill_frontmatter_preserves_the_guidance_comment_verbatim() {
        let template = "---\ntype: ADR\nstatus: Proposed            # Proposed | Accepted | Superseded | Deprecated\ntimestamp: <ISO 8601 datetime>\n---\n\n# Body\n";
        let filled = fill_frontmatter(template, "ADR", "2026-07-14T00:00:00Z");

        assert!(filled.contains("# Proposed | Accepted | Superseded | Deprecated"));
    }

    #[test]
    fn fill_frontmatter_leaves_the_body_untouched() {
        let template = "---\ntype: BDR\nstatus: Draft               # Draft | Accepted\ntimestamp: <ISO 8601 datetime>\n---\n\n<!-- Status lives in frontmatter (`status`), not a body line. -->\n<Replace the diagram above with a flowchart...>\n";
        let filled = fill_frontmatter(template, "BDR", "2026-07-14T00:00:00Z");

        assert!(
            filled.contains("<!-- Status lives in frontmatter (`status`), not a body line. -->")
        );
        assert!(filled.contains("<Replace the diagram above with a flowchart...>"));
    }

    #[test]
    fn fill_frontmatter_without_a_closing_fence_returns_the_template_unchanged() {
        let template = "no frontmatter here\n";
        assert_eq!(
            fill_frontmatter(template, "ADR", "2026-07-14T00:00:00Z"),
            template
        );
    }

    #[test]
    fn fill_frontmatter_title_replaces_the_placeholder_with_the_argument() {
        let template =
            "---\ntype: ADR\ntitle: <Short decision title>\nstatus: Proposed\n---\n\n# Body\n";
        let filled = fill_frontmatter_title(template, "My Decision");

        assert!(filled.contains("title: My Decision\n"));
        assert!(!filled.contains("<Short decision title>"));
    }

    #[test]
    fn fill_frontmatter_title_quotes_exactly_as_the_canonical_serializer_would() {
        let template =
            "---\ntype: ADR\ntitle: <Short decision title>\nstatus: Proposed\n---\n\n# Body\n";
        let filled = fill_frontmatter_title(template, "Caching: A Deep Dive");

        assert!(filled.contains(&format!(
            "title: {}\n",
            format_scalar("Caching: A Deep Dive")
        )));
    }

    #[test]
    fn fill_frontmatter_title_leaves_the_body_untouched() {
        let template =
            "---\ntype: Issue\ntitle: <Issue title>\n---\n\n## <Issue title>\n\n<intro guidance>\n";
        let filled = fill_frontmatter_title(template, "Fix It");

        assert!(filled.contains("## <Issue title>"));
        assert!(filled.contains("<intro guidance>"));
    }

    #[test]
    fn fill_frontmatter_title_without_a_closing_fence_returns_the_content_unchanged() {
        let content = "no frontmatter here\n";
        assert_eq!(fill_frontmatter_title(content, "My Decision"), content);
    }

    #[test]
    fn civil_date_from_unix_days_matches_known_calendar_dates() {
        assert_eq!(civil_date_from_unix_days(0), (1970, 1, 1));
        assert_eq!(civil_date_from_unix_days(1), (1970, 1, 2));
        assert_eq!(civil_date_from_unix_days(31), (1970, 2, 1));
    }

    #[test]
    fn now_iso8601_has_the_expected_shape() {
        let timestamp = now_iso8601();
        assert_eq!(timestamp.len(), 20);
        assert_eq!(&timestamp[4..5], "-");
        assert_eq!(&timestamp[7..8], "-");
        assert_eq!(&timestamp[10..11], "T");
        assert_eq!(&timestamp[13..14], ":");
        assert_eq!(&timestamp[16..17], ":");
        assert_eq!(&timestamp[19..20], "Z");
    }

    #[test]
    fn unsupported_type_message_names_the_offending_type() {
        assert!(unsupported_type_message("constitution").contains("constitution"));
    }

    /// A minimal in-memory [`DocStore`] test double, so `scaffold`'s tests
    /// need no filesystem at all — `living-docs-core` depends on no
    /// concrete adapter (issue 0006 slice 0006-D2).
    struct MapStore {
        files: RefCell<BTreeMap<PathBuf, String>>,
    }

    impl MapStore {
        fn new() -> Self {
            Self {
                files: RefCell::new(BTreeMap::new()),
            }
        }

        fn seeded(seed: &[(&str, &str)]) -> Self {
            let files = seed
                .iter()
                .map(|(path, contents)| (PathBuf::from(path), (*contents).to_string()))
                .collect();
            Self {
                files: RefCell::new(files),
            }
        }
    }

    impl DocStore for MapStore {
        fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(self
                .files
                .borrow()
                .keys()
                .filter(|path| path.starts_with(root))
                .cloned()
                .collect())
        }

        fn read(&self, path: &Path) -> io::Result<String> {
            self.files
                .borrow()
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
        }

        fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
            self.files
                .borrow_mut()
                .insert(path.to_path_buf(), contents.to_string());
            Ok(())
        }
    }

    #[test]
    fn scaffold_allocates_number_one_in_an_empty_type_directory() {
        let store = MapStore::new();

        let target = scaffold(
            &store,
            Path::new("/bundle"),
            "adr",
            "First Decision",
            "2026-07-17T00:00:00Z",
        )
        .expect("scaffold should succeed");

        assert_eq!(target, PathBuf::from("/bundle/adr/0001-first-decision.md"));
    }

    #[test]
    fn scaffold_allocates_max_existing_number_plus_one_through_next_number_from_store() {
        let store = MapStore::seeded(&[
            ("/bundle/adr/0001-first.md", "content"),
            ("/bundle/adr/0004-fourth.md", "content"),
        ]);

        let target = scaffold(
            &store,
            Path::new("/bundle"),
            "adr",
            "Fifth Decision",
            "2026-07-17T00:00:00Z",
        )
        .expect("scaffold should succeed");

        assert_eq!(target, PathBuf::from("/bundle/adr/0005-fifth-decision.md"));
    }

    #[test]
    fn scaffold_persists_the_filled_record_through_the_stores_write_method() {
        let store = MapStore::new();

        let target = scaffold(
            &store,
            Path::new("/bundle"),
            "adr",
            "Persisted Decision",
            "2026-07-17T00:00:00Z",
        )
        .expect("scaffold should succeed");

        let persisted = store
            .read(&target)
            .expect("scaffold must persist through DocStore::write");
        assert!(persisted.contains("type: ADR"));
        assert!(persisted.contains("status: Proposed"));
        assert!(persisted.contains("timestamp: 2026-07-17T00:00:00Z"));
        assert!(persisted.contains("title: Persisted Decision"));
    }

    /// `list` deliberately omits the record `read` still serves, simulating
    /// a store whose enumeration and lookup can disagree — proving the
    /// clobber guard checks `DocStore::read` directly rather than trusting
    /// `DocStore::list`'s allocation to have already ruled the path out.
    struct StaleListingStore {
        files: BTreeMap<PathBuf, String>,
    }

    impl DocStore for StaleListingStore {
        fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(Vec::new())
        }

        fn read(&self, path: &Path) -> io::Result<String> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
        }

        fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn scaffold_refuses_to_clobber_a_path_the_store_already_serves_even_when_listing_omits_it() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-first-decision.md"),
            "existing".to_string(),
        );
        let store = StaleListingStore { files };

        let err = scaffold(
            &store,
            Path::new("/bundle"),
            "adr",
            "First Decision",
            "2026-07-17T00:00:00Z",
        )
        .expect_err("clobbering an existing store record must fail");

        assert!(err.contains("already exists"), "got: {err}");
    }
}
