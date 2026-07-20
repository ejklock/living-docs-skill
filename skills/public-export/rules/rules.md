# public-export hard rules

- **Allowlist, never denylist.** Only `visibility: public|showcase` docs export (default-deny). A
  denylist leaks the day you add a new private doc and forget it.
- **Visibility is declared at authoring time, never judged at publish.** Missing ⇒ private.
- **Never hand-edit the public repo.** It is regenerated. Hand edits become a second source that
  silently diverges.
- **The gate vetoes the push.** `living-docs leak-gate` must pass before publishing. It fails closed
  on a private doc present in the bundle, a published doc linking to a withheld doc, or a secret/PII
  pattern match. Fix every finding at the *source* (the doc's `visibility`, the link, or the
  secret), never by editing the built output.
- **Strip the narrative, keep the credit.** Remove internal decision narrative when curating a
  showcase doc; **keep external provenance/attribution**. Stripping research ≠ stripping credit.
- **A leaked secret is compromised — rotate it.** Force-push/orphan history removes a secret from
  `main` but not from forks/clones/caches/the SHA. History hygiene is the backstop; the gate +
  allowlist are the real control.
- **git push is human-gated.** The tool builds and gates; it never pushes. You review the diff, then
  push by hand.
- **The destination repo is created PRIVATE, always.** Create with `gh repo create <owner>/<name>
  --private`, push the built+gated bundle, verify once more while it is still private, and only then
  flip it public (`gh repo edit <owner>/<name> --visibility public`). Never create a public repo and
  push in one shot — the first commit is instantly world-readable and a leak cannot be recalled.
