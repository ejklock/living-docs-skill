---
type: Architecture View
title: System context
description: Linkly's components and the outside world.
timestamp: 2026-06-20T00:00:00Z
---

# System context

What talks to what. Consult this when adding an endpoint or an external dependency.

```mermaid
flowchart LR
    U[Caller] -->|POST /shorten| API[Linkly API]
    U -->|GET /code| API
    API --> S[(SQLite store)]
```

The API is the only component; storage is the single-file SQLite database chosen in
[ADR 0002](/adr/0002-sqlite-store.md).
