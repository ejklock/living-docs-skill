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
//!
//! `include_tier3` (ADR 0012, `--check-tier3`) additionally runs the Tier-3
//! PII detectors — the highest-false-positive class — over every file scanned
//! for PII. It stays off by default: the flag is a policy of this command
//! layer, not of `pii::collect_pii_violations`, which is untouched.

use crate::check::file_name_str;
use crate::check::links::{link_destinations, resolve_destination};
use crate::frontmatter;
use crate::pii;
use crate::store::DocStore;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::OnceLock;

pub fn run(store: &dyn DocStore, bundle: &Path, include_tier3: bool) -> ExitCode {
    let all_md = store.list(bundle).unwrap_or_default();
    let bundle_str = bundle.to_string_lossy();
    let mut violations = Vec::new();

    for path in &all_md {
        collect_private_present_violation(store, path, &mut violations);
        collect_dangling_link_violations(store, path, &bundle_str, &mut violations);
        collect_secret_violations(store, path, &mut violations);
        collect_pii_violations(store, path, include_tier3, &mut violations);
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
    StripeSecretKey,
    GitHubToken,
    GitLabToken,
    GoogleApiKey,
    SlackToken,
    Jwt,
    BearerToken,
    HighEntropyAssignment,
}

/// How much of a matched value `mask` is allowed to reveal. Each
/// `SecretClass` maps to exactly one strategy (`mask_strategy`), so adding a
/// class never grows `mask` itself — it only ever switches on these three
/// shapes, keeping it within the complexity budget regardless of how many
/// provider patterns the ruleset (ADR 0011) grows to.
#[derive(Clone, Copy)]
enum MaskStrategy {
    Redact,
    KeepPrefix(usize),
    Email,
}

impl SecretClass {
    fn label(self) -> &'static str {
        match self {
            SecretClass::PemPrivateKey => "PEM private-key block",
            SecretClass::AwsAccessKeyId => "AWS access key id",
            SecretClass::SecretAssignment => "secret/token/password assignment",
            SecretClass::Email => "email address (PII)",
            SecretClass::StripeSecretKey => "Stripe secret key",
            SecretClass::GitHubToken => "GitHub token",
            SecretClass::GitLabToken => "GitLab personal access token",
            SecretClass::GoogleApiKey => "Google API key",
            SecretClass::SlackToken => "Slack token",
            SecretClass::Jwt => "JSON Web Token (JWT)",
            SecretClass::BearerToken => "generic Bearer token",
            SecretClass::HighEntropyAssignment => "high-entropy generic secret assignment",
        }
    }

    fn mask_strategy(self) -> MaskStrategy {
        match self {
            SecretClass::Email => MaskStrategy::Email,
            SecretClass::AwsAccessKeyId => MaskStrategy::KeepPrefix(4),
            SecretClass::StripeSecretKey => MaskStrategy::KeepPrefix(8),
            SecretClass::GitHubToken => MaskStrategy::KeepPrefix(4),
            SecretClass::GitLabToken => MaskStrategy::KeepPrefix(6),
            SecretClass::GoogleApiKey => MaskStrategy::KeepPrefix(4),
            SecretClass::SlackToken => MaskStrategy::KeepPrefix(5),
            SecretClass::PemPrivateKey
            | SecretClass::SecretAssignment
            | SecretClass::Jwt
            | SecretClass::BearerToken
            | SecretClass::HighEntropyAssignment => MaskStrategy::Redact,
        }
    }
}

/// The secret/PII pattern set (ADR 0010, deepened by ADR 0011): a curated,
/// gitleaks-style set of high-signal provider-token regexes compiled once.
/// This is heuristic and advisory, not exhaustive — it targets the shapes
/// most likely to leak (PEM key material, AWS access key ids, quoted
/// secret/token/password/api_key assignments, email addresses, and named
/// provider token formats for Stripe/GitHub/GitLab/Google/Slack/JWT/Bearer)
/// and is versioned alongside those ADRs rather than grown ad hoc.
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
            (
                SecretClass::StripeSecretKey,
                Regex::new(r"(?:sk|rk)_live_[0-9a-zA-Z]{20,}")
                    .expect("valid stripe secret key regex"),
            ),
            (
                SecretClass::GitHubToken,
                Regex::new(r"gh[pousr]_[0-9A-Za-z]{36}").expect("valid github token regex"),
            ),
            (
                SecretClass::GitLabToken,
                Regex::new(r"glpat-[0-9A-Za-z_-]{20}").expect("valid gitlab token regex"),
            ),
            (
                SecretClass::GoogleApiKey,
                Regex::new(r"AIza[0-9A-Za-z_-]{35}").expect("valid google api key regex"),
            ),
            (
                SecretClass::SlackToken,
                Regex::new(r"xox[baprs]-[0-9A-Za-z-]{10,}").expect("valid slack token regex"),
            ),
            (
                SecretClass::Jwt,
                Regex::new(r"eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}")
                    .expect("valid jwt regex"),
            ),
            (
                SecretClass::BearerToken,
                Regex::new(r"(?i)bearer\s+[A-Za-z0-9._~+/=-]{20,}")
                    .expect("valid bearer token regex"),
            ),
        ]
    })
}

/// Masks a matched secret/PII value so the gate never echoes it verbatim —
/// a leak gate that prints the secret it caught would itself be a leak
/// vector. Delegates to the class's `MaskStrategy` so this stays a fixed
/// three-arm match no matter how many provider classes the ruleset grows to.
fn mask(class: SecretClass, matched: &str) -> String {
    match class.mask_strategy() {
        MaskStrategy::Email => mask_email(matched),
        MaskStrategy::KeepPrefix(keep) => mask_keeping_prefix(matched, keep),
        MaskStrategy::Redact => "[redacted]".to_string(),
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
    collect_secret_pattern_violations(path, &contents, violations);
    collect_high_entropy_assignment_violations(path, &contents, violations);
}

fn collect_secret_pattern_violations(
    path: &Path,
    contents: &str,
    violations: &mut Vec<(PathBuf, String)>,
) {
    for (class, pattern) in secret_patterns() {
        let Some(found) = pattern.find(contents) else {
            continue;
        };
        let masked = mask(*class, found.as_str());
        violations.push((
            path.to_path_buf(),
            format!("{} detected (masked: {masked})", class.label()),
        ));
    }
}

/// Bits/char below which a quoted assignment value is treated as ordinary
/// text rather than a generic secret (ADR 0011). English prose and
/// structural strings (hex ids, short config values) cluster below this
/// line; base64-ish random token material sits comfortably above it. Kept
/// as a named threshold — like `secret_patterns()` — so it is versioned
/// rather than a magic number buried in the scan.
const HIGH_ENTROPY_THRESHOLD: f64 = 4.0;

/// Matches a quoted assignment (`identifier = "value"` / `identifier:
/// "value"`) whose value is at least 24 characters, the shape a generic
/// high-entropy secret takes. Scoping the entropy scan to this shape — never
/// free prose — is what keeps it quiet on a git SHA or an ordinary sentence
/// mentioned in a doc (ADR 0011).
fn assignment_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r#"(?i)([A-Za-z0-9_]+)\s*[:=]\s*["']([^"']{24,})["']"#)
            .expect("valid assignment regex")
    })
}

/// Shannon entropy of `value` in bits per character, over the character
/// frequency distribution. A generic random secret (base64/hex token
/// material) sits well above ordinary prose or structured ids, which is
/// what lets the entropy scan flag a high-entropy value assigned to a
/// non-secret-named key (ADR 0011).
fn shannon_entropy(value: &str) -> f64 {
    let len = value.chars().count();
    if len == 0 {
        return 0.0;
    }
    let mut counts = std::collections::HashMap::new();
    for c in value.chars() {
        *counts.entry(c).or_insert(0u32) += 1;
    }
    let len = len as f64;
    counts
        .values()
        .map(|&count| {
            let probability = f64::from(count) / len;
            -probability * probability.log2()
        })
        .sum()
}

fn collect_high_entropy_assignment_violations(
    path: &Path,
    contents: &str,
    violations: &mut Vec<(PathBuf, String)>,
) {
    for captures in assignment_pattern().captures_iter(contents) {
        let value = &captures[2];
        if shannon_entropy(value) < HIGH_ENTROPY_THRESHOLD {
            continue;
        }
        let masked = mask(SecretClass::HighEntropyAssignment, value);
        violations.push((
            path.to_path_buf(),
            format!(
                "{} detected (masked: {masked})",
                SecretClass::HighEntropyAssignment.label()
            ),
        ));
    }
}

/// The worldwide PII scan (ADR 0012): reads a doc's contents like the other
/// leak classes, including the reserved `index.md`/`log.md` listing files —
/// content is content regardless of which command normally writes it.
/// `include_tier3` additionally runs the opt-in, highest-false-positive
/// Tier-3 detectors over the same contents.
fn collect_pii_violations(
    store: &dyn DocStore,
    path: &Path,
    include_tier3: bool,
    violations: &mut Vec<(PathBuf, String)>,
) {
    let Ok(contents) = store.read(path) else {
        return;
    };
    pii::collect_pii_violations(path, &contents, violations);
    if include_tier3 {
        pii::collect_tier3_violations(path, &contents, violations);
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

        let code = run(&store, &bundle.root, false);

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

        let code = run(&store, &bundle.root, false);

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

        let code = run(&store, &bundle.root, false);

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

        let code = run(&store, &bundle.root, false);

        assert!(exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_passes_an_empty_bundle() {
        let bundle = TempBundle::new("empty-bundle");
        let store = MapStore {
            files: BTreeMap::new(),
        };

        let code = run(&store, &bundle.root, false);

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

        let code = run(&store, &bundle.root, false);

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

        let code = run(&store, &bundle.root, false);

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

        let code = run(&store, &bundle.root, false);

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

        let code = run(&store, &bundle.root, false);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_masks_the_matched_secret_value_in_the_reported_message() {
        let bundle = TempBundle::new("secret-masking");
        let doc_path = bundle.root.join("adr").join("0001-key.md");
        let raw_secret = "abcdefghijklmnop1234";
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

        let code = run(&store, &bundle.root, false);

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

        let code = run(&store, &bundle.root, false);
        assert!(!exit_code_is_success(code));
    }

    fn bundle_with_one_doc(label: &str, body: &str) -> (TempBundle, PathBuf, MapStore) {
        let bundle = TempBundle::new(label);
        let doc_path = bundle.root.join("adr").join("0001-doc.md");
        let mut files = BTreeMap::new();
        files.insert(
            doc_path.clone(),
            format!("---\ntype: ADR\nvisibility: public\n---\n{body}\n"),
        );
        let store = MapStore { files };
        (bundle, doc_path, store)
    }

    #[test]
    fn leak_gate_fails_when_a_doc_contains_a_stripe_secret_key() {
        let (bundle, _doc_path, store) =
            bundle_with_one_doc("stripe-secret-key", "sk_\x6cive_abcdefghijklmnopqrstuvwx");

        let code = run(&store, &bundle.root, false);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_doc_contains_a_github_token() {
        let (bundle, doc_path, store) =
            bundle_with_one_doc("github-token", "ghp_abcdefghijklmnopqrstuvwxyz0123456789");
        let mut violations = Vec::new();

        collect_secret_violations(&store, &doc_path, &mut violations);

        assert!(violations
            .iter()
            .any(|(path, m)| path == &doc_path && m.contains("GitHub token")));

        let code = run(&store, &bundle.root, false);
        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_doc_contains_a_gitlab_token() {
        let (bundle, _doc_path, store) =
            bundle_with_one_doc("gitlab-token", "glpat-abcdefghijklmnopqrst");

        let code = run(&store, &bundle.root, false);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_doc_contains_a_google_api_key() {
        let (bundle, _doc_path, store) =
            bundle_with_one_doc("google-api-key", "AIza0123456789abcdefghijklmnopqrstuvwxy");

        let code = run(&store, &bundle.root, false);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_doc_contains_a_slack_token() {
        let (bundle, _doc_path, store) =
            bundle_with_one_doc("slack-token", "xoxb-1234567890abcdef");

        let code = run(&store, &bundle.root, false);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_doc_contains_a_jwt() {
        let (bundle, _doc_path, store) = bundle_with_one_doc(
            "jwt",
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
        );

        let code = run(&store, &bundle.root, false);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_doc_contains_a_generic_bearer_token() {
        let (bundle, _doc_path, store) = bundle_with_one_doc(
            "bearer-token",
            "Authorization: Bearer abcdefghijklmnopqrstuvwxyz1234567890",
        );

        let code = run(&store, &bundle.root, false);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_high_entropy_value_is_assigned_to_a_non_secret_named_key() {
        let (bundle, doc_path, store) = bundle_with_one_doc(
            "high-entropy-assignment",
            "session_marker = \"N3xQ9pLvR8tYbZ6dS1jWaC4mK0fGhEuX\"",
        );
        let mut violations = Vec::new();

        collect_secret_violations(&store, &doc_path, &mut violations);

        assert!(violations
            .iter()
            .any(|(_, m)| m.contains("high-entropy generic secret assignment")));

        let code = run(&store, &bundle.root, false);
        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_secret_scan_does_not_false_fail_on_a_git_sha_mentioned_in_prose() {
        let (bundle, _doc_path, store) = bundle_with_one_doc(
            "git-sha-in-prose",
            "See commit a1b2c3d4e5f6789012345678901234567890abcd for the fix.",
        );

        let code = run(&store, &bundle.root, false);

        assert!(exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_secret_scan_does_not_false_fail_on_a_low_entropy_quoted_assignment() {
        let (bundle, _doc_path, store) = bundle_with_one_doc(
            "low-entropy-assignment",
            "note = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
        );

        let code = run(&store, &bundle.root, false);

        assert!(exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_masks_a_provider_token_keeping_only_a_short_public_prefix() {
        let raw_token = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let (bundle, doc_path, store) = bundle_with_one_doc("github-token-masking", raw_token);
        let mut violations = Vec::new();

        collect_secret_violations(&store, &doc_path, &mut violations);

        assert!(violations.iter().any(|(_, m)| !m.contains(raw_token)));

        let code = run(&store, &bundle.root, false);
        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_doc_contains_a_checksum_valid_cpf() {
        let (bundle, doc_path, store) =
            bundle_with_one_doc("cpf-pii", "Cliente CPF: 111.444.777-35");
        let mut violations = Vec::new();

        collect_pii_violations(&store, &doc_path, false, &mut violations);

        assert!(violations
            .iter()
            .any(|(_, m)| m.contains("Brazilian CPF") && !m.contains("111.444.777-35")));

        let code = run(&store, &bundle.root, false);
        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_stays_quiet_on_a_cpf_shaped_number_with_a_broken_check_digit() {
        let (bundle, _doc_path, store) =
            bundle_with_one_doc("cpf-pii-broken", "Reference number 111.444.777-00");

        let code = run(&store, &bundle.root, false);

        assert!(exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_composes_the_pii_scan_with_the_private_dangling_link_and_secret_checks() {
        let bundle = TempBundle::new("compose-with-pii");
        let private_path = bundle.root.join("adr").join("0001-private.md");
        let link_path = bundle.root.join("adr").join("0002-link.md");
        let secret_path = bundle.root.join("adr").join("0003-secret.md");
        let pii_path = bundle.root.join("adr").join("0004-pii.md");

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
        files.insert(
            pii_path.clone(),
            "---\ntype: ADR\nvisibility: public\n---\nCPF: 111.444.777-35\n".to_string(),
        );
        let store = MapStore { files };
        let bundle_str = bundle.root.to_string_lossy();
        let mut violations = Vec::new();

        for path in [&private_path, &link_path, &secret_path, &pii_path] {
            collect_private_present_violation(&store, path, &mut violations);
            collect_dangling_link_violations(&store, path, &bundle_str, &mut violations);
            collect_secret_violations(&store, path, &mut violations);
            collect_pii_violations(&store, path, false, &mut violations);
        }

        assert!(violations.iter().any(|(_, m)| m.contains("private")));
        assert!(violations.iter().any(|(_, m)| m.contains("withheld")));
        assert!(violations
            .iter()
            .any(|(_, m)| m.contains("AWS access key id")));
        assert!(violations.iter().any(|(_, m)| m.contains("Brazilian CPF")));

        let code = run(&store, &bundle.root, false);
        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_masks_a_high_entropy_value_fully() {
        let raw_value = "N3xQ9pLvR8tYbZ6dS1jWaC4mK0fGhEuX";
        let (bundle, doc_path, store) = bundle_with_one_doc(
            "high-entropy-masking",
            &format!("session_marker = \"{raw_value}\""),
        );
        let mut violations = Vec::new();

        collect_secret_violations(&store, &doc_path, &mut violations);

        assert!(violations.iter().any(|(_, m)| m
            .contains("high-entropy generic secret assignment")
            && !m.contains(raw_value)));

        let code = run(&store, &bundle.root, false);
        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_flags_an_ipv4_address_when_tier3_is_included() {
        let (bundle, _doc_path, store) =
            bundle_with_one_doc("ipv4-tier3-on", "Server at 192.168.1.1 on file.");

        let code = run(&store, &bundle.root, true);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_masks_the_reported_ipv4_address() {
        let (_bundle, doc_path, store) =
            bundle_with_one_doc("ipv4-tier3-masking", "Server at 192.168.1.1 on file.");
        let mut violations = Vec::new();

        collect_pii_violations(&store, &doc_path, true, &mut violations);

        assert!(violations
            .iter()
            .any(|(_, m)| m.contains("IPv4 address") && !m.contains("192.168.1.1")));
    }

    #[test]
    fn leak_gate_stays_quiet_on_an_ipv4_address_when_tier3_is_excluded_by_default() {
        let (bundle, doc_path, store) =
            bundle_with_one_doc("ipv4-tier3-off", "Server at 192.168.1.1 on file.");
        let mut violations = Vec::new();

        collect_pii_violations(&store, &doc_path, false, &mut violations);

        assert!(violations.is_empty());

        let code = run(&store, &bundle.root, false);
        assert!(exit_code_is_success(code));
    }
}
