---
type: Issue
title: data verbs panic when the reader closes stdout
description: A closed stdout pipe makes print! panic; every stdout write goes through one writer that ends silently with exit 0 on a broken pipe
status: open
timestamp: 2026-10-26T22:52:42Z
---

## 0099. data verbs panic when the reader closes stdout

`living-docs read --json | head -c 10` panics with `failed printing to stdout: Broken pipe (os error 32)` and a backtrace hint on stderr. Rust's `print!` panics when the reader closes the pipe. The v0.24.0 binary already does this, so it is not a regression; any verb that prints through `print!` behaves the same way.

### Scope

- Every stdout write in `cli/src`, routed through one writer in `cli/src/output.rs`.
- stderr writes stay as they are: a closed stderr is not this defect.

### Decision

- A closed stdout ends the process silently with exit 0, the way ripgrep handles it. The reader chose to stop reading, so nothing failed.
- Not taken: restoring the default `SIGPIPE` disposition at startup. It needs a direct `libc` dependency and an `unsafe` block, and it exits by signal (status 141), which falls outside ADR 0060's exit-code table.

### Acceptance

- `living-docs read --json | head -c 10` prints 10 bytes and nothing on stderr, and `living-docs` exits 0 (`PIPESTATUS`).
- The same holds for `check --json`, `guide adr` and `read --plain` with a closed pipe.
- Output with an open pipe is byte-identical to before for every verb.
- A stdout error other than a broken pipe still surfaces. It is never swallowed.

### Outcome

Every stdout write in the CLI goes through one writer in `cli/src/output.rs`, which writes and then flushes. One pure decision, `exit_for`, maps the result:

- success continues;
- a broken pipe exits 0 silently, the way ripgrep does;
- any other error prints `failed writing to stdout: <cause>` to stderr and exits 1.

`completions` generates into a buffer and writes it through the same path.

Verified:

- `read --json`, `check --json`, `guide adr` and `read --plain` piped into `head -c 10` each print 10 bytes, leave stderr empty and exit 0.
- `cli/tests/closed_stdout.rs` closes the reader before spawn (`std::io::pipe`), or after the first bytes for `read --full`. With the broken-pipe arm sabotaged, every closed-pipe test fails.
- Unit tests in `cli/src/output/tests.rs` cover each outcome of the decision and a writer whose flush fails.
- Open-pipe output is byte-identical to before: the whole suite passes unchanged, and `read` and `check` JSON match the parity corpus on the 2500- and 10000-doc bundles.

`living-docs-core` still prints to stdout in a few functions the CLI never calls. That is out of scope here.
