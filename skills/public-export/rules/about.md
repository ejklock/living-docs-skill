# public-export — mental model, composition, provenance

## Why this skill exists

Your written method was never the moat — the practiced judgment, the accumulated lessons, and the
decision trail are. So you *can* publish the scaffold and a curated showcase of your reasoning (a
strong credibility signal) while keeping the accumulation private. This skill is the disciplined
way to do that without leaking.

## Mental model: the public repo is a build artifact of the private one

You never hand-edit the public repo; you *generate* it. One-way flow: private (source of truth) →
build → gate → clean-history publish. In this repo the deterministic parts are the `living-docs`
tool itself — `export --visibility` (the allowlist build) and `leak-gate` (the fail-closed
backstop), decided in
[ADR 0010](/adr/0010-public-export-is-a-deterministic-allowlist-build-with-a-leak-gate-publish-is-a-human-gated-procedure.md).
This skill owns the **judgment** the tool cannot: what belongs in the curated showcase, and the
human review of the diff before any push.

The split is deliberate (ADR 0001 + ADR 0010): the tool is deterministic and never runs
destructive git; the irreversible clean-history publish is a human step this skill documents, never
a tool subcommand.

## Composes with

- **`living-docs`** — owns document authoring, the `visibility` field (ADR 0009), and records the
  publish decision; this skill does not author docs.
- **`okf-knowledge-format`** — the export respects OKF; this skill removes *internal* narrative from
  showcased docs, not the format.

## Provenance — instrumentalization, not invention

The "public repo as generated artifact / one-way source→public" pattern is standard docs-as-code
practice; allowlist-over-denylist is the least-privilege/default-deny security principle applied to
publishing; orphan-branch history hygiene and "rotate a leaked secret" are established git-secrets
guidance. This skill is the composition and the calibrated leak gate, not a new method.
