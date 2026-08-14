use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

struct ScratchBundle {
    root: PathBuf,
}

impl ScratchBundle {
    fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("living-docs-migrate-apply-{label}-{nanos}"));
        fs::create_dir_all(root.join("adr")).expect("scratch adr dir");
        fs::write(root.join("index.md"), "# Docs\n").expect("root index");
        fs::write(root.join("adr").join("0001-a.md"), "original A\n").expect("record");
        Self { root }
    }
}

impl Drop for ScratchBundle {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn restore_rewrites_changed_files_and_deletes_created_ones() {
    let bundle = ScratchBundle::new("restore");
    let snapshot = Snapshot::take(&bundle.root).expect("snapshot");

    fs::write(bundle.root.join("adr").join("0001-a.md"), "mutated\n").expect("mutate");
    fs::write(bundle.root.join("adr").join("0002-new.md"), "created\n").expect("create");
    fs::write(bundle.root.join("adr").join("index.md"), "# ADRs\n").expect("create index");

    snapshot.restore(&bundle.root).expect("restore");

    let restored = fs::read_to_string(bundle.root.join("adr").join("0001-a.md")).expect("read");
    assert_eq!(restored, "original A\n");
    assert!(!bundle.root.join("adr").join("0002-new.md").exists());
    assert!(!bundle.root.join("adr").join("index.md").exists());
}

#[test]
fn restore_is_a_no_op_on_an_untouched_tree() {
    let bundle = ScratchBundle::new("noop");
    let snapshot = Snapshot::take(&bundle.root).expect("snapshot");

    snapshot.restore(&bundle.root).expect("restore");

    let mut current = BTreeMap::new();
    collect_md(&bundle.root, &mut current).expect("collect");
    assert_eq!(current, snapshot.files);
}

#[test]
fn succeeded_distinguishes_success_from_failure_codes() {
    assert!(succeeded(ExitCode::SUCCESS));
    assert!(!succeeded(ExitCode::from(1)));
    assert!(!succeeded(ExitCode::from(2)));
}
