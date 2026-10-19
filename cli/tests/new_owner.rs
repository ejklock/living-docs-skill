use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("living-docs-owner-test-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs the binary from `cwd` with the git identity pinned by environment:
/// `identity` is a list of git config `(key, value)` pairs, empty for none.
fn run(cwd: &Path, identity: &[(&str, &str)], args: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_living-docs"));
    cmd.current_dir(cwd)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_COUNT", identity.len().to_string());
    for (i, (key, value)) in identity.iter().enumerate() {
        cmd.env(format!("GIT_CONFIG_KEY_{i}"), key)
            .env(format!("GIT_CONFIG_VALUE_{i}"), value);
    }
    cmd.args(["--docs-dir", "docs"])
        .args(args)
        .arg("--plain")
        .output()
        .expect("failed to run living-docs")
}

fn created_record(cwd: &Path, output: &Output) -> String {
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    fs::read_to_string(cwd.join(stdout.lines().next().unwrap())).unwrap()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn adr_without_owner_takes_the_git_user_name_after_description() {
    let cwd = temp_dir("name");
    let out = run(&cwd, &[("user.name", "Ada Lovelace")], &["new", "adr", "T"]);
    let record = created_record(&cwd, &out);
    let lines: Vec<&str> = record.lines().collect();
    let owner = lines
        .iter()
        .position(|l| *l == "owner: Ada Lovelace")
        .unwrap();
    assert!(lines[owner - 1].starts_with("description:"), "{record}");
    assert!(stderr_of(&out).contains("owner seeded from git config"));
    let check = run(&cwd, &[], &["check", "--require-owner", "docs"]);
    let report = String::from_utf8_lossy(&check.stdout);
    assert!(!report.contains("has no owner"), "{report}");
}

#[test]
fn adr_without_owner_falls_back_to_the_git_email() {
    let cwd = temp_dir("email");
    let out = run(
        &cwd,
        &[("user.email", "ada@example.com")],
        &["new", "adr", "T"],
    );
    assert!(created_record(&cwd, &out).contains("owner: ada@example.com"));
}

#[test]
fn explicit_owner_wins_and_stays_silent() {
    let cwd = temp_dir("explicit");
    let out = run(
        &cwd,
        &[("user.name", "Ada")],
        &["new", "adr", "T", "--owner", "Bob"],
    );
    let record = created_record(&cwd, &out);
    assert!(
        record.contains("owner: Bob") && !record.contains("Ada"),
        "{record}"
    );
    assert!(!stderr_of(&out).contains("owner seeded from git config"));
}

#[test]
fn adr_without_any_git_identity_has_no_owner_line() {
    let cwd = temp_dir("none");
    let out = run(&cwd, &[], &["new", "adr", "T"]);
    assert!(!created_record(&cwd, &out).contains("owner:"));
    assert!(!stderr_of(&out).contains("owner seeded from git config"));
}

#[test]
fn quiet_silences_the_seeded_owner_line_but_still_seeds() {
    let cwd = temp_dir("quiet");
    let out = run(
        &cwd,
        &[("user.name", "Ada")],
        &["--quiet", "new", "adr", "T"],
    );
    assert!(created_record(&cwd, &out).contains("owner: Ada"));
    assert!(!stderr_of(&out).contains("owner seeded from git config"));
}

#[test]
fn issue_never_gets_an_owner_from_git() {
    let cwd = temp_dir("issue");
    let out = run(&cwd, &[("user.name", "Ada")], &["new", "issue", "T"]);
    assert!(!created_record(&cwd, &out).contains("owner:"));
}
