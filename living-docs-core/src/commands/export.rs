//! `living-docs export`: materializes every record the active [`DocStore`]
//! lists back into conformant `.md` files on disk — the lossless
//! round-trip fitness function for issue 0006 slice 0006-D2 (ADR 0007).
//! `source` may be either backend: for a filesystem-backed store this is a
//! plain copy; for a database-backed store each record is read back
//! through its own canonical serializer, so the emitted tree is that
//! backend's byte-stable projection. With `visibility_filter` set (ADR
//! 0010), only records whose effective visibility is in the set are
//! materialized — the default-deny allowlist build.

use crate::frontmatter;
use crate::store::DocStore;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

pub fn export(
    source: &dyn DocStore,
    source_root: &Path,
    out_dir: &Path,
    visibility_filter: Option<Vec<String>>,
) -> ExitCode {
    match export_records(source, source_root, out_dir, visibility_filter.as_deref()) {
        Ok(count) => {
            println!("Exported {count} record(s) to {}", out_dir.display());
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs export: {message}");
            ExitCode::from(2)
        }
    }
}

fn export_records(
    source: &dyn DocStore,
    source_root: &Path,
    out_dir: &Path,
    visibility_filter: Option<&[String]>,
) -> Result<usize, String> {
    let paths = source.list(source_root).map_err(|e| e.to_string())?;
    let mut exported = 0;
    for path in &paths {
        if export_one(source, source_root, path, out_dir, visibility_filter)? {
            exported += 1;
        }
    }
    Ok(exported)
}

/// The default-deny fallback effective visibility for a record whose
/// frontmatter carries no `visibility` key at all — matches the check-side
/// (slice 1) and index-side (slice 2) domain, so a record with no
/// visibility is never exported under an allowlist filter unless `private`
/// is explicitly named.
const DEFAULT_VISIBILITY: &str = "private";

fn effective_visibility(contents: &str) -> String {
    frontmatter::read_scalar_from_str(contents, "visibility")
        .unwrap_or_else(|| DEFAULT_VISIBILITY.to_string())
}

fn record_visible(contents: &str, filter: Option<&[String]>) -> bool {
    match filter {
        None => true,
        Some(allowed) => allowed.contains(&effective_visibility(contents)),
    }
}

fn export_one(
    source: &dyn DocStore,
    source_root: &Path,
    path: &Path,
    out_dir: &Path,
    visibility_filter: Option<&[String]>,
) -> Result<bool, String> {
    let contents = source.read(path).map_err(|e| e.to_string())?;
    if !record_visible(&contents, visibility_filter) {
        return Ok(false);
    }

    let relative = path.strip_prefix(source_root).map_err(|_| {
        format!(
            "listed path {} is not under the source root {}",
            path.display(),
            source_root.display()
        )
    })?;
    let target = out_dir.join(relative);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&target, contents).map_err(|e| e.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::io;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct MapStore {
        files: BTreeMap<PathBuf, String>,
    }

    impl DocStore for MapStore {
        fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(self
                .files
                .keys()
                .filter(|path| path.starts_with(root))
                .cloned()
                .collect())
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

    struct FailingListStore;

    impl DocStore for FailingListStore {
        fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
            Err(io::Error::new(io::ErrorKind::Other, "listing unavailable"))
        }

        fn read(&self, _path: &Path) -> io::Result<String> {
            Err(io::Error::new(io::ErrorKind::NotFound, "unused"))
        }

        fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
            Ok(())
        }
    }

    struct OutOfRootStore {
        listed_path: PathBuf,
    }

    impl DocStore for OutOfRootStore {
        fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(vec![self.listed_path.clone()])
        }

        fn read(&self, _path: &Path) -> io::Result<String> {
            Ok("---\ntype: ADR\n---\n# Escaped\n".to_string())
        }

        fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
            Ok(())
        }
    }

    fn temp_out_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("living-docs-core-export-{label}-{nanos}"))
    }

    fn exit_code_is_success(code: ExitCode) -> bool {
        format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
    }

    #[test]
    fn export_materializes_every_listed_record_under_out_dir_preserving_its_relative_path() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-first.md"),
            "---\ntype: ADR\n---\n# First\n".to_string(),
        );
        files.insert(
            PathBuf::from("/bundle/bdr/0001-second.md"),
            "---\ntype: BDR\n---\n# Second\n".to_string(),
        );
        let store = MapStore { files };
        let out_dir = temp_out_dir("basic");

        let code = export(&store, Path::new("/bundle"), &out_dir, None);

        assert!(exit_code_is_success(code));
        assert_eq!(
            fs::read_to_string(out_dir.join("adr/0001-first.md")).expect("adr record exported"),
            "---\ntype: ADR\n---\n# First\n"
        );
        assert_eq!(
            fs::read_to_string(out_dir.join("bdr/0001-second.md")).expect("bdr record exported"),
            "---\ntype: BDR\n---\n# Second\n"
        );

        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn export_is_idempotent_byte_identical_on_a_second_run() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-first.md"),
            "---\ntype: ADR\n---\n# First\n".to_string(),
        );
        let store = MapStore { files };
        let out_dir = temp_out_dir("idempotent");

        export(&store, Path::new("/bundle"), &out_dir, None);
        let first = fs::read(out_dir.join("adr/0001-first.md")).expect("first export written");

        export(&store, Path::new("/bundle"), &out_dir, None);
        let second = fs::read(out_dir.join("adr/0001-first.md")).expect("second export written");

        assert_eq!(first, second, "export is not idempotent");

        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn export_of_an_empty_store_succeeds_with_zero_records() {
        let store = MapStore {
            files: BTreeMap::new(),
        };
        let out_dir = temp_out_dir("empty");

        let code = export(&store, Path::new("/bundle"), &out_dir, None);

        assert!(exit_code_is_success(code));

        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn export_fails_cleanly_when_the_source_store_errors_on_list() {
        let out_dir = temp_out_dir("list-fails");

        let code = export(&FailingListStore, Path::new("/bundle"), &out_dir, None);

        assert!(!exit_code_is_success(code));

        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn export_fails_and_writes_nothing_when_a_listed_path_escapes_source_root() {
        let escaped_root = temp_out_dir("escaped-record");
        let escaped_path = escaped_root.join("adr/0001-escaped.md");
        let store = OutOfRootStore {
            listed_path: escaped_path.clone(),
        };
        let out_dir = temp_out_dir("out-of-root");

        let code = export(&store, Path::new("/bundle"), &out_dir, None);

        assert!(!exit_code_is_success(code));
        assert!(
            !escaped_path.exists(),
            "export must not write outside out_dir via an absolute-path fallback"
        );
        assert!(
            !out_dir.exists(),
            "export must not create out_dir at all when every listed path escapes source_root"
        );

        let _ = fs::remove_dir_all(&out_dir);
        let _ = fs::remove_dir_all(&escaped_root);
    }

    fn record_with_visibility(name: &str, visibility_line: &str) -> (PathBuf, String) {
        (
            PathBuf::from(format!("/bundle/adr/{name}.md")),
            format!("---\ntype: ADR\n{visibility_line}---\n# {name}\n"),
        )
    }

    #[test]
    fn export_with_a_visibility_filter_writes_only_matching_records() {
        let mut files = BTreeMap::new();
        let (public_path, public_contents) =
            record_with_visibility("0001-public", "visibility: public\n");
        let (showcase_path, showcase_contents) =
            record_with_visibility("0002-showcase", "visibility: showcase\n");
        let (private_path, private_contents) =
            record_with_visibility("0003-private", "visibility: private\n");
        files.insert(public_path, public_contents);
        files.insert(showcase_path, showcase_contents);
        files.insert(private_path, private_contents);
        let store = MapStore { files };
        let out_dir = temp_out_dir("filtered");

        let code = export(
            &store,
            Path::new("/bundle"),
            &out_dir,
            Some(vec!["public".to_string(), "showcase".to_string()]),
        );

        assert!(exit_code_is_success(code));
        assert!(out_dir.join("adr/0001-public.md").exists());
        assert!(out_dir.join("adr/0002-showcase.md").exists());
        assert!(!out_dir.join("adr/0003-private.md").exists());

        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn export_without_a_visibility_filter_still_writes_every_record() {
        let mut files = BTreeMap::new();
        let (public_path, public_contents) =
            record_with_visibility("0001-public", "visibility: public\n");
        let (private_path, private_contents) =
            record_with_visibility("0002-private", "visibility: private\n");
        files.insert(public_path, public_contents);
        files.insert(private_path, private_contents);
        let store = MapStore { files };
        let out_dir = temp_out_dir("unfiltered");

        let code = export(&store, Path::new("/bundle"), &out_dir, None);

        assert!(exit_code_is_success(code));
        assert!(out_dir.join("adr/0001-public.md").exists());
        assert!(out_dir.join("adr/0002-private.md").exists());

        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn export_default_deny_treats_an_absent_visibility_record_as_private() {
        let mut files = BTreeMap::new();
        let (absent_path, absent_contents) = record_with_visibility("0001-absent", "");
        files.insert(absent_path.clone(), absent_contents.clone());
        let store = MapStore {
            files: files.clone(),
        };

        let out_dir_public = temp_out_dir("default-deny-public");
        export(
            &store,
            Path::new("/bundle"),
            &out_dir_public,
            Some(vec!["public".to_string()]),
        );
        assert!(!out_dir_public.join("adr/0001-absent.md").exists());

        let out_dir_private = temp_out_dir("default-deny-private");
        export(
            &store,
            Path::new("/bundle"),
            &out_dir_private,
            Some(vec!["private".to_string()]),
        );
        assert_eq!(
            fs::read_to_string(out_dir_private.join("adr/0001-absent.md"))
                .expect("private filter admits the absent-visibility record"),
            absent_contents
        );

        let _ = fs::remove_dir_all(&out_dir_public);
        let _ = fs::remove_dir_all(&out_dir_private);
    }

    #[test]
    fn export_with_a_visibility_filter_is_idempotent_byte_identical_on_a_second_run() {
        let mut files = BTreeMap::new();
        let (public_path, public_contents) =
            record_with_visibility("0001-public", "visibility: public\n");
        files.insert(public_path, public_contents);
        let store = MapStore { files };
        let out_dir = temp_out_dir("filtered-idempotent");
        let filter = || Some(vec!["public".to_string()]);

        export(&store, Path::new("/bundle"), &out_dir, filter());
        let first = fs::read(out_dir.join("adr/0001-public.md")).expect("first export written");

        export(&store, Path::new("/bundle"), &out_dir, filter());
        let second = fs::read(out_dir.join("adr/0001-public.md")).expect("second export written");

        assert_eq!(first, second, "filtered export is not idempotent");

        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn export_with_a_visibility_filter_reports_only_the_written_count() {
        let mut files = BTreeMap::new();
        let (public_path, public_contents) =
            record_with_visibility("0001-public", "visibility: public\n");
        let (private_path, private_contents) =
            record_with_visibility("0002-private", "visibility: private\n");
        files.insert(public_path, public_contents);
        files.insert(private_path, private_contents);
        let store = MapStore { files };
        let out_dir = temp_out_dir("filtered-count");

        let count = export_records(
            &store,
            Path::new("/bundle"),
            &out_dir,
            Some(&["public".to_string()]),
        )
        .expect("export_records should succeed");

        assert_eq!(count, 1);

        let _ = fs::remove_dir_all(&out_dir);
    }
}
