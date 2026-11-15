//! `manage html` verb wrapper: renders the bundle with
//! `living_docs_core::html::render` and replaces one local file atomically.
//! mermaid.js comes from `--mermaid`, else the installed copy when its
//! SHA-256 is the pinned one, else the pinned CDN tag; a present but altered
//! installed copy is an error, never a fallback. The bundle is only read.

use crate::mermaid_asset;
use crate::output::{self, OutputMode};
use crate::store::build_store;
use living_docs_core::html::{render, MermaidSource};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const DEFAULT_OUT: &str = ".living-docs/html/index.html";

#[derive(Serialize)]
struct WrittenJson<'a> {
    path: &'a Path,
    mermaid: &'a str,
}

/// The mermaid text or tag chosen, with the words that name where it came from.
struct Chosen {
    source: MermaidSource,
    label: String,
}

pub(crate) fn run_html(
    docs_dir: &Path,
    out: Option<PathBuf>,
    mermaid: Option<PathBuf>,
    mode: OutputMode,
    quiet: bool,
) -> ExitCode {
    let out = out.unwrap_or_else(|| PathBuf::from(DEFAULT_OUT));
    let result = choose_mermaid(mermaid.as_deref(), installed_candidate().as_deref())
        .and_then(|chosen| write_page(docs_dir, &out, &chosen).map(|()| chosen));
    match result {
        Ok(chosen) => {
            if !quiet {
                eprintln!("mermaid: {}", chosen.label);
            }
            render_written(&out, &chosen.label, mode);
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs manage html: {message}");
            ExitCode::from(2)
        }
    }
}

fn installed_candidate() -> Option<PathBuf> {
    mermaid_asset::installed_path().ok()
}

fn choose_mermaid(flag: Option<&Path>, installed: Option<&Path>) -> Result<Chosen, String> {
    choose_mermaid_verified(flag, installed, mermaid_asset::SHA256)
}

/// `expected` is the SHA-256 an installed copy must have to be inlined.
fn choose_mermaid_verified(
    flag: Option<&Path>,
    installed: Option<&Path>,
    expected: &str,
) -> Result<Chosen, String> {
    if let Some(path) = flag {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("cannot read --mermaid {}: {error}", path.display()))?;
        return Ok(Chosen {
            source: MermaidSource::Inline(text),
            label: path.display().to_string(),
        });
    }
    if let Some(path) = installed {
        if let Some(text) = mermaid_asset::read_verified(path, expected)? {
            return Ok(Chosen {
                source: MermaidSource::Inline(text),
                label: format!("installed {}", mermaid_asset::VERSION),
            });
        }
    }
    Ok(Chosen {
        source: MermaidSource::Pinned,
        label: format!("pinned {}", mermaid_asset::URL),
    })
}

fn write_page(docs_dir: &Path, out: &Path, chosen: &Chosen) -> Result<(), String> {
    let store = build_store(docs_dir);
    let page = render(store.as_ref(), docs_dir, &chosen.source);
    replace_atomically(out, page.as_bytes())
}

/// Writes `bytes` to a temporary file beside `dest` and renames it over
/// `dest`; the temporary file is removed on any failure.
fn replace_atomically(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = match dest.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    fs::create_dir_all(dir).map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
    let name = dest
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let partial = dir.join(format!(".{name}.{}.part", std::process::id()));
    let result = fs::write(&partial, bytes).and_then(|()| fs::rename(&partial, dest));
    if let Err(error) = result {
        let _ = fs::remove_file(&partial);
        return Err(format!("cannot write {}: {error}", dest.display()));
    }
    Ok(())
}

fn render_written(path: &Path, label: &str, mode: OutputMode) {
    if mode.is_json() {
        let json = WrittenJson {
            path,
            mermaid: label,
        };
        output::outln!("{}", output::to_json(&json));
    } else {
        output::outln!("{}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn sha256_hex(text: &str) -> String {
        Sha256::digest(text.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("html-unit-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_installed_copy_with_the_expected_hash_is_inlined() {
        let dir = scratch("match");
        let file = dir.join("mermaid.min.js");
        fs::write(&file, "installed-bytes").unwrap();

        let chosen =
            choose_mermaid_verified(None, Some(&file), &sha256_hex("installed-bytes")).unwrap();

        assert!(
            matches!(chosen.source, MermaidSource::Inline(ref text) if text == "installed-bytes")
        );
        assert_eq!(chosen.label, "installed 12.3.0");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_installed_copy_with_another_hash_is_an_error_not_a_fallback() {
        let dir = scratch("mismatch");
        let file = dir.join("mermaid.min.js");
        fs::write(&file, "installed-bytes").unwrap();

        let error = choose_mermaid_verified(None, Some(&file), &sha256_hex("other"))
            .err()
            .unwrap();

        assert!(error.contains("does not match"), "{error}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_installed_file_falls_back_to_the_pinned_tag() {
        let dir = scratch("absent");

        let chosen = choose_mermaid_verified(None, Some(&dir.join("none.js")), "00").unwrap();

        assert!(matches!(chosen.source, MermaidSource::Pinned));
        let _ = fs::remove_dir_all(&dir);
    }
}
