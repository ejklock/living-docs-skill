# Enforcement modes

The five Core invariants (in the living-docs SKILL.md) are **always** hard stops — they hold in every project regardless of mode (an orphan file or a silent rewrite is never acceptable). What the *mode* governs is the **doc trail** (constitution → PRD → ADR/BDR → issues → code): how strictly the agent refuses a structural or behavioral change that ships without its decision record.

The mode is chosen **once, by the user, the first time living-docs runs in the project**, and persisted in the project guide. The default is `strict` — the discipline is **opt-out, not opt-in**, because a flow nobody is forced to follow is the flow that rots first.

| Mode | Doc trail | Agent behavior |
|---|---|---|
| `strict` (default) | Mandatory | Refuses to report a structural/behavioral task complete without its required doc (PRD/ADR/BDR/issue). Same hard-stop weight as the five invariants. |
| `guided` | Prompted | Pauses and asks the user before skipping a doc-trail step. The user may waive a step per task; the waiver is not remembered. |
| `lite` | Advisory | Only the five invariants are hard stops. The doc trail is recommended, never enforced (this is the pre-0.3 behavior). |

### Mode governs completion, not elicitation

A frequent misread is that `strict` means "the agent interrogates every decision." It does not. The mode governs **completion enforcement** — whether a structural/behavioral task may be reported done without its doc. It says nothing about *how the decision inside that doc was reached*.

**Decision elicitation (grilling) is a separate, always-on concern**, independent of mode. A load-bearing decision is never recorded from the agent's own inference alone: before writing an ADR/BDR, surface the decision, **≥2 materially-distinct alternatives**, and a recommendation to the user, then record the chosen option **and the rejected ones**. Run the `grill-me` companion (see *Composition*) to drive that interrogation if it is installed; otherwise do the lightweight inline version. **Never write an ADR for a decision the user was never asked about** — in any mode. Mode changes whether you may *ship* without the doc; it never licenses inventing the decision the doc records.

### First-run question

When living-docs is invoked in a project and **no enforcement preference is yet persisted** (no `## Living Docs` block in the project guide), ask the user once, *before* doing the work:

> This project hasn't set a living-docs enforcement mode yet. How strictly should the doc trail (PRD → ADR/BDR → issues) be enforced?
> - **strict** (recommended) — every structural/behavioral change must carry its doc; I'll refuse to call a task done without it.
> - **guided** — I'll ask before skipping a doc-trail step.
> - **lite** — only the five invariants are enforced; the doc trail is advice.

Persist the answer immediately in the project guide (`CLAUDE.md`, where it enters context at session start), then proceed:

```
## Living Docs
enforcement: strict   # strict | guided | lite
onboarded: <YYYY-MM-DD>
```

**Absence of the block is the only first-run signal**; presence of any valid `enforcement` value means onboarded — never ask again, just read it and apply it. To change modes later, the user edits the block.

Doc-trail enforcement is a **judgement** call (there is no sound oracle for "did this change need an ADR"), so it lives with the agent, not with `living-docs check`. The mechanical invariants (frontmatter, indexing, links, supersede) are checked the same way in every mode.

## Agent enforcement (refusal triggers)

The value of this skill is not the discipline — humans abandon "nothing structural without
its doc" at the first deadline. The value is an **agent that enforces it automatically**. So
the invariants are not advice; they are **hard stops**. When acting as the agent, do **not**
report a docs-touching task as complete if any of these hold — fix it or surface it first:

1. **Orphan.** A new or moved concept file is not listed in its directory `index.md` (and that
   directory is not reachable from the bundle-root `docs/index.md`). *Indexed or it doesn't exist.*
2. **Stale diagram.** A structural change (schema, module layout, data flow, new component)
   landed without updating its Mermaid diagram in the **same** change.
3. **Silent rewrite.** A decision/requirement was edited in place instead of superseded — or a
   record is `status: Superseded` with no `superseded_by`.
4. **Untyped doc.** A non-reserved `.md` is missing frontmatter or a non-empty `type`; or an
   `index.md`/`log.md` carries frontmatter (except the bundle-root `index.md`).
5. **Broken link.** A bundle-relative (`/…`) or relative link points at a file that does not exist.
6. **Duplicate home.** The same fact now lives in two files (cross-reference instead).
7. **Broken doc trail** *(mode-gated — see Enforcement modes)*. A structural change shipped without
   its ADR, or a behavioral change without its BDR. Under `strict` this is a blocked task — refuse it
   like an orphan. Under `guided`, pause and ask the user before proceeding. Under `lite` it is
   advisory only.

Triggers **1, 3, 4, 5** are mechanical — run `living-docs check` (below) and treat a non-zero
exit as a blocked task, not a warning. Triggers **2**, **6**, and **7** are semantic (no sound oracle)
and stay a judgement call: inspect the diff before declaring done. Trigger **7** additionally depends
on the project's enforcement mode.
