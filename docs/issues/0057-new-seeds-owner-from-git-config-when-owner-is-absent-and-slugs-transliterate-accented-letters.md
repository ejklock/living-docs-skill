---
type: Issue
title: new seeds owner from git config when --owner is absent and slugs transliterate accented letters
description: new adr without --owner reads git config user.name as the owner instead of leaving the field out, and slugify maps accented Latin letters to their ASCII base so horario never becomes hor-rio.
status: closed
timestamp: 2026-10-19T13:13:52Z
---

## 0057. new seeds owner from git config when --owner is absent and slugs transliterate accented letters

Two recurring `check` findings in the owner's agent transcripts are caused by the tool, not the author. "ADR record has no owner" appears 48 times because agents call `new adr` without `--owner`; and a heading-versus-title mismatch appears 45 times because agents strip accents from titles to dodge a slug like `hor-rio`, which `slugify` produces by dropping every non-ASCII letter (recorded in the owner's memory as a known trap).

### Scope

- `new` seeds `owner` from `git config user.name` (then `user.email`) when `--owner` is absent and the type's registry row requires an owner. A type that does not require one is never seeded. No git identity means no owner, as today, with no error. The seeded value is reported on stderr as an informational line `--quiet` silences.
- `slugify` transliterates accented Latin letters to their ASCII base before the ASCII filter (`horário` to `horario`, `ação` to `acao`), with a small table for the letters that do not decompose (`ß` to `ss`, `ø` to `o`, `æ` to `ae`, `œ` to `oe`, `ł` to `l`, `ð` to `d`, `þ` to `th`). `new` and `set title` both use it.
- Every existing ASCII slug is unchanged.

### Decision

- The owner fallback applies only where the registry requires an owner; option not taken: seeding every type, which adds a field nobody asked for on issues.
- Transliteration uses Unicode canonical decomposition with combining marks stripped, through the `unicode-normalization` crate; the gap is that the standard library has no normalization and the baseline is the slug `hor-rio`. Option not taken: a hand-written accent table, which is the same table the crate derives from the Unicode data.
- The heading-versus-title advisory stays strict; with accents surviving in the slug there is no reason left to strip them from a title.

### Acceptance

- In a repository with `user.name` set, `new adr "T"` writes `owner: <user.name>` and `check` reports no owner finding; `--owner X` wins over git config; with no git identity the record has no `owner` field and the verb succeeds.
- `new issue "T"` never writes an owner from git config.
- `slugify("Horário de atendimento")` is `horario-de-atendimento`; `slugify("Ação")` is `acao`; `slugify("Straße")` is `strasse`; `slugify("My Title")` is `my-title`.
- `set NNNN title "Horário"` renames the file to a slug carrying `horario`.
- Every record under `docs/` keeps its filename when its title is re-slugged (a test over the bundle).
- The full suite and `make check` pass.

### Outcome

Delivered, uncommitted, in the working tree: `transliterate` and the accent-folding `slugify` in `living-docs-core/src/paths.rs` (crate `unicode-normalization`), owner seeded from `git config user.name` then `user.email` for `requires_owner` types in `cli/src/commands/new.rs`, silenced by `--quiet`. Proof: `cargo test -p living-docs --test new_owner` (6 passed), `cargo test -p living-docs --test slug_transliteration` (3 passed), `cargo test --workspace` green, clippy and fmt clean, `living-docs check` OK. The bundle-stability acceptance was proved as a differential test (`slugify(title)` equals the old ASCII-only rule for every record under `docs/adr` and `docs/issues`), because filenames and titles already diverge in the tree (ADR 0015).
