---
type: Issue
title: Initialize-era MCP clients are offered resource subscriptions the server cannot serve them
description: living-docs mcp advertises resources subscribe and listChanged on the initialize revision, where resources/subscribe is refused and no list change is sent.
status: closed
timestamp: 2026-11-27T05:50:10Z
---

## 0161. Initialize-era MCP clients are offered resource subscriptions the server cannot serve them

## Context

`living-docs mcp` advertises the resources capability with `subscribe` and `listChanged` in both protocol revisions, because `rmcp` 3.12 only acknowledges a `resourceSubscriptions` filter on `subscriptions/listen` when `subscribe` is advertised (rmcp `src/model.rs` capability intersection), and `get_info` cannot tell the revisions apart. A client on the earlier `initialize` revision that trusts the capability gets method-not-found on `resources/subscribe` and never receives an unsolicited `notifications/resources/list_changed`, since events flow only through `subscriptions/listen`. Found in review of issue 0156, slice S7; the README documents the limitation.

## Decision

- Serve initialize-era clients on the same event path: `resources/subscribe` and `resources/unsubscribe` register a URI for the session, and the server sends `notifications/resources/updated` and `notifications/resources/list_changed` to it (session, 2026-11-27). Not taken: withholding `subscribe` and `listChanged`, because rmcp 3.12 acknowledges a `resourceSubscriptions` listen only when `subscribe` is advertised and `get_info` cannot tell the revisions apart.

## Acceptance

- A client on the `initialize` revision either is not offered `subscribe` and `listChanged`, or `resources/subscribe` and list-change notifications work for it on the same event path as `subscriptions/listen`.

## Scope

In: `mcp/src/lib.rs`, `mcp/src/events.rs`. Out: the 2026-07-28 listen path.
