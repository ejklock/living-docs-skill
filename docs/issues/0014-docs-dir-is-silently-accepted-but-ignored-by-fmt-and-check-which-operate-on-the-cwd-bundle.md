---
type: Issue
title: "--docs-dir is silently accepted but ignored by fmt and check, which operate on the cwd bundle"
description: Make fmt and check either honor --docs-dir or hard-error on it, so the flag can never silently rewrite a different tree than the one the user pointed at.
status: closed
timestamp: 2026-07-30T18:58:08Z
---

## Problem

`fmt` and `check` accept `--docs-dir` in both global and subcommand position without error, but ignore it and operate on the `docs/` bundle of the CURRENT WORKING DIRECTORY. Their `--help` does document that they match `[BUNDLE_ROOT]` positionally "rather than the global `--docs-dir`" — but accepting the flag silently and mutating a different tree than the one the user pointed at turns a documented quirk into a footgun. The other authoring verbs (`new`, `status`, `index`, `supersede`) honor the flag, so the same spelling changes meaning per verb.

## Repro (v0.8.0 release binary, macOS arm64)

```
mkdir -p repro/cwd/docs/adr repro/target/adr
cp <some-non-canonical-record>.md repro/cwd/docs/adr/
cp <same-record>.md repro/target/adr/
cd repro/cwd
living-docs --docs-dir "$PWD/../target" fmt   # exit 0, "1 record(s) rewritten"
```

Result: `repro/cwd/docs/adr/<record>.md` REWRITTEN (sha256 changed), `repro/target/adr/<record>.md` UNTOUCHED (sha256 identical). Same outcome with the flag in subcommand position (`living-docs fmt --docs-dir ../target` → "0 record(s) rewritten" because the cwd copy was already canonical). The positional form `living-docs fmt ../target` works as designed.

## Impact

In a downstream repo (ai-configs, 2026-07-30) an agent session ran `living-docs --docs-dir <scratch-copy> fmt` intending an isolated experiment; the CLI rewrote 145 records of the LIVE tree with exit 0, and the mutation was only caught later by `git status`. Recovery required restoring 137 files from HEAD.

## Proposal

Either honor `--docs-dir` in `fmt`/`check` as the bundle root, or hard-error when the flag is combined with these verbs ("fmt/check take a positional bundle root; --docs-dir is not honored"). At minimum, print the RESOLVED bundle root before rewriting. Silent acceptance + cwd fallback is the one behavior that should be impossible.
