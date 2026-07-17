//! `living-docs export`: materializes every record the active [`DocStore`]
//! lists back into conformant `.md` files on disk — the lossless
//! round-trip fitness function for issue 0006 slice 0006-D2 (ADR 0007).
//! `source` may be either backend: for a filesystem-backed store this is a
//! plain copy; for a database-backed store each record is read back
//! through its own canonical serializer, so the emitted tree is that
//! backend's byte-stable projection.

use crate::store::DocStore;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

pub fn export(source: &dyn DocStore, source_root: &Path, out_dir: &Path) -> ExitCode {
    match export_records(source, source_root, out_dir) {
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
) -> Result<usize, String> {
    let paths = source.list(source_root).map_err(|e| e.to_string())?;
    for path in &paths {
        export_one(source, source_root, path, out_dir)?;
    }
    Ok(paths.len())
}

fn export_one(
    source: &dyn DocStore,
    source_root: &Path,
    path: &Path,
    out_dir: &Path,
) -> Result<(), String> {
    let contents = source.read(path).map_err(|e| e.to_string())?;
    let relative = path.strip_prefix(source_root).unwrap_or(path);
    let target = out_dir.join(relative);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&target, contents).map_err(|e| e.to_string())
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

        let code = export(&store, Path::new("/bundle"), &out_dir);

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

        export(&store, Path::new("/bundle"), &out_dir);
        let first = fs::read(out_dir.join("adr/0001-first.md")).expect("first export written");

        export(&store, Path::new("/bundle"), &out_dir);
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

        let code = export(&store, Path::new("/bundle"), &out_dir);

        assert!(exit_code_is_success(code));

        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn export_fails_cleanly_when_the_source_store_errors_on_list() {
        let out_dir = temp_out_dir("list-fails");

        let code = export(&FailingListStore, Path::new("/bundle"), &out_dir);

        assert!(!exit_code_is_success(code));

        let _ = fs::remove_dir_all(&out_dir);
    }
}
