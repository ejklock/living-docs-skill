//! The pinned mermaid bundle: its version, download URL and checksums, where
//! it lives in the user data directory, and a download that is verified
//! before it is placed. The binary carries no HTTP client; `curl` fetches.

use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) const VERSION: &str = "12.3.0";
pub(crate) const URL: &str = "https://cdn.jsdelivr.net/npm/mermaid@12.3.0/dist/mermaid.min.js";
pub(crate) const SHA256: &str = "336df803d87a38b70a01e736e666c8f7601d2ce42f5e47c8fafe5d721074b9a3";
/// The Subresource Integrity value of the pinned file, for an HTML page that
/// references it.
#[allow(dead_code)]
pub(crate) const SRI: &str =
    "sha384-HHW54r639fK42Ip0coGF3mz8oSiGn0eEoWNTLs75X7H63pLSDBa0Tqs0RwGmLlXE";
pub(crate) const FILE_NAME: &str = "mermaid.min.js";

/// The user data directory: `$XDG_DATA_HOME` when it is set to an absolute
/// path, else `$HOME/.local/share`. An empty or relative `XDG_DATA_HOME` is
/// ignored, as the XDG specification requires; `None` when neither resolves.
fn data_dir(xdg_data_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let xdg = xdg_data_home.map(PathBuf::from);
    if let Some(path) = xdg.filter(|path| path.is_absolute()) {
        return Some(path);
    }
    let home = home.filter(|value| !value.is_empty())?;
    Some(PathBuf::from(home).join(".local/share"))
}

/// The directory holding the pinned version's file, under `data_home`.
fn install_dir_in(data_home: &Path) -> PathBuf {
    data_home.join("living-docs/mermaid").join(VERSION)
}

/// The full path of the installed mermaid file, resolved from the process
/// environment.
pub(crate) fn installed_path() -> Result<PathBuf, String> {
    let data_home = data_dir(std::env::var_os("XDG_DATA_HOME"), std::env::var_os("HOME"))
        .ok_or("cannot resolve the user data directory: set XDG_DATA_HOME or HOME")?;
    Ok(install_dir_in(&data_home).join(FILE_NAME))
}

/// Reads the installed mermaid file at `path` as text, only when its
/// SHA-256 equals `expected`. `Ok(None)` when no file is installed there; an
/// unreadable, non-UTF-8 or altered file is an error, never a silent skip.
pub(crate) fn read_verified(path: &Path, expected: &str) -> Result<Option<String>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    let actual = hex_digest(&bytes);
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(format!(
            "the installed mermaid at {} does not match the pinned SHA-256 (expected {expected}, found {actual}); run `living-docs manage install mermaid` to replace it",
            path.display()
        ));
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| format!("{} is not valid UTF-8", path.display()))
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// Runs `curl -fsSL -o <partial> <url>` as an argument vector, returning the
/// reason it could not deliver the bytes.
fn fetch(curl: &Path, url: &str, partial: &Path) -> Result<(), String> {
    let status = Command::new(curl)
        .args(["-fsSL", "-o"])
        .arg(partial)
        .arg(url)
        .status()
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => {
                "curl not found on PATH; install curl to download mermaid".to_string()
            }
            _ => format!("cannot run curl: {error}"),
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("curl failed downloading {url} ({status})"))
    }
}

fn verify_and_place(partial: &Path, expected: &str, dest: &Path) -> Result<(), String> {
    let bytes = fs::read(partial).map_err(|error| format!("cannot read the download: {error}"))?;
    let actual = hex_digest(&bytes);
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(format!(
            "SHA-256 mismatch: expected {expected}, downloaded {actual}"
        ));
    }
    fs::rename(partial, dest).map_err(|error| format!("cannot place {}: {error}", dest.display()))
}

/// Downloads `url` with `curl` into `dest`, placing the file only when its
/// SHA-256 equals `expected`. The bytes land in a temporary file beside
/// `dest` and are renamed into place, so `dest` is never partly written; on
/// any failure the temporary file is removed and `dest` is untouched.
pub(crate) fn download_verified(
    curl: &Path,
    url: &str,
    expected: &str,
    dest: &Path,
) -> Result<(), String> {
    let dir = dest.parent().ok_or("the destination has no directory")?;
    fs::create_dir_all(dir).map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
    let partial = dir.join(format!(".{FILE_NAME}.{}.part", std::process::id()));
    let result =
        fetch(curl, url, &partial).and_then(|()| verify_and_place(&partial, expected, dest));
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result
}

#[cfg(test)]
mod tests;
