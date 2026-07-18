---
name: research-artifacts
description: Organize, format, and index research as OKF-conformant knowledge bundles — dated, sourced, append-only snapshots of the external evidence behind a decision (technology evaluations, library comparisons, industry-practice surveys). Use when recording research output, structuring a docs/research/ session, enforcing source discipline (primary sources, vendor-COI flags, inference-vs-fact labels, confidence levels), or wiring the research → decision → issue traceable chain. Pairs with deep-research (which gathers/verifies the evidence) and living-docs (which owns the ADR/BDR/issue artifacts research feeds into).
version: "0.6.0"
metadata:
  type: skill
  layer: procedural
  tags: [documentation, research, evidence, okf, sourcing, traceability]
---

# Research Artifacts

Research records the external evidence behind a decision: technology evaluations, library comparisons, industry-practice surveys. Research is **dated, sourced, and append-only** — a snapshot of what the evidence said at a point in time, not a living opinion.

This skill defines how research is *organized, formatted, and indexed*. The `deep-research` skill defines how it is *gathered and cross-verified*; `living-docs` owns the decisions and issues it feeds. The three compose. Research artifacts are **OKF concepts** — see the `okf-knowledge-format` skill for the frontmatter/reserved-file rules applied here.

---

## Using this skill (progressive disclosure)

This SKILL.md is a **slim stub** — a trigger plus a task->topic router. The `living-docs` CLI holds the full research rules, source discipline, structure and traceable chain, and discloses them progressively. **Before authoring anything, load the topic for your task:**

- `living-docs skill research-artifacts --list` — discover every topic.
- `living-docs skill research-artifacts --topic <topic>` — load that topic.

Piped output is minified JSON (machine default); `--plain` for human text, `--json` to force JSON. Topics: rules, structure, research-report, research-index, research-general-references, about.

---

## When to invoke

- Recording the output of a research session into `docs/research/` — `living-docs skill research-artifacts --topic research-report`.
- Structuring or indexing a research note (`docs/research/NNNN-<slug>.md` + the index listing + the general roll-up) — `living-docs skill research-artifacts --topic research-index` / `--topic research-general-references` for the templates, `--topic structure` for the layout rules.
- Enforcing source discipline on a draft (primary sources, vendor-COI flags, inference-vs-fact labels, confidence levels, fetch-failure notes) — `living-docs skill research-artifacts --topic rules`.
- Wiring the research → decision → issue traceable chain (an accepted recommendation must reach an ADR/BDR and then issues, in `living-docs`) — `living-docs skill research-artifacts --topic structure`.
