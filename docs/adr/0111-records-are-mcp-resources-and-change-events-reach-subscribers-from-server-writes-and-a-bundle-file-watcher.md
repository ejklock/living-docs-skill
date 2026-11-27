---
type: ADR
title: Records are MCP resources and change events reach subscribers from server writes and a bundle file watcher
description: Every record is a resource; subscriptions/listen delivers list_changed and updated for the server's writes and for outside edits a coalescing file watcher sees.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-27T02:00:23Z
---

# 0111. Records are MCP resources and change events reach subscribers from server writes and a bundle file watcher

## Context

ADR 0078 left records out of MCP resources and named `subscriptions/listen` as a follow-up for a client that must learn mid-session that a record changed or was superseded. In MCP 2026-07-28, `subscriptions/listen` replaces `resources/subscribe`: the client asks for `resourcesListChanged` and for `resourceSubscriptions` on URIs, the server acknowledges the subset it honours, then sends `notifications/resources/list_changed` and `notifications/resources/updated` on the same stdio channel, tagged with the subscription id [1]. `rmcp` 3.x implements the listen handler and the notification helpers [2]. A record also changes outside the server: the author edits a body by hand, `git checkout` switches branches, another session runs the CLI. Claude Code documents refetching resources on `list_changed`; its docs do not mention `resources/updated` [3].

## Decision

We will expose every record as a resource and emit change events through `subscriptions/listen`, for writes made by the server and for edits a file watcher sees in the bundle.

- Each record is a resource whose URI names it as `TYPE/NNNN` does in the CLI, with its markdown as the content and its title and description as metadata. Architecture views, domain records and the constitution follow the same rule by their own reference form.
- The server honours `resourcesListChanged` and `resourceSubscriptions`. A record created, renumbered or deleted emits `list_changed`; a record whose file content changes emits `updated` for its URI to a subscriber of it.
- A file watcher over the bundle root feeds the same events as the server's own writes, so a hand edit or a branch switch reaches the client. Bursts are coalesced so one checkout yields one `list_changed` and one `updated` per changed record.
- Without a subscriber the watcher does not run.

Rejected alternatives:

- **Events for the server's own writes only.** Needs no new dependency, but a hand edit or a branch switch leaves a client reading a stale record, the case the follow-up was opened for.
- **Polling the bundle on a timer.** Costs work when nothing changes and delays every event by the period.

## Consequences

**Easier / gained:**
- A client reads records as resources and learns when one changes, wherever the change came from.

**Harder / accepted trade-offs:**
- A file-watching dependency enters the `mcp` crate, with a background task per subscribed session.
- Watch behaviour differs by platform (inotify, FSEvents); the tests prove the coalesced events, not the platform's timing.
- Claude Code may ignore `resources/updated` today.

**Follow-ups:**
- Implemented as a slice of issue 0156.

## Verification

**Implementation impact:** the `mcp` crate (resources, listen handler, watcher), `mcp/Cargo.toml`, `mcp/tests/stdio_server/`.

**Verification criteria:**
- Over stdio, `resources/list` lists every record in force with its reference URI, and `resources/read` returns its markdown.
- A `write` `new` emits one `list_changed` to a listener; a `write` `set` emits `updated` for that record to its subscriber only.
- A file rewritten outside the server emits `updated`; a burst of writes to one file within the coalescing window emits one event; no event reaches a client that did not ask for that type.

# References

[1] [MCP 2026-07-28 subscriptions](https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/subscriptions)

[2] [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk)

[3] [Claude Code MCP](https://code.claude.com/docs/en/mcp)
