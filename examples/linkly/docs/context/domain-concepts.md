---
type: Context
title: "Domain concepts"
description: Core domain entities and rules for Linkly.
tags: [domain, vocabulary]
timestamp: 2026-06-20T00:00:00Z
---

# Domain concepts

The vocabulary the code, docs, and reviews all use for Linkly's domain. One home per
concept — other docs link here rather than redefine.

## Link

A `LINK` binds a short `code` to a `target_url`. It is **immutable**: once minted, a code
always resolves to the same target. Defined in the [constitution](/constitution.md) data
model.

## Code

The short, URL-safe identifier minted for a link. Globally unique. Appears as the path
segment in `GET /{code}`.

## Mint

The act of creating a new `LINK` for a submitted URL. See
[BDR 0001](/bdr/0001-shorten-and-redirect.md), Scenario 1.

## Resolve

The act of turning a `code` back into its `target_url` for redirect. See
[BDR 0001](/bdr/0001-shorten-and-redirect.md), Scenario 2.
