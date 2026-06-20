# lint-docs.sh fixture corpus

The parity/fixture test that makes `skills/living-docs/scripts/lint-docs.sh`
trustworthy. The linter is the *instrument* for the mechanical Living Docs
invariants ("a constraint without an instrument is a vibe"); this corpus proves
it (a) stays silent on a clean bundle and (b) catches each violation it claims
to — one violation per dirty fixture, in the spirit of an arch-gate parity test
(clean partition passes silently; dirty partition each caught 1/1).

## Run

```bash
make test-lint-docs        # default-mode fixture/parity corpus
make test-ratchet          # diff-aware ratchet corpus
# or directly
tests/lint-docs/run.sh
tests/lint-docs/run-ratchet.sh
```

The runners exit non-zero if any case misbehaves. CI runs both on every push/PR.

## Two corpora

- **`run.sh`** — default whole-bundle mode: clean passes silently, each
  mechanical violation is caught (one per dirty fixture, table below).
- **`run-ratchet.sh`** — the **diff-aware ratchet** (`lint-docs.sh --ratchet
  <ref>`): proves only NEW violations block while pre-existing debt is
  grandfathered. Each case spins up a throwaway git repo (init → commit a
  baseline → mutate the working tree), exercising the real `git worktree add`
  baseline materialization. Cases: `new-blocks` (introduce one violation →
  exit 1, named), `preexisting-ok` (committed debt + unrelated clean change →
  exit 0), `fix-passes` (remove a pre-existing violation → exit 0),
  `baseline-absent` (ratchet against a missing ref → baseline empty, violation
  counts as new → exit 1, fail-closed).

## Layout

| Path | Role |
|---|---|
| `fixtures/clean/docs/` | a minimal, hand-rolled clean bundle → must exit `0` |
| `fixtures/dirty-NN-*/docs/` | a minimal bundle with **exactly one** violation → must exit `1` |
| `expect/dirty-NN-*.grep` | substring that must appear in the linter output for that case |
| `expect/dirty-NN-*.exit` | (optional) expected exit code; defaults to `1` for dirty cases |
| `run.sh` | the runner — asserts exit code + message substring, prints PASS/FAIL + summary |

The shipped worked example `examples/linkly/docs` is also asserted clean as a
second clean-partition case.

## Coverage — one fixture per mechanical check

| Fixture | Check exercised |
|---|---|
| `dirty-01-missing-root-index` | bundle-root `index.md` missing |
| `dirty-02-no-frontmatter` | non-reserved `.md` with no frontmatter |
| `dirty-03-empty-type` | frontmatter present but `type` empty |
| `dirty-04-index-has-frontmatter` | non-root `index.md` carrying frontmatter (OKF §6) |
| `dirty-05-root-index-no-okf` | bundle-root `index.md` frontmatter without `okf_version` (the allowed-exception path) |
| `dirty-06-orphan` | concept file not listed in its directory `index.md` |
| `dirty-07-unreachable-index` | directory `index.md` not reachable from the bundle-root index |
| `dirty-08-broken-link` | broken local markdown link |
| `dirty-09-superseded-empty` | `status: Superseded` with empty `superseded_by` |
| `dirty-10-superseded-missing-target` | `superseded_by: NNNN` with no matching record |
| `dirty-11-no-dir-index` | concept file in a directory with no `index.md` |

The allowed case "bundle-root `index.md` with `okf_version`" is covered by the
clean fixtures (their root index declares `okf_version` and passes).

Each dirty fixture is verified to trigger **exactly one** violation, so a green
case can only mean the intended check fired.
