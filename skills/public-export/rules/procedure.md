# public-export procedure

1. **Set visibility at the source.** Confirm each doc that should ship carries `visibility: public`
   (scaffold) or `visibility: showcase` (curated portfolio); everything else stays absent/private.
   `living-docs check docs` must be green (it validates the visibility domain).
2. **Build the allowlist bundle.** `living-docs export --visibility public,showcase <out>`
   materializes only the allowlisted docs into `<out>` — default-deny, so a private or
   absent-visibility doc never lands there.
3. **Gate.** `living-docs leak-gate <out>`. It exits non-zero on any private doc in the bundle, any
   published doc linking to a withheld doc, or any secret/PII pattern match. Fix every finding at the
   source (visibility, link, or secret), rebuild, and re-gate — never edit `<out>` by hand.
4. **Create the destination repo PRIVATE (first time only).** `gh repo create <owner>/<name>
   --private --description "<one-liner>"`.
5. **Clean-history publish (with the human).** In the still-private destination, create the curated
   release commit (orphan branch or history filter that keeps only `<out>`), review the diff, then
   `git push --force-with-lease`. Private keeps the real granular history; public shows curated
   release commits.
6. **Flip to public only after verifying clean while private.** Re-run `living-docs leak-gate`
   against the pushed tree, eyeball it, then `gh repo edit <owner>/<name> --visibility public`.
   Anything ever wrong is caught while still private.
7. **Record** the decision (what is published, what stays private) via `living-docs`.
