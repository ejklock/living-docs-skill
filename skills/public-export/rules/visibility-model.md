# How private stays separate from public (the tool never judges at publish)

Do **not** trust an LLM to decide, at export time, what is private — that is LLM-as-judge on a
security boundary, the exact failure this system forbids (ADR 0009). The judgment is made **once, by
the human, at authoring time**, recorded as **data** in the living-docs/OKF frontmatter, and
enforced deterministically forever:

- Every OKF concept doc carries **`visibility: private | public | showcase`** in frontmatter, set
  when the doc is written and confirmed by you.
- **Missing or unknown `visibility` ⇒ private** (default-deny). Omission can never publish something
  by accident. `living-docs check` validates the domain (a typo fails), so the rule is an
  instrument, not a vibe.
- The export is driven by that declaration: `living-docs export --visibility public,showcase <out>`
  materializes only allowlisted docs, so **no LLM judgment sits in the publish path**.
- The **leak gate is the backstop**: even a mismarked-public doc is caught if it links to a withheld
  doc, or carries a secret.

The AI's only role is at *authoring*: when it writes an ADR/PRD/BDR/research note, it proposes a
`visibility` (defaulting to private) for you to confirm — never to decide what ships at publish.
