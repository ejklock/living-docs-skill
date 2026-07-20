# The three buckets (decide per doc, explicitly)

| Bucket | Examples | Public? |
|---|---|---|
| **Scaffold** — the method | `skills/*/SKILL.md`, templates, engine source, README, LICENSE | yes |
| **Accumulation** — the moat | full lessons, memory DB, client specs, in-flight bets, outcome data | **never** |
| **Curated showcase** — portfolio | hand-picked, cleaned ADRs/research (e.g. a "why I rejected X" record) | yes, deliberately |

The showcase is the only nuance: each showcased doc is elevated **one at a time** by setting
`visibility: showcase` on it, never by publishing `docs/adr/**` wholesale. Curation is manual on
purpose.

**Derived artifacts sit in the same bucket as their sources.** Embeddings, traces, and eval outputs
derived from Accumulation content are Accumulation — an embedding is a **copy** of its source text,
not an opaque transform. A memory DB (vectors included) never exports, and that line is the reason,
so the rule survives future judgment calls.
