# Doc Size Targets

**Aim for ~100 body lines; the checker advises at 120.** The target is uniform across
decision and execution records — ADR, BDR, PRD, issue. Exempt: research (long-form dated
evidence), the constitution, context and architecture docs, and the reserved
`index.md`/`log.md` listings.

Body lines are what follows the closing frontmatter fence; the frontmatter block is
never counted.

## Why a target at all

Two costs grow linearly with prose length, and both are paid on every doc:

1. **Authoring cost.** When the author is an agent, every body line is output tokens.
   Terse records are the single largest authoring saving available — larger than any
   cheaper-model scheme, because the mechanical half is already free (the CLI) and the
   judgment half cannot be delegated.
2. **Reading and compaction cost.** Long docs are the first casualties of context
   compaction, and every reader — human or agent — pays the length again on every
   load. A record that fits in ~100 lines survives both.

## The two standing rules

1. **Advisory, never a gate.** `living-docs check` prints a `SIZE` note for an
   over-target body and always leaves the exit code untouched. Judgment prose is never
   truncated to satisfy a number.
2. **The target never trumps the rationale.** A load-bearing why stays, whatever the
   line count. Trim connective prose, restated cross-links, and content another doc
   already owns (one home per fact) — never the decision's substance.
