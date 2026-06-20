# Living Docs enforcement — harness-agnostic ratchet triggers

The mechanical Living Docs invariants are checked by one instrument:
`skills/living-docs/scripts/lint-docs.sh`. Its **diff-aware ratchet** mode
(`--ratchet <baseline-ref>`) fails **only on NEW violations** introduced by a
change and **grandfathers pre-existing debt** — the same semantics the repo's
other gates use.

This directory holds two **thin triggers** that call that one CLI. Neither
reimplements any check:

| Trigger | When it runs | Bypassable? |
|---|---|---|
| `pre-commit.sh` | locally, on `git commit` | yes — `git commit --no-verify` |
| `../.github/actions/living-docs-ratchet/` | in CI, on a pull request | no — the unbypassable backstop |

**Why one CLI instead of a per-harness plugin.** Enforcing at the git boundary
(pre-commit) and in CI (the Action) is *harness-agnostic*: it covers Claude
Code, Pi, OpenCode, Cursor, Copilot **and** plain human commits with one
mechanism. A per-harness plugin would be the optional *in-loop* layer that calls
this **same** CLI; it is a separate scaffold and is **not** built here.

---

## 1. Pre-commit hook (local, fast feedback)

The hook runs `lint-docs.sh --ratchet HEAD` on your docs bundle and blocks a
commit that introduces a new violation.

Git hooks **do not travel by clone** — install per checkout. The clean way is
`core.hooksPath` (one config, no copying into `.git/hooks`):

```bash
# from your repo root, point git at a tracked hooks directory
mkdir -p .githooks
cp path/to/living-docs-skill/enforcement/pre-commit.sh .githooks/pre-commit
chmod +x .githooks/pre-commit
git config core.hooksPath .githooks
```

Now every `git commit` runs the ratchet. To bypass intentionally:

```bash
git commit --no-verify
```

Configuration (env vars, optional):

- `DOCS_BUNDLE` — the bundle to lint (default: `docs`).
- `LINT_DOCS` — explicit path to `lint-docs.sh`. If unset, the hook auto-discovers
  it under `skills/living-docs/scripts/`, `scripts/`, `.claude/skills/...`, or `PATH`.

Defensive by design: if `lint-docs.sh` or `git` is missing, or there is no docs
bundle, the hook exits `0` — it never wedges a commit because tooling is absent.

---

## 2. GitHub Action (CI, unbypassable backstop)

A **composite action** at
[`.github/actions/living-docs-ratchet`](../.github/actions/living-docs-ratchet/action.yml).
On a pull request it runs `lint-docs.sh --ratchet <base-ref>` with the baseline
set to the PR's **merge base** (so only violations the PR introduces fail the
check).

Drop this into a consumer repo at `.github/workflows/living-docs.yml`:

```yaml
name: Living Docs ratchet
on:
  pull_request:

permissions:
  contents: read

jobs:
  living-docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0   # required: the ratchet needs the base ref's history

      - uses: ejklock/living-docs-skill/.github/actions/living-docs-ratchet@main
        with:
          bundle: docs                 # optional, default: docs
          base-ref: ${{ github.event.pull_request.base.sha }}  # optional; auto-detected if omitted
```

`fetch-depth: 0` is **required** — the ratchet materializes the baseline with
`git worktree add`, which needs the base commit present in the checkout.

If you vendor `lint-docs.sh` somewhere non-standard, pass `lint-docs:` with its
path. The action degrades gracefully: a missing baseline ref is treated as empty
(every current violation counts as new — fail-closed).
