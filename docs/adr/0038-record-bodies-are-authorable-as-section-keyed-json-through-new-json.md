---
type: ADR
title: Record bodies are authorable as section-keyed JSON through new --json
description: "new gains --json: a sections object whose keys must match the type template's own headings; the CLI validates keys, fills each named section and the title heading, and leaves unnamed sections as template guidance — one call authors a complete record with no scaffold-then-edit round trip."
status: Proposed
timestamp: 2026-08-14T08:21:07Z
---

# 0038. Record bodies are authorable as section-keyed JSON through new --json

## Context

The authoring loop today is scaffold-then-edit: `new` writes a template full of guidance
comments and placeholders, the model reads it back, then edits the body. That costs a
read plus an edit per record, spends tokens re-processing boilerplate the model never
needed, and — decisive for compliance — makes the *easy* path a file edit, which is
exactly the reflex that leaks into hand-editing records the CLI owns. Models are
conditioned by tool-calling (MCP-style) interfaces: one call, structured payload, done.
The CLI is the harness-agnostic single entry point (ADR 0001), so the structured payload
belongs on the CLI itself.

Epistemic type: **Judgment**. External critic: **pending** (real authoring usage).

## Decision

We will add `--json <sections>` to `living-docs new`: a flat JSON object whose keys must
match the type template's own body headings (every `##`/`#` heading after the title
heading, e.g. `Context`, `Decision`, `Consequences`, `References` for an ADR). The CLI
validates keys against the template — an unknown key is refused naming the valid
sections — fills each named section's content in place of the template guidance, fills
the title heading (`# NNNN. <title>` for numbered types), and leaves unnamed sections as
template guidance for later authoring. `@file` and `-` (stdin) payload forms are
accepted. Batch creation is shell composition — one `new --json` per record, chained.

## Consequences

**Easier / gained:**
- One call authors a complete record: no scaffold read-back, no boilerplate tokens, and the compliant path becomes the cheapest path.
- Section keys are derived from the registry's own templates, so the contract can never drift from the template (a renamed heading renames the key).
- Composes with `--description` / `--kind`; everything `check` enforces still applies to the result.

**Harder / accepted trade-offs (the declared sacrifice):**
- Section granularity, not free-form: a body that deviates from the template's headings
  still needs a follow-up edit. Steelman of the rejected alternative — a single
  `--body` string would cover any shape; rejected because it validates nothing, still
  makes the model type the headings, and reopens the drift the section contract closes.
- An MCP server would match model conditioning even better; rejected for this slice —
  the CLI stays the single harness-agnostic surface, and an MCP wrapper can later expose
  this exact JSON contract without a second implementation.

**Follow-ups:**
- Skill `procedure`/`migration` topics teach `--json` as the preferred authoring path.

## Verification

**Implementation impact:** `living-docs-core/src/commands/new/sections.rs` (new),
`new.rs` options plumbing, `cli/src/{args,main}.rs`, `web` caller.

**Verification criteria:**
- `new adr "T" --json '{"Context": "...", "Decision": "..."}'` writes a record whose
  sections carry the payload, whose H1 is `# NNNN. T`, and which passes `check`.
- An unknown key is refused listing the type's sections; a non-object payload is refused.
- Fitness function: `commands::new::sections` unit tests pin fill, validation, and the
  every-registry-template-headings contract.

# References

[1] [ADR 0001](/adr/0001-living-docs-cli.md)
