---
type: Issue
title: Formal proofs of state and decision artifacts, model-checker export, and bounded model checking of the CLI
description: check proves state views and decision tables, a state view exports to a model-checker spec for the project's CI, and the CLI's own pure functions get bounded model checking.
status: open
timestamp: 2026-11-23T05:14:53Z
---

## 0152. Formal proofs of state and decision artifacts, model-checker export, and bounded model checking of the CLI

Follows the domain records of [ADR 0099](../adr/0099-domain-records-are-a-registry-type-in-docs-context-nested-in-folders-by-bounded-context.md) and the generated views of [ADR 0101](../adr/0101-generated-view-regions-carry-a-hash-of-their-inputs-stamped-by-manage-view-splice-and-checked-for-staleness.md): the engineering artifacts a bundle keeps (state diagrams, decision tables) are small formal models, and the CLI itself has pure functions whose safety rests on universal properties.

### Scope

- Proofs over a bundle's artifacts inside `check`, as advisories: a state view's reachability, dead states and dangling transitions; a decision table's completeness and consistency.
- A deterministic export of a state view to a model-checker specification, which the project's own CI runs; the binary never runs the checker.
- Bounded model checking of the CLI's own pure functions (path validation, glob pruning, inputs hash, slug, marker parser), in a manually dispatched CI job, with nothing added to the binary.

Kept: the determinism boundary (no model inside the binary, no external tool run by `check`), and every existing rule.

### Decision

- Formal verification (owner, 2026-11-23): all three arms: proofs of state and decision-table artifacts in `check`; export of state views to a model-checker specification for the project's CI; bounded model checking of the CLI's own pure functions. Not taken: any one arm alone (the session recommended the `check` proofs first, the export only on demand).
- Proofs Kani cannot reach as the code stands (owner, 2026-11-23): refactor production code so glob pruning soundness and hash order independence become provable: a segment matcher without `regex` beside the regex one, and the canonical byte stream split from the SHA-256 digest. Not taken: property tests with a random seed for those two (rejected for determinism in issue 0149); leaving them to today's example tests.
- Export format (owner, 2026-11-23): TLA+ only; Alloy on demand. Not taken: TLA+ and Alloy, two output grammars to keep.
- Materiality (design review, 2026-11-23): one ADR for the `check` proofs and the decision-table grammar, one ADR for the TLA+ export; the bounded model checking of the CLI is this issue's Decision, with no consumer contract. Not taken: one ADR per arm.
- State checks (design review): unreachable state, dead end (a non-final state with no outgoing transition and no ancestor composite that has one), and more than one initial transition in a scope; "undeclared state" and "transition from a final state" are dropped, since Mermaid creates every named state and `[*]` on the right is always an end.
- Kani locally (owner, 2026-11-23): Kani and its toolchain are installed on the development machine so every harness runs before a push; the manual-dispatch CI job still runs them. Not taken: running Kani only in CI, where a harness would land on the branch without ever having run.
- Proof scope after measuring Kani (owner, 2026-11-23): at the first bounds only the CRLF harness closed (4 bytes, about 3.5 minutes); slugify and the subfolder validation never finished, because slugify runs through Unicode normalisation tables. Kani now proves slugify on ASCII input with the Unicode transliteration stubbed, the subfolder validation against the proved slugify contract, the marker parser and CRLF at small bounds; a bounded exhaustive enumeration in the normal test suite covers every string up to a small length over an alphabet that includes non-ASCII letters, `/`, `..`, CR, LF and space. Not taken: compositional Kani alone; enumeration alone; keeping only the harnesses that closed.
- Proof closure (owner, 2026-11-23, after a second Kani round): bounded exhaustive enumeration in the normal test suite is the main proof of slugify, the subfolder validation and CRLF normalisation (11,111, 19,608 and 127 cases in 0.09 s); Kani keeps only the harnesses that finish, in the manual job: CRLF at 4 bytes, which covers all 256 byte values; the marker parser gave no verdict at 1- and 2-byte gaps, so a bounded enumeration over marker, fence and text lines proves it never panics and returns ordered spans. Not taken: Kani at smaller bounds than the enumeration already covers; Kani with a model replacing std Path.
- Glob pruning and hash order (owner, 2026-11-23): proved by bounded enumeration in the normal test suite, with no production refactor and no Kani harness. Not taken: the planned refactor for Kani, which the second Kani round showed would likely not close; closing them on today's example tests alone.
- S5 lands as approved and its two normal review findings become S8 (owner, 2026-11-23): the test-first hook treats the test-only `exhaustive.rs` modules as production code, so no edit can follow a red run. Not taken: a deliberately wrong pinned count to force a red run; fixing the shared hook before landing.
- MCP export is its own `manage` operation (session, 2026-11-23): `export_view` with `view` and `format`, so the published schema of the splice `view` operation stays unchanged. Not taken: an optional `export` field on the splice arguments, which would make `inputs` and `mermaid` optional for existing callers.

### Acceptance

- To be set by the ADRs this issue produces, after a design review.

### Plan

- S1: the ADRs for the `check` proofs and decision-table grammar, and for the TLA+ export | files: docs/adr | after: none
- S2: `state-machine` advisory over state views | files: living-docs-core/src/check (new module), check/kinds.rs, check/mermaid.rs, skills/living-docs/rules | after: S1, issue 0151 S3 (kinds.rs)
- S3: Kani harnesses for slug, `--dir` validation, marker parser and newline normalisation, a manual-dispatch workflow and the `check-cfg` lint entry | files: living-docs-core/src/*/proofs.rs, Cargo.toml, .github/workflows | after: none
- S4: `decision-table` advisory | files: living-docs-core/src/check (new modules), check/kinds.rs, skills/living-docs/templates/domain.md, skills/living-docs/rules | after: S2
- S5: bounded enumerations proving glob-pruning soundness and hash-order independence | files: living-docs-core/src/inputs_hash/exhaustive.rs, living-docs-core/src/exhaustive.rs | after: S3
- S6: `manage view <view> --export tla` | files: cli/src/args/manage.rs, cli/src/commands, living-docs-core, skill recipe | after: S2, issue 0151 S2
- S7: the MCP `manage` tool's view operation exposes `export` with the same output as `manage view --export tla` (review of S6 found the parity gap) | files: mcp/src/tools.rs, mcp tests | after: S6
- S8: compile each glob once in the pruning and hash enumerations (15 to 44 s today) and state in their docblock that they prove `can_match_below` sound against `is_match` through the test's own walk model, the walker's use being covered by fs-store's examples (review of S5; blocked until the test-first hook counts `exhaustive.rs` as a test file) | files: living-docs-core/src/inputs_hash/exhaustive.rs, living-docs-core/src/exhaustive.rs | after: S5
- S9: one CommonMark fence helper shared by the decision-table reader, sections and generated regions, so a longer fence holding a shorter one is skipped (review of S4) | files: living-docs-core/src/check/decision_table/parse.rs, living-docs-core/src/sections.rs, living-docs-core/src/generated_region.rs | after: S4
