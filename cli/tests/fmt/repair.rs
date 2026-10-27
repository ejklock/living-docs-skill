use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

const ADR_HAND: &str = "---\ntype: ADR\ntitle: Hand\ndescription: d\nstatus: Accepted\n---\n\n# 0003. Other heading\n\nBody text.\n";
const HAND: &str = "adr/0003-hand.md";

fn fmt_with(target: &Path, identity: &[(&str, &str)], flags: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_living-docs"));
    cmd.env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_COUNT", identity.len().to_string());
    for (i, (key, value)) in identity.iter().enumerate() {
        cmd.env(format!("GIT_CONFIG_KEY_{i}"), key)
            .env(format!("GIT_CONFIG_VALUE_{i}"), value);
    }
    cmd.arg("fmt")
        .args(flags)
        .arg(target)
        .arg("--plain")
        .output()
        .expect("failed to run living-docs fmt")
}

fn run(target: &Path) -> Output {
    fmt_with(target, &[], &[])
}

fn git(dir: &Path, date: Option<&str>, args: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir)
        .args(["-c", "user.name=T", "-c", "user.email=t@example.com"])
        .args(args);
    if let Some(date) = date {
        cmd.env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date);
    }
    assert!(cmd.output().unwrap().status.success(), "git {args:?}");
}

fn utc_now() -> String {
    let mut date = Command::new("date");
    let out = date.args(["-u", "+%FT%TZ"]).output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn timestamp_of(contents: &str) -> String {
    let line = contents.lines().find_map(|l| l.strip_prefix("timestamp: "));
    line.expect("a timestamp line").to_owned()
}

pub(super) fn body_of(contents: &str) -> &str {
    let body = contents.splitn(3, "---\n").nth(2).unwrap();
    body.trim_start_matches('\n')
}

fn without_first_heading(body: &str) -> String {
    let at = body.lines().position(|l| l.starts_with('#')).unwrap();
    let lines: Vec<&str> = body.split_inclusive('\n').collect();
    [&lines[..at], &lines[at + 1..]].concat().concat()
}

fn repo_bundle(label: &str) -> Scratch {
    let bundle = temp_bundle(label);
    git(bundle.parent().unwrap(), None, &["init", "-q"]);
    bundle
}

fn assert_stamped_inside_the_run(bundle: &Path) -> Output {
    let before = utc_now();
    let output = run(bundle);
    let (stamp, after) = (timestamp_of(&read(bundle, HAND)), utc_now());
    assert!(
        before <= stamp && stamp <= after,
        "{before} {stamp} {after}"
    );
    output
}

fn commit(root: &Path, date: &str, flags: &str) {
    git(root, Some(date), &["commit", flags, "m"]);
}

#[test]
fn timestamp_comes_from_the_adding_commit_not_a_later_one() {
    let bundle = repo_bundle("ts-commit");
    let root = bundle.parent().unwrap();
    write(&bundle, HAND, ADR_HAND);
    git(root, None, &["add", "."]);
    commit(root, "2021-03-04T05:06:07+02:00", "-qm");
    write(&bundle, HAND, &format!("{ADR_HAND}More.\n"));
    commit(root, "2022-05-06T07:08:09+00:00", "-qam");

    let output = run(&bundle);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(timestamp_of(&read(&bundle, HAND)), "2021-03-04T03:06:07Z");
}

#[test]
fn timestamp_of_a_file_never_committed_falls_inside_the_run_window() {
    let bundle = repo_bundle("ts-uncommitted");
    write(&bundle, HAND, ADR_HAND);

    assert_stamped_inside_the_run(&bundle);
}

#[test]
fn timestamp_outside_a_git_repository_is_the_clock_without_error() {
    let bundle = temp_bundle("ts-no-repo");
    write(&bundle, HAND, ADR_HAND);

    let output = assert_stamped_inside_the_run(&bundle);

    assert!(output.status.success(), "{output:?}");
}

#[test]
fn timestamp_already_present_is_kept_byte_for_byte() {
    let bundle = temp_bundle("ts-present");
    write(
        &bundle,
        HAND,
        &ADR_HAND.replace("---\n\n#", "timestamp: 2019-01-02T03:04:05Z\n---\n\n#"),
    );

    run(&bundle);

    assert_eq!(timestamp_of(&read(&bundle, HAND)), "2019-01-02T03:04:05Z");
}

pub(super) fn fmt_record(identity: &[(&str, &str)], record: &str, rel: &str) -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let bundle = temp_bundle(&format!("record-{}", NEXT.fetch_add(1, Ordering::SeqCst)));
    write(&bundle, rel, record);
    fmt_with(&bundle, identity, &[]);
    read(&bundle, rel)
}

#[test]
fn owner_on_an_adr_is_the_git_user_name() {
    let identity = [
        ("user.name", "Ada Lovelace"),
        ("user.email", "ada@example.com"),
    ];
    let after = fmt_record(&identity, ADR_HAND, HAND);
    assert!(after.contains("\nowner: Ada Lovelace\n"), "{after}");
}

#[test]
fn owner_falls_back_to_the_git_user_email() {
    let after = fmt_record(&[("user.email", "ada@example.com")], ADR_HAND, HAND);
    assert!(after.contains("\nowner: ada@example.com\n"), "{after}");
}

#[test]
fn owner_stays_absent_without_a_git_identity_and_check_reports_it() {
    let bundle = temp_bundle("owner-none");
    write(&bundle, HAND, ADR_HAND);
    fmt_with(&bundle, &[], &[]);
    assert!(!read(&bundle, HAND).contains("owner:"));

    let check = Command::new(env!("CARGO_BIN_EXE_living-docs"))
        .args(["check", "--plain"])
        .arg(&bundle)
        .output()
        .unwrap();

    assert!(stdout_of(&check).contains("OWNER advisory"), "{check:?}");
}

#[test]
fn owner_is_not_seeded_on_a_type_that_does_not_require_one() {
    let issue = "---\ntype: Issue\ntitle: X\ndescription: d\nstatus: open\n---\n\n## 0001. X\n";
    let after = fmt_record(&[("user.name", "Ada Lovelace")], issue, "issues/0001-x.md");
    assert!(!after.contains("owner:"), "{after}");
}

#[test]
fn owner_already_present_is_never_replaced() {
    let record = ADR_HAND.replace("description: d\n", "description: d\nowner: Grace\n");
    let after = fmt_record(&[("user.name", "Ada Lovelace")], &record, HAND);
    assert!(
        after.contains("\nowner: Grace\n") && !after.contains("Ada"),
        "{after}"
    );
}

#[test]
fn heading_text_follows_the_frontmatter_title_and_other_bytes_stay() {
    let after = fmt_record(&[], ADR_HAND, HAND);
    assert!(after.contains("\n# 0003. Hand\n"), "{after}");
    assert_eq!(
        without_first_heading(body_of(&after)),
        without_first_heading(body_of(ADR_HAND))
    );
}

#[test]
fn heading_of_an_issue_keeps_its_level() {
    let issue =
        "---\ntype: Issue\ntitle: Real\ndescription: d\nstatus: open\n---\n\n## 0001. X\n\nBody.\n";
    let after = fmt_record(&[], issue, "issues/0001-real.md");
    assert!(after.contains("\n## 0001. Real\n"), "{after}");
}

#[test]
fn heading_seeds_a_missing_frontmatter_title() {
    let record =
        "---\ntype: ADR\ndescription: d\nstatus: Accepted\n---\n\n# 0004. Seeded\n\nBody.\n";
    let after = fmt_record(&[], record, "adr/0004-seeded.md");
    assert!(after.contains("\ntitle: Seeded\n"), "{after}");
    assert!(after.contains("\n# 0004. Seeded\n"), "{after}");
}

#[test]
fn heading_change_skips_fenced_code_and_later_matching_headings() {
    let body =
        "# 0003. Other heading\n\n```\n# 0003. Other heading\n```\n\n# 0003. Other heading\n";
    let record = ADR_HAND.replace("# 0003. Other heading\n\nBody text.\n", body);
    let after = fmt_record(&[], &record, HAND);
    let expected = body.replacen("# 0003. Other heading", "# 0003. Hand", 1);
    assert_eq!(body_of(&after), expected);
}

#[test]
fn heading_change_passes_over_a_fenced_line_that_precedes_the_heading() {
    let body = "```\n# 0003. Other heading\n```\n\n# 0003. Other heading\n";
    let record = ADR_HAND.replace("# 0003. Other heading\n", body);
    let after = fmt_record(&[], &record, HAND);
    let expected = "```\n# 0003. Other heading\n```\n\n# 0003. Hand\n\nBody text.\n";
    assert_eq!(body_of(&after), expected);
}

#[test]
fn heading_absent_leaves_the_body_unchanged() {
    let record = ADR_HAND.replace("# 0003. Other heading\n\n", "");
    let after = fmt_record(&[], &record, HAND);
    assert_eq!(body_of(&after), "Body text.\n");
}

#[test]
fn check_mode_lists_a_record_needing_repair_and_writes_nothing() {
    let bundle = temp_bundle("check-mode");
    write(&bundle, HAND, ADR_HAND);

    let output = fmt_with(&bundle, &[], &["--check"]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(stdout_of(&output).contains("0003-hand.md"));
    assert_eq!(read(&bundle, HAND), ADR_HAND);
}

#[test]
fn check_mode_is_clean_after_one_fmt_and_a_second_fmt_writes_nothing() {
    let bundle = temp_bundle("idempotent");
    write(&bundle, HAND, ADR_HAND);
    run(&bundle);
    let first = read(&bundle, HAND);

    let second = run(&bundle);
    let check = fmt_with(&bundle, &[], &["--check"]);

    assert!(
        stdout_of(&second).contains("0 record(s) rewritten."),
        "{second:?}"
    );
    assert_eq!(read(&bundle, HAND), first);
    assert_eq!(check.status.code(), Some(0), "{check:?}");
}

#[test]
fn check_mode_with_a_record_path_repairs_that_record_only() {
    let bundle = temp_bundle("one-record");
    let other = ADR_HAND.replace("Hand", "Second");
    write(&bundle, HAND, ADR_HAND);
    write(&bundle, "adr/0004-second.md", &other);

    run(&bundle.join(HAND));

    assert!(read(&bundle, HAND).contains("# 0003. Hand"));
    assert_eq!(read(&bundle, "adr/0004-second.md"), other);
}
