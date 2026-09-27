---
type: Issue
title: check mermaid accepts diagrams that mermaid.js rejects
description: merman-core 0.7.0 parses flowchart edges that mermaid.js rejects, so check passes a diagram that fails to render on GitHub
status: closed
timestamp: 2026-09-27T02:35:31Z
---

## 0051. check mermaid accepts diagrams that mermaid.js rejects

`check` validates mermaid fences in-process through `merman-core` 0.7.0 ([ADR 0013](/adr/0013-mermaid-validation-runs-in-process-via-merman-core-not-a-docker-mermaid-cli-shell-out.md)), and the module calls it "the real Mermaid grammar parser". It is more lenient than mermaid.js, the parser GitHub renders with. A diagram can pass `check` and still fail to render.

Found on 2026-09-26 in the relent repository, while repairing 17 diagrams that `check` reported as invalid. One repair passed `check` but broke the diagram:

```text
flowchart LR
    AX -- "~4.0:1 today" --> per-story disable --> GAP[blind spot]
```

- `living-docs check` (0.20.0): no violation.
- mermaid.js `Diagram.fromText`: `Parse error … got 'NODE_STRING'` on the bare multi-word node `per-story disable`.
- Rendered: `per-story` becomes a dead-end node and `GAP` is dropped, so the gate passes a diagram that silently loses meaning.

A second gap in the same run: the full `check` prints only `invalid mermaid diagram` with the fence line. The parser's own message appears only under `check --mermaid-only --json`, so every failure took an extra command to diagnose.

### Scope

- A regression test for each known leniency, starting with the one above. It pins the current verdict of `merman-core`, so an upgrade that fixes the leniency fails the test and flips it to rejected.
- Find out whether a newer `merman-core` release rejects the fixture. If none does, report it upstream with the fixture and record the pin.

### Decision

No `merman-core` release rejects the fixture. On 2026-09-26, 0.7.0 (latest stable) and 0.8.0-alpha.6 both returned a parsed diagram under `ParseOptions::strict()`. Both split `per-story disable` into two nodes with no edge between them. mermaid.js 11.16.1 and 11.17.2 reject it with `got 'NODE_STRING'`.

The owner chose to keep the pin at `=0.7.0` and to report the leniency upstream as [Latias94/merman#150](https://github.com/Latias94/merman/issues/150). No in-house grammar check and no fork: both would need a new ADR to amend ADR 0013. The first acceptance criterion changes to match this decision.
- The full `check` carries the parser's error detail in each mermaid violation message, not only `--mermaid-only`.
- Kept: validation stays in-process and deterministic. Replacing `merman-core` with a mermaid.js process would reverse ADR 0013 and needs its own ADR first.

### Acceptance

- A unit test pins that `merman-core` 0.7.0 accepts the fixture above, and links upstream #150. When a pin upgrade fixes the leniency, the test fails, and the fixture must then make `check` exit 1.
- A mermaid violation printed by the full `check` includes the parser's message, e.g. the expected and found token.
- Every existing mermaid fixture keeps its current verdict.
- `cargo test --workspace`, clippy, fmt and `scripts/check-file-size.sh` stay green.
