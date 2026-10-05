---
type: Issue
title: "Section schema in the registry: check requires Scope and Acceptance on live issues and read gains a contract level"
description: "Implements ADR 0064 for the issue type: a per-type section schema, a check pass for required sections on live records, and read --contract that omits detail-tier sections."
status: closed
timestamp: 2026-10-05T20:53:51Z
---

## 0052. Section schema in the registry: check requires Scope and Acceptance on live issues and read gains a contract level

Implements [ADR 0064](/adr/0064-each-doc-type-declares-its-sections-in-the-registry-required-or-optional-contract-or-detail-tier-so-check-enforces-the-required-ones-on-live-records-and-read-discloses-by-tier.md) for the issue type. Today a live issue can drop `Acceptance` and still pass `check`, and `read --full` hands an agent the delivery plan together with the contract.

### Scope

- A section schema field on `DocTypeSpec`: each entry a heading name, required or optional, contract or detail tier.
- The issue row: `Scope`, `Acceptance` required and contract; `Decision` optional and contract; `Plan`, `Outcome` optional and detail. Every other row empty.
- A `check` pass that fails a live record missing a required section; terminal and retired records skipped through the existing retired-record predicate (issue 0049).
- `read --contract`: preamble plus contract-tier and unknown sections, detail-tier sections omitted; text and JSON output (`body` carries the contract body). Mutually exclusive with `--full`.
- Issue template hints and the `issue-workflow` and `doc-trail` guide topics teach the tiers.

Kept unchanged: `read` default and `read --full` output, every other doc type's behavior, `new` output apart from the template hint text.

### Decision

- The finding is an invariant (non-zero exit), not an advisory: a missing acceptance on live work is a defect, and terminal records are already exempt. Option not taken: an advisory, which would leave the gap the ADR closes.
- The flag is `--contract`, not `--tier contract`: it sits beside the existing boolean `--full`. Option not taken: reviving ADR 0050's `--tier` enum, which would rename a flag callers already use.

### Acceptance

- A filled `new issue` record passes `check`; with `### Acceptance` removed it fails while `status: open` and passes after `living-docs set <n> status closed`.
- The finding names the record and the missing section.
- `read --contract` over an issue with `Scope`, `Acceptance`, `Plan` and `Outcome` prints the first two and neither of the last two, in text and JSON.
- A heading no schema names appears under `read --contract`.
- `read --full` and default `read` output over `docs/` are byte-identical before and after.
- A core test fails when a required section in any registry row is absent from that type's template.
- `living-docs check` over `docs/`, `cargo clippy -D warnings`, `scripts/check-file-size.sh` and `se-gates check` pass.

### Plan

1. Core: section schema types and the issue row in `doc_type.rs`; template-coverage test.
2. Core: heading extraction shared by `check` and `read`; the `check` pass and its registration.
3. Core and CLI: the contract level in `effective` render and the `--contract` flag.
4. Template hints and guide topics; dogfood `check` and `read --contract` over `docs/`.

### Outcome

Closed: `check` requires Scope and Acceptance on live issues, and `read --contract` omits Plan and Outcome. Default `read` and `read --full` output is byte-identical to the previous release. The workspace suite went from 601 to 636 tests. Two heading-parser gaps found in review are tracked in [issue 0053](/issues/0053-section-heading-parser-counts-indented-code-headings-and-always-drops-the-first-heading-as-the-title.md).
