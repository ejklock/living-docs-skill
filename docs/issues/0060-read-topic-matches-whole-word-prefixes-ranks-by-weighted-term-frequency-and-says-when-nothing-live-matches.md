---
type: Issue
title: read --topic matches whole-word prefixes, ranks by weighted term frequency and says when nothing live matches
description: A topic term matches a word that starts with it, every term must match, records rank by title x3, description x2, body x1, and an empty result prints a stderr hint naming the withheld count and a broader search.
status: closed
timestamp: 2026-10-19T13:13:52Z
---

## 0060. read --topic matches whole-word prefixes, ranks by weighted term frequency and says when nothing live matches

Agents in the owner's transcripts used `read --topic` 225 times against 3,391 direct file reads of `docs/`. The matcher is a case-sensitive-folded substring over title, description and body with no ranking, so `--topic heading` returns six ADRs in reading order and `--topic tailwind` returns `{"withheld":8,"records":[]}` with no word about what to try next; half the recorded topics were code terms (`zod`, `luxon`, `jest`). [ADR 0056](/adr/0056-effective-topic-ranks-by-the-fts5-read-model-when-a-fresh-projection-exists-falling-back-to-a-deterministic-relevance-rank.md) decided a deterministic weighted rank as the fallback; the read model it preferred was removed by ADR 0059 and the fallback was never written.

### Scope

- The topic splits on whitespace into terms; a term matches a word (a run of alphanumerics, accent-folded, lowercased) that starts with it.
- A record matches when every term matches somewhere in its title, description or body.
- Matching records rank by weighted hits: title times 3, description times 2, body times 1; ties keep the reading order.
- When nothing live matches, stderr prints one line naming the topic, the count of retired records withheld and a broader search to try; stdout keeps its shape and the exit code is 0.
- JSON and plain shapes are unchanged.

### Decision

- Terms combine with AND; option not taken: OR, which turns a two-word topic into a wider net than one word.
- Prefix match, not substring: `cach` finds `caching`, `ache` no longer finds `cache`; option not taken: stemming, which needs a dictionary.
- Accent folding reuses the transliteration `slugify` gains in [issue 0057](/issues/0057-new-seeds-owner-from-git-config-when-owner-is-absent-and-slugs-transliterate-accented-letters.md) so `ação` and `acao` meet; the two issues share that function.

### Acceptance

- `--topic cache` lists a record titled "Caching strategy" above one that mentions caching only in its body; `--topic ache` lists nothing.
- `--topic "owner style"` lists only records where both terms match.
- `--topic ação` and `--topic acao` list the same records.
- With no live match, stderr carries one line naming the topic and the withheld count, stdout is `{"withheld":N,"records":[]}` under `--json`, and the exit code is 0.
- The full suite and `make check` pass.

### Outcome

Delivered, uncommitted, in the working tree: `read --topic` splits the topic into terms, folds accents and case, and keeps a live record only when every term starts a word in its title, description or body (`living-docs-core/src/commands/effective/topic.rs`); matches rank by weighted hits (title 3, description 2, body 1) with the reading order as the tie-break, so no topic leaves the order unchanged; when nothing live matches, a stderr line names the topic, the retired count withheld and a broader search, silenced by `--quiet`; stdout shapes are unchanged. The `semantic-index` skill topic states the new rule. Proof: `cargo test -p living-docs-core effective` (16 passed), `cargo test -p living-docs --test read` (6 passed), `cargo test --workspace` green (761), `se-gates check` pass.
