use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

struct ScratchRepo {
    root: PathBuf,
}

impl ScratchRepo {
    fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("living-docs-seal-{label}-{nanos}"));
        fs::create_dir_all(root.join(".git")).expect("scratch .git");
        fs::create_dir_all(root.join("docs").join("adr")).expect("scratch docs");
        Self { root }
    }

    fn seal_dir(&self) -> PathBuf {
        seal_dir_for(&self.root.join("docs")).expect("scratch repo must resolve a seal dir")
    }
}

impl Drop for ScratchRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const RECORD: &str =
    "---\ntype: ADR\ntitle: X\nstatus: Proposed\ntags: [a]\ntimestamp: t\n---\n\nBody.\n";

#[test]
fn sealed_keys_match_the_hooks_cli_owned_keys() {
    let hook = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../skills/living-docs/hooks/block-docs-handwrite.sh"
    ))
    .expect("hook script must exist");

    let expected = format!("CLI_OWNED_KEYS='{}'", SEALED_KEYS.join("|"));
    assert!(
        hook.contains(&expected),
        "hook CLI_OWNED_KEYS disagrees with seal::SEALED_KEYS — expected {expected}"
    );
}

#[test]
fn owned_lines_keep_cli_owned_keys_and_drop_free_ones() {
    let lines = owned_lines(RECORD);

    assert!(lines.contains("type: ADR"));
    assert!(lines.contains("status: Proposed"));
    assert!(lines.contains("timestamp: t"));
    assert!(!lines.contains("tags"));
    assert_eq!(owned_lines("no frontmatter"), "");
}

#[test]
fn seal_value_changes_with_owned_lines_and_key_but_not_with_the_body() {
    let key = b"k1";
    let base = seal_value(key, "docs/adr/0001-x.md", RECORD);

    assert_eq!(
        base,
        seal_value(
            key,
            "docs/adr/0001-x.md",
            &RECORD.replace("Body.", "Other.")
        )
    );
    assert_ne!(
        base,
        seal_value(
            key,
            "docs/adr/0001-x.md",
            &RECORD.replace("Proposed", "Accepted")
        )
    );
    assert_ne!(base, seal_value(b"k2", "docs/adr/0001-x.md", RECORD));
    assert_ne!(base, seal_value(key, "docs/adr/0002-y.md", RECORD));
}

#[test]
fn seal_dir_resolves_through_the_nearest_git_directory_and_none_outside_one() {
    let repo = ScratchRepo::new("dir-resolution");

    let seal_dir = repo.seal_dir();

    assert!(seal_dir.ends_with(".git/living-docs"));
    assert!(seal_dir_for(Path::new("/nonexistent-root-path")).is_none());
}

#[test]
fn seal_record_is_a_no_op_without_a_key_and_upserts_with_one() {
    let repo = ScratchRepo::new("upsert");
    let record = repo.root.join("docs").join("adr").join("0001-x.md");
    fs::write(&record, RECORD).expect("write record");
    let seal_dir = repo.seal_dir();

    seal_record(&seal_dir, &record, RECORD);
    assert!(read_ledger(&seal_dir).is_empty());

    let key = generate_key(&seal_dir).expect("key generation");
    seal_record(&seal_dir, &record, RECORD);

    let ledger = read_ledger(&seal_dir);
    let entry = ledger_key(&seal_dir, &record).expect("record under root");
    assert_eq!(ledger.get(&entry), Some(&seal_value(&key, &entry, RECORD)));
}
