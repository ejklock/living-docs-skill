//! A reader that closes stdout early ends the process silently with the exit
//! status the run would have had, instead of a "Broken pipe" panic.

mod common;

use common::living_docs;
use std::io::{pipe, Read};
use std::process::{Command, Stdio};

/// Larger than any pipe buffer the supported platforms use, so the child is
/// still writing when the reader goes away.
const PIPE_BUFFER_BYTES: usize = 64 * 1024;

fn docs_dir() -> String {
    format!("{}/../docs", env!("CARGO_MANIFEST_DIR"))
}

fn command(args: &[&str]) -> Command {
    let mut command = living_docs();
    command.args(["--docs-dir", &docs_dir()]).args(args);
    command
}

fn open_pipe_run(args: &[&str]) -> (Option<i32>, usize) {
    let output = command(args).output().expect("failed to run living-docs");
    (output.status.code(), output.stdout.len())
}

fn exit_and_stderr(output: &std::process::Output) -> (Option<i32>, String) {
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Runs `args` with stdout wired to a pipe whose read end is already gone, so
/// the child's first write fails regardless of how much it writes.
fn run_with_reader_already_closed(args: &[&str]) -> (Option<i32>, String) {
    let (reader, writer) = pipe().expect("failed to create a pipe");
    drop(reader);
    let output = command(args)
        .stdout(Stdio::from(writer))
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run living-docs");
    exit_and_stderr(&output)
}

/// Runs `args`, reads `read_bytes` bytes of stdout, closes the read end and
/// returns the exit code with everything written to stderr.
fn run_with_reader_closing_mid_stream(args: &[&str], read_bytes: usize) -> (Option<i32>, String) {
    let mut child = command(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn living-docs");
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut head = vec![0u8; read_bytes];
    stdout
        .read_exact(&mut head)
        .expect("the child writes first");
    drop(stdout);
    exit_and_stderr(&child.wait_with_output().expect("failed to wait"))
}

fn assert_matches_open_run(args: &[&str], closed: (Option<i32>, String)) -> usize {
    let (open_code, open_len) = open_pipe_run(args);
    let (code, stderr) = closed;
    assert_eq!(stderr, "", "{args:?} wrote to stderr");
    assert_eq!(code, open_code, "{args:?} changed its exit status");
    open_len
}

fn assert_ends_silently_when_reader_is_gone(args: &[&str]) {
    assert_matches_open_run(args, run_with_reader_already_closed(args));
}

fn assert_ends_silently_when_reader_closes_mid_stream(args: &[&str]) {
    let closed = run_with_reader_closing_mid_stream(args, 10);
    let open_len = assert_matches_open_run(args, closed);
    assert!(
        open_len > PIPE_BUFFER_BYTES,
        "{args:?} wrote only {open_len} bytes, too few to outlive the pipe buffer"
    );
}

#[test]
fn read_json_full_ends_silently_when_the_reader_closes_stdout() {
    assert_ends_silently_when_reader_closes_mid_stream(&["read", "--json", "--full"]);
}

#[test]
fn read_plain_full_ends_silently_when_the_reader_closes_stdout() {
    assert_ends_silently_when_reader_closes_mid_stream(&["read", "--plain", "--full"]);
}

#[test]
fn check_json_ends_silently_when_the_reader_closes_stdout() {
    assert_ends_silently_when_reader_is_gone(&["check", "--json", "--advisories"]);
}

#[test]
fn guide_ends_silently_when_the_reader_closes_stdout() {
    assert_ends_silently_when_reader_is_gone(&["guide"]);
}

#[test]
fn guide_list_ends_silently_when_the_reader_closes_stdout() {
    assert_ends_silently_when_reader_is_gone(&["guide", "--list", "--plain"]);
}

#[test]
fn completions_end_silently_when_the_reader_closes_stdout() {
    assert_ends_silently_when_reader_is_gone(&["completions", "zsh"]);
}

#[test]
fn open_stdout_still_carries_the_whole_output() {
    let (code, len) = open_pipe_run(&["read", "--json", "--full"]);
    assert_eq!(code, Some(0));
    assert!(len > PIPE_BUFFER_BYTES);
}
