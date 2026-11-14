use super::*;

const ADR: &str = "---\ntype: ADR\ntitle: Hand\ndescription: d\nstatus: Accepted\n---\n\n# 0003. Hand\n\nBody text.\n";
const RECORD: &str = "adr/0003-hand.md";

fn repo_with(label: &str, config: &[(&str, &str)]) -> Scratch {
    let bundle = temp_bundle(label);
    let root = bundle.parent().unwrap();
    git_in(root, &["init", "-q"]);
    for (key, value) in config {
        git_in(root, &["config", key, value]);
    }
    write(&bundle, RECORD, ADR);
    bundle
}

fn git_in(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

fn fmt_against(docs_dir: &Path, target: Option<&Path>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_living-docs"));
    cmd.env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .arg("--docs-dir")
        .arg(docs_dir)
        .args(["manage", "fmt"]);
    if let Some(target) = target {
        cmd.arg(target);
    }
    let output = cmd.arg("--plain").output().unwrap();
    assert!(output.status.success(), "{output:?}");
    output
}

fn owner_line(bundle: &Path) -> Option<String> {
    let record = read(bundle, RECORD);
    let line = record.lines().find_map(|l| l.strip_prefix("owner: "));
    line.map(str::to_owned)
}

fn docs_repo() -> Scratch {
    repo_with("owner-docs", &[("user.name", "Docs Owner")])
}

fn target_repo(label: &str) -> Scratch {
    repo_with(label, &[("user.name", "Target Owner")])
}

#[test]
fn bundle_argument_takes_the_owner_of_its_own_repository() {
    let (docs, target) = (docs_repo(), target_repo("owner-bundle"));

    fmt_against(&docs, Some(&target));

    assert_eq!(owner_line(&target).as_deref(), Some("Target Owner"));
}

#[test]
fn record_argument_takes_the_owner_of_its_own_repository() {
    let (docs, target) = (docs_repo(), target_repo("owner-record"));

    fmt_against(&docs, Some(&target.join(RECORD)));

    assert_eq!(owner_line(&target).as_deref(), Some("Target Owner"));
}

#[test]
fn no_argument_formats_the_docs_dir_with_its_owner() {
    let docs = docs_repo();

    fmt_against(&docs, None);

    assert_eq!(owner_line(&docs).as_deref(), Some("Docs Owner"));
}

#[test]
fn target_repository_without_a_name_falls_back_to_its_email() {
    let docs = docs_repo();
    let target = repo_with("owner-email", &[("user.email", "t@example.com")]);

    fmt_against(&docs, Some(&target));

    assert_eq!(owner_line(&target).as_deref(), Some("t@example.com"));
}

#[test]
fn target_outside_any_repository_is_not_given_the_docs_owner() {
    let docs = docs_repo();
    let target = temp_bundle("owner-outside");
    write(&target, RECORD, ADR);

    fmt_against(&docs, Some(&target));

    assert_eq!(owner_line(&target), None);
}

#[cfg(unix)]
#[test]
fn symlinked_bundle_argument_takes_the_owner_of_the_linked_repository() {
    let (docs, target) = (docs_repo(), target_repo("owner-link"));
    let scratch = Scratch::new("living-docs-fmt-test-owner-link");
    let link = scratch.join("linked");
    std::os::unix::fs::symlink(&*target, &link).unwrap();

    fmt_against(&docs, Some(&link));

    assert_eq!(owner_line(&target).as_deref(), Some("Target Owner"));
}
