---
type: ADR
title: In-memory store for minted links
description: Store links in a process-local map for the first prototype.
status: Superseded
supersedes:
superseded_by: 0002
tags: [storage]
timestamp: 2026-06-20T00:00:00Z
---

# 0001. In-memory store for minted links

<!-- This record is history. It is NOT edited — it is superseded by 0002. Kept so a
     future reader sees why the in-memory choice was made and why it was abandoned. -->

## Context

The first prototype needed *something* to hold `code → target_url`. The fastest path to a
working redirect was a process-local hash map: zero dependencies, zero schema.

## Decision

We will store minted links in an in-process map keyed by `code`.

## Consequences

**Easier / gained:**
- Nothing to install or migrate; the redirect path works on day one.

**Harder / accepted trade-offs:**
- Links vanish on restart — violates the constitution's "links are permanent"
  non-negotiable. Acceptable only for a throwaway prototype.

**Follow-ups:**
- Replace with durable storage before any real use → [ADR 0002](0002-sqlite-store.md).
