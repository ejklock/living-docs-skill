---
type: Issue
title: Help lists the record types, references accept the directory name and a record path, and a type error names the valid ones
description: new --help enumerates the registered types, set and supersede resolve issues/0049 and docs/issues/0049-slug.md as references, and an unknown type or key error lists what is valid.
status: closed
timestamp: 2026-10-19T13:13:52Z
---

## 0056. Help lists the record types, references accept the directory name and a record path, and a type error names the valid ones

Every tenth CLI call in the owner's agent transcripts is `--help` (590 of 8,376, led by `new` with 169 and `set` with 140), and the most common usage error is a type token the registry does not know: `set issues/0049`, `set docs/adr/0023-slug.md`, `new issues "T"`. The help does not list the types, and the reference grammar accepts only `NNNN` or `type/NNNN`. This issue makes the surface answer those questions itself.

### Scope

- `new --help` and the root help list the registered type tokens from the registry (`adr`, `prd`, `issue`, `research`, `view`, `constitution`), never a hand-kept list.
- A type token also accepts the type's directory name (`issues` resolves to `issue`), in `new`, `index`, `set` and `supersede` references.
- A reference also accepts a record path: bundle-relative (`issues/0049-slug.md`), repo-relative (`docs/issues/0049-slug.md`) or absolute, resolved by directory and number prefix so a renamed file still resolves.
- An unknown type token exits 2 and lists the valid tokens; an unknown `set` key keeps listing the valid keys.
- Hostile references: a path outside the bundle, a path whose directory and number disagree with the bundle, and a path with no number prefix each exit 2 naming the rule.

### Decision

- Token and reference resolution live in one place (the paths and doc-type modules) and every verb calls it; option not taken: per-verb aliases.
- The error lists the whole token set; option not taken: a nearest-match suggestion, which the short list makes unnecessary.

### Acceptance

- `living-docs new --help` names the six tokens; the list changes when a registry row changes (asserted against `DOC_TYPES`).
- `new issues "T"` creates `docs/issues/NNNN-t.md`; `new bogus "T"` exits 2 and the message lists the six tokens.
- `set issues/0049 status closed`, `set docs/issues/0049-slug.md status closed`, `set issues/0049-slug.md status closed` and the absolute path each resolve the same record; `set 0049 status closed` is unchanged.
- `supersede issues/0001 issues/0002` resolves both.
- `set ../elsewhere/0049-x.md status closed` and `set adr/0049-an-issue-slug.md ...` where the file lives under issues each exit 2.
- The full suite and `make check` pass.

### Outcome

Delivered, uncommitted, in the working tree: `new --help` and `index --help` list the registered type tokens from the registry (`cli/src/args/help.rs › doc_type_help`); a directory name resolves as its type token in one place (`doc_type::spec_for`); `set` and `supersede` accept a record path (bundle-relative, repo-relative or absolute under the bundle) through `living-docs-core/src/commands/reference.rs`, which refuses `..`, a path outside the bundle, a non-type directory and a file without a record number, each naming its rule; an unknown type exits 2 listing the valid tokens. Proof: `cargo test -p living-docs --test reference_paths` (6 passed), `--test help` (46 passed), `cargo test --workspace` green (710), `se-gates check` pass.
