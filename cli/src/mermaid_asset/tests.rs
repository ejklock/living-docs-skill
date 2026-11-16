use super::*;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

const BODY: &str = "mermaid bundle bytes";

/// Spawning a child while another thread holds a script open for writing
/// makes that script fail to exec ("text file busy"); stub-curl tests take
/// this lock so a write and a spawn never overlap.
fn serial() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn scratch(label: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "living-docs-mermaid-asset-{label}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn stub_curl(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("curl");
    let script = format!(
        "#!/bin/sh\nout=\"\"\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = \"-o\" ]; then out=\"$2\"; fi\n  shift\ndone\nprintf '%s' '{body}' > \"$out\"\n"
    );
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn sha256_of(bytes: &str) -> String {
    hex_digest(bytes.as_bytes())
}

fn os(value: &str) -> Option<OsString> {
    Some(OsString::from(value))
}

#[test]
fn matching_bytes_are_placed_at_the_destination() {
    let dir = scratch("match");
    let _serial = serial();
    let curl = stub_curl(&dir, BODY);
    let dest = dir.join("out/mermaid.min.js");

    let result = download_verified(&curl, "https://example.test/m.js", &sha256_of(BODY), &dest);

    assert_eq!(result, Ok(()));
    assert_eq!(fs::read_to_string(&dest).unwrap(), BODY);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mismatching_bytes_are_refused_and_leave_no_file() {
    let dir = scratch("mismatch");
    let _serial = serial();
    let curl = stub_curl(&dir, "tampered");
    let dest = dir.join("out/mermaid.min.js");

    let result = download_verified(&curl, "https://example.test/m.js", &sha256_of(BODY), &dest);

    let message = result.unwrap_err();
    assert!(message.contains("SHA-256 mismatch"), "{message}");
    assert!(!dest.exists());
    let leftovers: Vec<_> = fs::read_dir(dest.parent().unwrap()).unwrap().collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_mismatch_keeps_a_previously_installed_file() {
    let dir = scratch("keeps");
    let _serial = serial();
    let curl = stub_curl(&dir, "tampered");
    let dest = dir.join("out/mermaid.min.js");
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    fs::write(&dest, "previous").unwrap();

    let result = download_verified(&curl, "https://example.test/m.js", &sha256_of(BODY), &dest);

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(&dest).unwrap(), "previous");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_curl_is_reported_by_name() {
    let _serial = serial();
    let dir = scratch("nocurl");
    let dest = dir.join("out/mermaid.min.js");

    let result = download_verified(
        &dir.join("absent-curl"),
        "https://example.test/m.js",
        &sha256_of(BODY),
        &dest,
    );

    assert!(result.unwrap_err().contains("curl"));
    assert!(!dest.exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_digest_matches_the_known_sha256_of_an_empty_input() {
    assert_eq!(
        hex_digest(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn an_absolute_xdg_data_home_wins_over_home() {
    assert_eq!(
        data_dir(os("/xdg/data"), os("/home/u")),
        Some(PathBuf::from("/xdg/data"))
    );
}

#[test]
fn an_unset_xdg_data_home_falls_back_to_home_local_share() {
    assert_eq!(
        data_dir(None, os("/home/u")),
        Some(PathBuf::from("/home/u/.local/share"))
    );
}

#[test]
fn an_empty_xdg_data_home_falls_back_to_home_local_share() {
    assert_eq!(
        data_dir(os(""), os("/home/u")),
        Some(PathBuf::from("/home/u/.local/share"))
    );
}

#[test]
fn a_relative_xdg_data_home_falls_back_to_home_local_share() {
    assert_eq!(
        data_dir(os("relative/data"), os("/home/u")),
        Some(PathBuf::from("/home/u/.local/share"))
    );
}

#[test]
fn no_xdg_data_home_and_no_home_resolves_nothing() {
    assert_eq!(data_dir(None, None), None);
    assert_eq!(data_dir(None, os("")), None);
}

#[test]
fn the_install_dir_nests_the_pinned_version_under_living_docs() {
    assert_eq!(
        install_dir_in(Path::new("/xdg/data")),
        PathBuf::from("/xdg/data/living-docs/mermaid/12.3.0")
    );
}
