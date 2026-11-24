---
type: ADR
title: Write new adr lists in-force ADRs that share rare words with the new one
description: write new adr lists up to three in-force ADRs sharing two or more rare words with the new title, description and body, beside the fixed question, without judging a conflict.
owner: Evaldo Klock
status: Proposed
timestamp: 2026-11-24T09:52:55Z
---

# 0106. Write new adr lists in-force ADRs that share rare words with the new one

## Context

[ADR 0105](0105-write-new-adr-asks-whether-the-decision-contradicts-a-record-in-force.md) added a fixed question to `write new adr`. Issue 0153 measured it: Sonnet, unasked, caught 9 of 12 paraphrased conflicts with the question, against 2 of 12 with neither variant and 6 of 12 with the empty-search note alone; the rule fixed before running needs 11 of 12. A design review found that the words of the new decision's body, not its title, overlap the paraphrased record it reverses (9 of the 10 earlier misses shared at least two rare words with the plant), and that over this repository's own ADRs such a finder prints candidates on most writes. A duplicate pass of this kind was cut once ([ADR 0057](0057-refocus-living-docs-on-a-decision-log-delete-bdr-cut-peripheral-verbs-and-check-advisories-and-consolidate-the-rule-corpus.md)); this one differs in that it lists, never judges, and runs only on a new ADR.

## Decision

We will make `write new adr` list, beside the question of ADR 0105, the in-force ADRs that share rare words with the new one.

- The query is the new record's title, description and body when given (`--body`, `--description`). Words are lowercased ASCII runs of letters of length 4 or more, minus a fixed English stop list; no stemmer. A word is rare when it occurs in at most max(2, 3%) of the in-force ADRs.
- A candidate is an in-force ADR, other than the new one, sharing at least 2 rare words with the query. At most 3 are listed, ordered by the count of shared rare words (descending), then by record number; only integers decide the order, so the output is byte-stable.
- JSON adds `candidates`: a list of `{type, number, title, shared}` with the shared words sorted; text mode prints one stderr line per candidate after the question, saying the record shares words with the new one and naming them. The MCP `write` tool returns the same field. With no candidate, nothing is added: the tool never says there is none.
- The line says "shares words", never "conflicts": which decision governs stays the author's call ([ADR 0001](0001-living-docs-cli.md)). Exit codes do not change; only the ADR type lists.

Settled at implementation (2026-11-24): the rarity cut is floor(3% of the in-force ADRs), at least 2, counted over the in-force ADRs other than the new record, as document frequency; each in-force ADR is read as its title, description and whole body; the stop list is these 100 words: about, after, again, against, also, always, among, because, been, before, being, between, both, cannot, could, does, done, down, during, each, either, else, even, ever, every, from, further, have, having, here, hers, herself, himself, into, itself, just, keep, like, made, make, many, more, most, much, must, never, none, once, only, other, others, otherwise, ours, over, same, shall, should, since, some, such, than, that, their, theirs, them, then, there, these, they, this, those, though, through, thus, under, until, upon, used, uses, using, very, want, wants, were, what, when, whenever, where, whether, which, while, whom, whose, will, with, within, without, would, your, yours.

The word length, stop list, rarity cut and threshold are part of this contract; changing them needs a later ADR. V3 ships only if issue 0153's remeasurement under its fixed rule keeps it.

## Consequences

- An agent writing a decision sees the records most likely to govern the same topic even when the words differ from its search terms, without asking.
- On a real bundle the list appears on most writes; it is advice, and an agent may learn to skip it, which the remeasurement and a count over real bundles will show.
- One more full read of the in-force ADRs per `write new adr`, shared with the count of ADR 0105.
- English-only: a bundle in another language gets few rare-word matches and so few candidates.
