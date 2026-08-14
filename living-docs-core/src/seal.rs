//! Provenance seal (ADR 0039): every fs-mode CLI write of a record inside
//! the CLI-owned scope is sealed — HMAC-SHA256 of the record's repo-relative
//! path plus its CLI-owned frontmatter lines, keyed by a per-clone secret —
//! into a ledger under `.git/living-docs/`. Neither key nor ledger is ever
//! committed; a fresh clone starts unsealed (fail-open) until
//! `living-docs seal init` baselines it. Friction, not cryptography: an LLM
//! is never a security boundary, and the ADR says so.

use crate::frontmatter::frontmatter_block;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The frontmatter keys the seal covers — exactly the hook's
/// `CLI_OWNED_KEYS` (`block-docs-handwrite.sh`), pinned to it by a parity
/// test. Body prose, `description`, `tags`, and `kind` stay freely editable
/// and unsealed.
pub const SEALED_KEYS: [&str; 6] = [
    "type",
    "title",
    "status",
    "supersedes",
    "superseded_by",
    "timestamp",
];

/// The seal directory (`<repo>/.git/living-docs`) for the repository
/// containing `start`, found by walking up to the nearest `.git` directory.
/// `None` outside any git repository — sealing is then inactive.
pub fn seal_dir_for(start: &Path) -> Option<PathBuf> {
    let origin = if start.is_dir() {
        start
    } else {
        start.parent()?
    };
    let mut dir = fs::canonicalize(origin).ok()?;
    loop {
        if dir.join(".git").is_dir() {
            return Some(dir.join(".git").join("living-docs"));
        }
        dir = dir.parent()?.to_path_buf();
    }
}

pub fn load_key(seal_dir: &Path) -> Option<Vec<u8>> {
    fs::read(seal_dir.join("seal.key"))
        .ok()
        .filter(|k| !k.is_empty())
}

/// Generates and stores a fresh random key, replacing any prior one — the
/// re-baseline operation's first half.
pub fn generate_key(seal_dir: &Path) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; 32];
    read_urandom(&mut buf)?;
    fs::create_dir_all(seal_dir).map_err(|e| e.to_string())?;
    fs::write(seal_dir.join("seal.key"), &buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

fn read_urandom(buf: &mut [u8]) -> Result<(), String> {
    use std::io::Read;
    let mut f = fs::File::open("/dev/urandom")
        .map_err(|e| format!("cannot open /dev/urandom for the seal key: {e}"))?;
    f.read_exact(buf)
        .map_err(|e| format!("cannot read the seal key bytes: {e}"))
}

pub fn read_ledger(seal_dir: &Path) -> BTreeMap<String, String> {
    fs::read_to_string(seal_dir.join("seals.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn write_ledger(seal_dir: &Path, ledger: &BTreeMap<String, String>) -> Result<(), String> {
    fs::create_dir_all(seal_dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(ledger).map_err(|e| e.to_string())?;
    fs::write(seal_dir.join("seals.json"), text).map_err(|e| e.to_string())
}

/// The ledger key for `record_path`: its canonical path relative to the
/// repository root the seal directory belongs to. `None` when the record
/// does not resolve under that root (then it cannot be sealed or checked).
pub fn ledger_key(seal_dir: &Path, record_path: &Path) -> Option<String> {
    let root = seal_dir.parent()?.parent()?;
    let canonical = fs::canonicalize(record_path).ok()?;
    let relative = canonical.strip_prefix(fs::canonicalize(root).ok()?).ok()?;
    Some(relative.to_string_lossy().to_string())
}

/// The seal value: hex HMAC-SHA256 over `<ledger key>\0<owned lines>`.
pub fn seal_value(key: &[u8], ledger_key: &str, contents: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("hmac accepts any key length");
    mac.update(ledger_key.as_bytes());
    mac.update(b"\0");
    mac.update(owned_lines(contents).as_bytes());
    hex(&mac.finalize().into_bytes())
}

/// The CLI-owned frontmatter lines of `contents`, in document order — the
/// exact byte content the seal covers.
pub fn owned_lines(contents: &str) -> String {
    let Some(block) = frontmatter_block(contents) else {
        return String::new();
    };
    block
        .lines()
        .filter(|line| {
            SEALED_KEYS
                .iter()
                .any(|key| line.starts_with(&format!("{key}:")))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Upserts `record_path`'s seal after a CLI write. A no-op unless a key
/// already exists (the feature is opt-in via `seal init`) and the record
/// resolves under the seal directory's repository root.
pub fn seal_record(seal_dir: &Path, record_path: &Path, contents: &str) {
    let Some(key) = load_key(seal_dir) else {
        return;
    };
    let Some(entry) = ledger_key(seal_dir, record_path) else {
        return;
    };
    let mut ledger = read_ledger(seal_dir);
    ledger.insert(entry.clone(), seal_value(&key, &entry, contents));
    let _ = write_ledger(seal_dir, &ledger);
}

/// Baselines the repository containing `docs_dir`: a fresh key and a ledger
/// sealing every record `store` lists — asserting "trusted from here on",
/// which is what makes adoption, git merges, and fresh containers workable.
pub fn init(store: &dyn crate::store::DocStore, docs_dir: &Path) -> Result<usize, String> {
    let seal_dir = seal_dir_for(docs_dir).ok_or_else(|| {
        "not inside a git repository — the seal lives under .git/living-docs".to_string()
    })?;
    let key = generate_key(&seal_dir)?;
    let mut ledger = BTreeMap::new();
    for path in store.list(docs_dir).map_err(|e| e.to_string())? {
        let Ok(contents) = store.read(&path) else {
            continue;
        };
        let Some(entry) = ledger_key(&seal_dir, &path) else {
            continue;
        };
        ledger.insert(entry.clone(), seal_value(&key, &entry, &contents));
    }
    write_ledger(&seal_dir, &ledger)?;
    Ok(ledger.len())
}

#[cfg(test)]
mod tests;
