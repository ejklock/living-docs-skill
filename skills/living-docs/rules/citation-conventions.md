# Citation Conventions (ABNT NBR 6023)

Every doc that cites sources — **Research** above all, but also any **ADR**/**BDR** that rests on evidence — lists them under a `# References` heading (OKF §8). References follow the **ABNT NBR 6023 structure** and **always carry the link**. The connective **labels** follow the project doc language (default English) — see `rules/doc-language.md`.

## The two iron rules

1. **Always the link.** Every reference to anything with a URL (paper, blog, vendor doc, repo, standard) includes the URL and the access date. A citation without its link is not verifiable — it is hearsay. (For a print-only book with no online edition, the link is omitted; everything else carries it.)
2. **ABNT NBR 6023 structure, labels in the doc language.** Author surnames in CAPS, title in **bold**, an ISO access date — these are invariant. The connective labels localize to the pinned doc language: **English** (default) → `Available at: <URL>. Accessed on: 2026-06-15.`; **Portuguese** (NBR native) → `Disponível em: <URL>. Acesso em: 2026-06-15.`. Don't mix languages within a corpus — the format is always NBR; only the labels follow `rules/doc-language.md`. The examples below use the English (default) labels.

## Templates by source type

- **Book** — `SURNAME, First name. **Title**: subtitle. Edition. City: Publisher, year.`
  > FEATHERS, Michael. **Working Effectively with Legacy Code**. Upper Saddle River: Prentice Hall, 2004.

- **Paper / preprint (online)** — `SURNAME, First name et al. **Title**. Year. Available at: <URL>. Accessed on: YYYY-MM-DD.`
  > ALSHAHWAN, Nadia et al. **Automated Unit Test Improvement using Large Language Models at Meta**. 2024. Available at: https://arxiv.org/abs/2402.09171. Accessed on: 2026-06-15.

- **Blog post / online article** — `SURNAME, First name. **Article title**. Site name, year. Available at: <URL>. Accessed on: YYYY-MM-DD.`
  > GAUTHIER, Paul. **Separating code reasoning and editing**. Aider, 2024. Available at: https://aider.chat/2024/09/26/architect.html. Accessed on: 2026-06-15.

- **Vendor / institutional doc (no personal author)** — `ORGANIZATION. **Title**. Year. Available at: <URL>. Accessed on: YYYY-MM-DD.`
  > ANTHROPIC. **Building Effective Agents**. 2024. Available at: https://www.anthropic.com/research/building-effective-agents. Accessed on: 2026-06-15.

- **Source code / repository** — `AUTHOR/ORG. **Repository name**. Year. Available at: <URL>. Accessed on: YYYY-MM-DD.`

## Rules

1. **Alphabetical by first element** (author surname or organization) within `# References`.
2. **One entry per source.** If the same source is cited many times in the body, it appears once in `# References`.
3. **Access date is the day you verified it** — not the publication date. When a primary URL was unreachable (e.g. HTTP 403) and the claim was triangulated, say so in the entry (`(verified via secondary sources)` / `(exact URL not captured — unverified)`) rather than implying you read the original. Never fabricate a link.
4. **Append-only with the doc** — research is dated evidence (the living-docs maintenance invariant). When a source is superseded, annotate; do not silently rewrite a citation.
5. In-body, refer to a source by `(SURNAME, year)` or a short marker; the full entry lives once under `# References`.

Load `okf-knowledge-format` for the `# References` heading placement (§8); this file governs the *format of each entry*.
