//! Provenance-seal check (ADR 0039): active only when the repository has a
//! seal key (`living-docs seal init`), silent otherwise — fail-open like
//! the write-time hook. When active, every record in the CLI-owned scope
//! (type directories + bundle-root singletons, the same scope as the
//! canonical-frontmatter check) must carry a ledger seal matching its
//! CLI-owned frontmatter; a missing or stale seal is a violation naming the
//! re-issue path.

use super::{canonical, is_bundle_singleton, records, Reporter};
use crate::seal;
use crate::store::DocStore;
use std::path::{Path, PathBuf};

pub(crate) fn check_seals(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    let Some(seal_dir) = seal::seal_dir_for(bundle) else {
        return;
    };
    let Some(key) = seal::load_key(&seal_dir) else {
        return;
    };
    let ledger = seal::read_ledger(&seal_dir);
    for path in owned_records(bundle, all_md) {
        check_record_seal(store, &seal_dir, &key, &ledger, &path, reporter);
    }
}

fn owned_records(bundle: &Path, all_md: &[PathBuf]) -> Vec<PathBuf> {
    all_md
        .iter()
        .filter(|path| !records::is_reserved(&super::file_name_str(path)))
        .filter(|path| canonical::in_cli_owned_dir(path) || is_bundle_singleton(bundle, path))
        .cloned()
        .collect()
}

fn check_record_seal(
    store: &dyn DocStore,
    seal_dir: &Path,
    key: &[u8],
    ledger: &std::collections::BTreeMap<String, String>,
    path: &Path,
    reporter: &mut Reporter,
) {
    let Ok(contents) = store.read(path) else {
        return;
    };
    let Some(entry) = seal::ledger_key(seal_dir, path) else {
        return;
    };
    match ledger.get(&entry) {
        None => reporter.report(
            path,
            "SEAL no provenance seal — record was created outside the CLI; re-issue it via `living-docs new`, or re-baseline a trusted tree with `living-docs seal init` (ADR 0039)",
        ),
        Some(stored) if *stored != seal::seal_value(key, &entry, &contents) => reporter.report(
            path,
            "SEAL CLI-owned frontmatter does not match its provenance seal — edited outside the CLI; redo the change via `living-docs status`/`supersede`/`fmt`, or re-baseline with `living-docs seal init` (ADR 0039)",
        ),
        Some(_) => {}
    }
}
