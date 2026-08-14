//! ADR 0026 decision point 8: `block-docs-handwrite.sh` cannot read a Rust
//! `const`, so its scope regex and deny message stay a hardcoded bash copy
//! of `doc_type::DOC_TYPES` on purpose (fast, fail-open, binary-independent).
//! The duplication is allowed to survive; drift between the copy and the
//! registry is not. This test reads the hook script from disk and asserts
//! both copies agree with the registry's `Identity::Numbered` and
//! `Identity::Named` rows (the CLI-owned record directories, ADR 0036),
//! compared as sets so a spurious extra entry on either side fails too.

use living_docs_core::doc_type::{Identity, DOC_TYPES};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn hook_source() -> String {
    let path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .join("skills/living-docs/hooks/block-docs-handwrite.sh");
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read hook script at {}: {e}", path.display()))
}

fn alternation_group_after<'a>(source: &'a str, marker: &str) -> &'a str {
    let start = source.find(marker).unwrap_or_else(|| {
        panic!("hook script marker `{marker}` not found — did the hook's structure change?")
    }) + marker.len();
    let rest = &source[start..];
    let end = rest.find([')', '>']).unwrap_or_else(|| {
        panic!("hook script alternation group after `{marker}` has no closing delimiter")
    });
    &rest[..end]
}

fn scope_regex_directories(source: &str) -> BTreeSet<String> {
    alternation_group_after(source, "\"$BUNDLE\"/(")
        .split('|')
        .map(str::to_owned)
        .collect()
}

fn deny_message_tokens(source: &str) -> BTreeSet<String> {
    alternation_group_after(source, "living-docs new <")
        .split('|')
        .map(str::to_owned)
        .collect()
}

fn registry_numbered_directories() -> BTreeSet<String> {
    DOC_TYPES
        .iter()
        .filter_map(|spec| match spec.identity {
            Identity::Numbered { dir } | Identity::Named { dir } => Some(dir.to_owned()),
            Identity::Singleton { .. } => None,
        })
        .collect()
}

fn registry_numbered_tokens() -> BTreeSet<String> {
    DOC_TYPES
        .iter()
        .filter_map(|spec| match spec.identity {
            Identity::Numbered { .. } | Identity::Named { .. } => Some(spec.token.to_owned()),
            Identity::Singleton { .. } => None,
        })
        .collect()
}

#[test]
fn hook_scope_regex_agrees_with_the_registrys_numbered_directories() {
    let hook_dirs = scope_regex_directories(&hook_source());
    let registry_dirs = registry_numbered_directories();
    assert_eq!(
        hook_dirs, registry_dirs,
        "block-docs-handwrite.sh's scope regex directories {hook_dirs:?} disagree with \
         doc_type::DOC_TYPES's Identity::Numbered directories {registry_dirs:?} — edit \
         the hook's scope regex to match the registry"
    );
}

#[test]
fn hook_deny_message_agrees_with_the_registrys_numbered_tokens() {
    let hook_tokens = deny_message_tokens(&hook_source());
    let registry_tokens = registry_numbered_tokens();
    assert_eq!(
        hook_tokens, registry_tokens,
        "block-docs-handwrite.sh's guard_write deny message tokens {hook_tokens:?} disagree \
         with doc_type::DOC_TYPES's Identity::Numbered tokens {registry_tokens:?} — edit \
         the hook's deny message to match the registry"
    );
}
