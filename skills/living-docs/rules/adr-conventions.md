# ADR Conventions (MADR-lite)

An Architecture Decision Record captures **one** decision: the context that forced it, the choice made, and the consequences accepted. ADRs are how a future reader understands *why* the code is the way it is — and why a tempting alternative was not taken.

## Format

Each ADR is an **OKF concept** (`type: ADR`) — see the `okf-knowledge-format` skill. Use a lightweight MADR structure — frontmatter plus three body sections, no ceremony:

- **`status` (frontmatter)** — `Proposed` | `Accepted` | `Superseded` | `Deprecated`. Lives in the YAML frontmatter, not a body line. Supersession is recorded with the `supersedes` / `superseded_by` frontmatter keys (NNNN).
- **Context** — the forces at play: the problem, constraints, and what made a decision necessary. Written so a newcomer understands the pressure without prior knowledge.
- **Decision** — the choice, stated in active voice ("We will…"). Specific and testable.
- **Consequences** — what becomes easier, what becomes harder, what is now forbidden. Include the trade-offs you are knowingly accepting, not just the upside.

See `templates/adr.md` for the skeleton. Required sections: `Context`, `Decision` and `Consequences`; `check` enforces them on live records (a Deprecated or Superseded ADR is history and is not held to them).

## Rules

1. **One decision per ADR.** If you are recording two decisions, write two ADRs. Bundled decisions can't be superseded independently.
2. **Number sequentially, never reuse.** `docs/adr/NNNN-kebab-slug.md`. The number is permanent even after the ADR is superseded.
3. **Supersede, never delete or rewrite.** When a decision changes:
   - Set the old ADR's frontmatter `status: Superseded` and `superseded_by: NNNN` (do not edit its Decision/Context — that is history).
   - Write a new ADR with `supersedes: NNNN` that references the one it supersedes in its Context.
   - If the old ADR is only *partially* affected, annotate the affected section with a pointer to the new ADR rather than rewriting it.
   - `supersede` also writes a callout at the top of the old ADR's body, above its heading, naming the successor. Never write or edit that callout by hand; it is the signal that stops a reader from acting on a retired record.
4. **Record load-bearing rejections.** When a design candidate is rejected for a reason a future explorer would otherwise re-discover the hard way, that reason is an ADR. Skip ephemeral ("not worth it now") or self-evident reasons.
5. **Link the evidence.** If the decision rests on research, link the research artifact bundle-relative (`/research/<…>/report.md`). If it implements a requirement, link the PRD/issue.
6. **Name the fitness function for measurable characteristics.** When an ADR decides a measurable architecture characteristic (a latency budget, a dependency-direction rule, a coupling/granularity constraint), the Consequences section SHOULD name the **fitness function** that enforces it — the executable check (a test, a build/lint rule, an arch-unit assertion) that lives in the suite and fails when the characteristic is violated. The ADR records *why*; the fitness function keeps it true. A measurable decision without an instrument is a vibe (see `memory/lessons.md`).
7. **Index every ADR, and keep an active view.** Add a row to `docs/adr/index.md` (OKF reserved listing — number, title, status). The index carries no frontmatter; the decision log is the listing plus each ADR's `status`/`superseded_by` frontmatter. **As the corpus grows, split the listing by status** — an `## Active` section above a `## Superseded` section — so a reader sees what is *in force* without reading through history. Append-only + supersede means `docs/adr/` only grows; this convention keeps "indexed or it doesn't exist" from degrading into "indexed but unreadable by volume". `status` already lives in frontmatter, so the split is mechanical. Each retired row names its successor under a one-line history note atop the section; `living-docs index` writes both — never hand-edit a row or the note. See the worked [`examples/linkly/docs/adr/index.md`](../../../examples/linkly/docs/adr/index.md).
8. **Bind measurable decisions to a check (optional `## Verification`).** When an ADR must be honored in code, add a `## Verification` block (see `templates/adr.md`): the files it touches and **checkable** verification criteria — ideally a named fitness function (rule 6). This closes the doc → implement → verify loop a review step can consume; a structural decision a future agent must respect should not leave "did we honor it?" to inspection. Omit the block for a purely advisory record.
9. **Test-strategy decisions are ADRs, not a new record type.** A *test* decision with a rejected alternative and a consequence — a non-default test level/technique for a behavior, a deviation from the project's standing test bar, a deliberate decision not to test a seam (with the risk accepted), a golden-master/characterization oracle choice — is recorded as an ordinary ADR carrying `tags: [testing]` (a "test-strategy ADR"). The *what/how* of testing lives in the tests themselves (the scenarios and the suite); the ADR holds only the *why*. There is deliberately **no "Test Decision Record"** — the field's direction is "**Any** Decision Record" (one template absorbs domain decisions, MADR 3.0), and "TDR" already names "Technical Debt Record". Provenance (instrumentalized, not invented): the *what* is Specification by Example / Given-When-Then (ADZIC; NORTH); the *vocabulary* of a test decision (policy/strategy/approach/level) is ISTQB / ISO-IEC-IEEE 29119; the absorb-into-ADR stance is MADR "Any Decision Records" (ZIMMERMANN).

10. **Materiality — a decision earns an ADR only when it is expensive to reverse.** The trail is gated by *materiality*, not by *layer*. A decision gets its own ADR when it changes a stated invariant or constitution article, a public contract or schema, a dependency direction or module boundary, or a pinned external dependency — or it reverses a prior ADR. A decision that is cheap to reverse lives in the body of the issue that carries the work, not in an ADR. The one test: **"would a future reader pay to rediscover this?"** A *material* decision shipped without its ADR is an incomplete change; a cheap one recorded in its issue is complete. Do not manufacture a record per layer.
    - *Stays in the issue:* "we'll name the flag `--include-stale`, not `--with-stale`" — a rename, reversible in one commit. Put it in the issue's `## Decision`; do not open an ADR.
    - *Earns an ADR:* "records live as `.md` in git, never in a database" — fixes the source of truth every verb and every consumer builds on; expensive to reverse. Write the ADR.

## Anti-patterns

- Editing an accepted ADR's Decision to reflect a new choice — that erases history. Supersede instead.
- "Status: Accepted" with an empty Consequences section — every decision has trade-offs; if you can't name them, the decision isn't understood yet.
- An ADR that restates the code. ADRs explain *why*, not *what*. The code says what.
- Re-litigating a decision an existing ADR already settled without marking the contradiction explicitly.
