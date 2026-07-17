//! `living-docs leak-gate <bundle>`: fails closed on ADR 0010's three leak
//! classes over an already-exported bundle directory — a private (or
//! visibility-absent) doc that leaked into the bundle, a link from a
//! published doc to a target withheld from the bundle, and a doc carrying a
//! high-signal secret or an email address (PII).
//!
//! The dangling-link scan reuses `check::links`'s destination extraction and
//! resolution rather than re-implementing it, so the two invariants ("does
//! this link exist" for `check`, "was this link's target withheld" here)
//! never drift apart. This command keeps its own small violation collector
//! instead of `check::Reporter` — the two commands report unrelated
//! invariants and have no reason to share output shape.

use crate::check::file_name_str;
use crate::check::links::{link_destinations, resolve_destination};
use crate::frontmatter;
use crate::store::DocStore;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::OnceLock;

pub fn run(store: &dyn DocStore, bundle: &Path) -> ExitCode {
    let all_md = store.list(bundle).unwrap_or_default();
    let bundle_str = bundle.to_string_lossy();
    let mut violations = Vec::new();

    for path in &all_md {
        collect_private_present_violation(store, path, &mut violations);
        collect_dangling_link_violations(store, path, &bundle_str, &mut violations);
        collect_secret_violations(store, path, &mut violations);
    }

    report(violations)
}

const DEFAULT_VISIBILITY: &str = "private";
const BUNDLE_VISIBLE: [&str; 2] = ["public", "showcase"];

fn effective_visibility(contents: &str) -> String {
    frontmatter::read_scalar_from_str(contents, "visibility")
        .unwrap_or_else(|| DEFAULT_VISIBILITY.to_string())
}

fn is_bundle_visible(visibility: &str) -> bool {
    BUNDLE_VISIBLE.contains(&visibility)
}

/// `index.md` and `log.md` carry no frontmatter by design (OKF, ADR 0007) —
/// they are directory listings, not records — so they are exempt from the
/// private-doc-present check rather than defaulting to "leaked private".
fn is_reserved_listing_file(path: &Path) -> bool {
    let name = file_name_str(path);
    name == "index.md" || name == "log.md"
}

fn collect_private_present_violation(
    store: &dyn DocStore,
    path: &Path,
    violations: &mut Vec<(PathBuf, String)>,
) {
    if is_reserved_listing_file(path) {
        return;
    }
    let Ok(contents) = store.read(path) else {
        return;
    };
    let visibility = effective_visibility(&contents);
    if !is_bundle_visible(&visibility) {
        violations.push((
            path.to_path_buf(),
            format!("private doc present in bundle (visibility: {visibility})"),
        ));
    }
}

fn collect_dangling_link_violations(
    store: &dyn DocStore,
    path: &Path,
    bundle: &str,
    violations: &mut Vec<(PathBuf, String)>,
) {
    let Ok(contents) = store.read(path) else {
        return;
    };
    let file_str = path.to_string_lossy();
    for dest in link_destinations(&contents) {
        let Some(target) = resolve_destination(&file_str, &dest, bundle) else {
            continue;
        };
        if !Path::new(&target).exists() {
            violations.push((
                path.to_path_buf(),
                format!("link to withheld doc -> {target}"),
            ));
        }
    }
}

/// A leak class the secret/PII scan can flag. Kept as a closed enum (rather
/// than a bare `&str` label) so `mask` can switch on how much of a match is
/// safe to surface without re-parsing the class from its own report text.
#[derive(Clone, Copy)]
enum SecretClass {
    PemPrivateKey,
    AwsAccessKeyId,
    SecretAssignment,
    Email,
}

impl SecretClass {
    fn label(self) -> &'static str {
        match self {
            SecretClass::PemPrivateKey => "PEM private-key block",
            SecretClass::AwsAccessKeyId => "AWS access key id",
            SecretClass::SecretAssignment => "secret/token/password assignment",
            SecretClass::Email => "email address (PII)",
        }
    }
}

/// The secret/PII pattern set (ADR 0010): a small, high-signal set of
/// regexes compiled once. This is heuristic and advisory, not exhaustive —
/// it targets the shapes most likely to leak (PEM key material, AWS access
/// key ids, quoted secret/token/password/api_key assignments, and email
/// addresses) and is versioned alongside ADR 0010 rather than grown ad hoc.
fn secret_patterns() -> &'static [(SecretClass, Regex)] {
    static PATTERNS: OnceLock<Vec<(SecretClass, Regex)>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            (
                SecretClass::PemPrivateKey,
                Regex::new(r"-----BEGIN [A-Z ]*PRIVATE KEY-----").expect("valid pem regex"),
            ),
            (
                SecretClass::AwsAccessKeyId,
                Regex::new(r"AKIA[0-9A-Z]{16}").expect("valid aws access key regex"),
            ),
            (
                SecretClass::SecretAssignment,
                Regex::new(
                    r#"(?i)(secret|token|password|api[_-]?key)\s*[:=]\s*["'][^"']{16,}["']"#,
                )
                .expect("valid secret-assignment regex"),
            ),
            (
                SecretClass::Email,
                Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}")
                    .expect("valid email regex"),
            ),
        ]
    })
}

/// Masks a matched secret/PII value so the gate never echoes it verbatim —
/// a leak gate that prints the secret it caught would itself be a leak
/// vector. Key material and secret assignments are fully redacted; an AWS
/// key id keeps only its public `AKIA` prefix; an email keeps its first
/// character and domain so the report stays locatable without exposing the
/// full address.
fn mask(class: SecretClass, matched: &str) -> String {
    match class {
        SecretClass::Email => mask_email(matched),
        SecretClass::AwsAccessKeyId => mask_keeping_prefix(matched, 4),
        SecretClass::PemPrivateKey | SecretClass::SecretAssignment => "[redacted]".to_string(),
    }
}

fn mask_keeping_prefix(value: &str, keep: usize) -> String {
    let mut chars = value.chars();
    let prefix: String = chars.by_ref().take(keep).collect();
    let masked_len = chars.count();
    format!("{prefix}{}", "*".repeat(masked_len))
}

fn mask_email(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return "[redacted]".to_string();
    };
    let first = local.chars().next().unwrap_or('*');
    format!("{first}***@{domain}")
}

fn collect_secret_violations(
    store: &dyn DocStore,
    path: &Path,
    violations: &mut Vec<(PathBuf, String)>,
) {
    let Ok(contents) = store.read(path) else {
        return;
    };
    for (class, pattern) in secret_patterns() {
        let Some(found) = pattern.find(&contents) else {
            continue;
        };
        let masked = mask(*class, found.as_str());
        violations.push((
            path.to_path_buf(),
            format!("{} detected (masked: {masked})", class.label()),
        ));
    }
}

fn report(violations: Vec<(PathBuf, String)>) -> ExitCode {
    for (file, message) in &violations {
        println!("  {:<44} {message}", file.display().to_string());
    }
    println!();
    if violations.is_empty() {
        println!("OK — no leaks detected.");
        ExitCode::SUCCESS
    } else {
        println!("FAIL — {} leak(s) detected.", violations.len());
        ExitCode::from(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::io;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn exit_code_is_success(code: ExitCode) -> bool {
        format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
    }

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

    struct TempBundle {
        root: PathBuf,
    }

    impl TempBundle {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before unix epoch")
                .as_nanos();
            let root =
                std::env::temp_dir().join(format!("living-docs-core-leak-gate-{label}-{nanos}"));
            fs::create_dir_all(root.join("adr")).expect("create temp bundle adr dir");
            Self { root }
        }
    }

    impl Drop for TempBundle {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn leak_gate_fails_when_a_private_doc_is_present_in_the_bundle() {
        let bundle = TempBundle::new("private-present");
        let mut files = BTreeMap::new();
        files.insert(
            bundle.root.join("adr").join("0001-secret.md"),
            "---\ntype: ADR\nvisibility: private\n---\n# Secret\n".to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_names_the_offending_file_and_its_visibility() {
        let bundle = TempBundle::new("private-message");
        let doc_path = bundle.root.join("adr").join("0001-secret.md");
        let mut files = BTreeMap::new();
        files.insert(
            doc_path.clone(),
            "---\ntype: ADR\n---\n# Absent Visibility\n".to_string(),
        );
        let store = MapStore { files };
        let mut violations = Vec::new();

        collect_private_present_violation(&store, &doc_path, &mut violations);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].0, doc_path);
        assert!(violations[0].1.contains("private"));
    }

    #[test]
    fn leak_gate_exempts_reserved_index_and_log_files_from_the_private_doc_check() {
        let bundle = TempBundle::new("reserved-exempt");
        let mut files = BTreeMap::new();
        files.insert(bundle.root.join("index.md"), "# Index\n".to_string());
        files.insert(bundle.root.join("log.md"), "# Log\n".to_string());
        files.insert(
            bundle.root.join("adr").join("0001-doc.md"),
            "---\ntype: ADR\nvisibility: public\n---\n# Doc\n".to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert!(exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_link_targets_a_doc_withheld_from_the_bundle() {
        let bundle = TempBundle::new("dangling-link");
        let mut files = BTreeMap::new();
        files.insert(
            bundle.root.join("adr").join("0001-doc.md"),
            "---\ntype: ADR\nvisibility: public\n---\n# Doc\n\n[missing](./0002-missing.md)\n"
                .to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_dangling_link_message_locates_the_link_and_its_target() {
        let bundle = TempBundle::new("dangling-link-message");
        let doc_path = bundle.root.join("adr").join("0001-doc.md");
        let mut files = BTreeMap::new();
        files.insert(
            doc_path.clone(),
            "[missing](./0002-missing.md)\n".to_string(),
        );
        let store = MapStore { files };
        let mut violations = Vec::new();

        collect_dangling_link_violations(
            &store,
            &doc_path,
            &bundle.root.to_string_lossy(),
            &mut violations,
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].0, doc_path);
        assert!(violations[0].1.contains("0002-missing.md"));
    }

    #[test]
    fn leak_gate_passes_a_clean_bundle_where_every_doc_is_public_and_every_link_resolves() {
        let bundle = TempBundle::new("clean-bundle");
        let target_path = bundle.root.join("adr").join("0002-target.md");
        fs::write(
            &target_path,
            "---\ntype: ADR\nvisibility: public\n---\n# Target\n",
        )
        .expect("write real target file backing the resolved link");

        let mut files = BTreeMap::new();
        files.insert(
            bundle.root.join("adr").join("0001-doc.md"),
            "---\ntype: ADR\nvisibility: public\n---\n# Doc\n\n[target](./0002-target.md)\n"
                .to_string(),
        );
        files.insert(
            target_path.clone(),
            "---\ntype: ADR\nvisibility: public\n---\n# Target\n".to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert!(exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_passes_an_empty_bundle() {
        let bundle = TempBundle::new("empty-bundle");
        let store = MapStore {
            files: BTreeMap::new(),
        };

        let code = run(&store, &bundle.root);

        assert!(exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_doc_contains_a_pem_private_key_block() {
        let bundle = TempBundle::new("pem-private-key");
        let mut files = BTreeMap::new();
        files.insert(
            bundle.root.join("adr").join("0001-key.md"),
            "---\ntype: ADR\nvisibility: public\n---\n-----BEGIN RSA PRIVATE KEY-----\nMIIBOgIBAAJBAK...\n-----END RSA PRIVATE KEY-----\n"
                .to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_doc_contains_an_aws_access_key_id() {
        let bundle = TempBundle::new("aws-access-key");
        let mut files = BTreeMap::new();
        files.insert(
            bundle.root.join("adr").join("0001-key.md"),
            "---\ntype: ADR\nvisibility: public\n---\nAKIAABCDEFGHIJKLMNOP\n".to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_doc_contains_a_secret_assignment() {
        let bundle = TempBundle::new("secret-assignment");
        let mut files = BTreeMap::new();
        files.insert(
            bundle.root.join("adr").join("0001-key.md"),
            "---\ntype: ADR\nvisibility: public\n---\napi_key = \"sk_\x6cive_abcdefghijklmnopqrstuvwxyz\"\n"
                .to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_doc_contains_an_email_address() {
        let bundle = TempBundle::new("email-pii");
        let mut files = BTreeMap::new();
        files.insert(
            bundle.root.join("adr").join("0001-contact.md"),
            "---\ntype: ADR\nvisibility: public\n---\nContact jane.doe@example.com for details.\n"
                .to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_masks_the_matched_secret_value_in_the_reported_message() {
        let bundle = TempBundle::new("secret-masking");
        let doc_path = bundle.root.join("adr").join("0001-key.md");
        let raw_secret = "sk_\x6cive_abcdefghijklmnopqrstuvwxyz";
        let mut files = BTreeMap::new();
        files.insert(
            doc_path.clone(),
            format!("---\ntype: ADR\nvisibility: public\n---\napi_key = \"{raw_secret}\"\n"),
        );
        let store = MapStore { files };
        let mut violations = Vec::new();

        collect_secret_violations(&store, &doc_path, &mut violations);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].0, doc_path);
        assert!(!violations[0].1.contains(raw_secret));
    }

    #[test]
    fn leak_gate_masks_the_local_part_of_a_reported_email_address() {
        let bundle = TempBundle::new("email-masking");
        let doc_path = bundle.root.join("adr").join("0001-contact.md");
        let mut files = BTreeMap::new();
        files.insert(doc_path.clone(), "jane.doe@example.com\n".to_string());
        let store = MapStore { files };
        let mut violations = Vec::new();

        collect_secret_violations(&store, &doc_path, &mut violations);

        assert_eq!(violations.len(), 1);
        assert!(!violations[0].1.contains("jane.doe@example.com"));
        assert!(violations[0].1.contains("example.com"));
    }

    #[test]
    fn leak_gate_secret_scan_does_not_false_fail_on_ordinary_prose_mentioning_password() {
        let bundle = TempBundle::new("secret-scan-no-false-positive");
        let mut files = BTreeMap::new();
        files.insert(
            bundle.root.join("adr").join("0001-doc.md"),
            "---\ntype: ADR\nvisibility: public\n---\n# Doc\n\nThe password field must not be left empty, and reviewers should reach the team through the support channel if something looks wrong.\n"
                .to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert!(exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_composes_private_doc_dangling_link_and_secret_leak_classes() {
        let bundle = TempBundle::new("compose-classes");
        let private_path = bundle.root.join("adr").join("0001-private.md");
        let link_path = bundle.root.join("adr").join("0002-link.md");
        let secret_path = bundle.root.join("adr").join("0003-secret.md");

        let mut files = BTreeMap::new();
        files.insert(
            private_path.clone(),
            "---\ntype: ADR\nvisibility: private\n---\n# Private\n".to_string(),
        );
        files.insert(
            link_path.clone(),
            "---\ntype: ADR\nvisibility: public\n---\n# Link\n\n[missing](./0099-missing.md)\n"
                .to_string(),
        );
        files.insert(
            secret_path.clone(),
            "---\ntype: ADR\nvisibility: public\n---\nAKIAABCDEFGHIJKLMNOP\n".to_string(),
        );
        let store = MapStore { files };
        let bundle_str = bundle.root.to_string_lossy();
        let mut violations = Vec::new();

        for path in [&private_path, &link_path, &secret_path] {
            collect_private_present_violation(&store, path, &mut violations);
            collect_dangling_link_violations(&store, path, &bundle_str, &mut violations);
            collect_secret_violations(&store, path, &mut violations);
        }

        assert!(violations.iter().any(|(_, m)| m.contains("private")));
        assert!(violations.iter().any(|(_, m)| m.contains("withheld")));
        assert!(violations
            .iter()
            .any(|(_, m)| m.contains("AWS access key id")));

        let code = run(&store, &bundle.root);
        assert!(!exit_code_is_success(code));
    }
}
