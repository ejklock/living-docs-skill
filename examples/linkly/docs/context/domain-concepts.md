---
type: Context
title: Domain concepts
description: Core domain entities and rules for Linkly.
tags: [domain, vocabulary]
timestamp: 2026-06-20T00:00:00Z
---

# Domain concepts

The vocabulary the code, docs, and reviews all use for Linkly's domain. One home per
concept — other docs link here rather than redefine.

## Link

A `LINK` binds a short `code` to a `target_url`. It is **immutable**: once minted, a code
always resolves to the same target. Defined in the [constitution](../constitution.md) data
model.

## Code

The short, URL-safe identifier minted for a link. Globally unique. Appears as the path
segment in `GET /{code}`.

## Mint

The act of creating a new `LINK` for a submitted URL. See
[issue 0001](../issues/0001-implement-shorten-endpoint.md), acceptance.

## Resolve

The act of turning a `code` back into its `target_url` for redirect. See
[issue 0001](../issues/0001-implement-shorten-endpoint.md), acceptance.
