---
type: Issue
title: size-targets guide exempts architecture docs, but the registry gives the architecture view a size target
description: The size-targets guide and the registry disagree on architecture views, so check emits SIZE advisories the guide says cannot exist.
status: open
timestamp: 2026-10-10T19:16:42Z
---

## 0056. size-targets guide exempts architecture docs, but the registry gives the architecture view a size target

Found adopting 0.21.0 in a downstream bundle (relent). `skills/living-docs/rules/size-targets.md` lists the exempt doc kinds as "research (long-form dated evidence), the constitution, context and architecture docs, and the reserved `index.md`/`log.md` listings". The registry row for the architecture view in `living-docs-core/src/doc_type.rs` sets `body_size: BodySize::Targeted`, while the constitution and research rows are `BodySize::Exempt`.

Reproduction: `living-docs new view "Rings" --kind component`, fill the body with more than 120 lines, run `living-docs check docs`. A `SIZE` advisory names the view, contradicting the guide.

One of the two is wrong. A view is one concern per file by design (`new view` creates one file per concern), so a targeted size nudges authors to split a large view, which is the intended pressure.

### Scope

Align guide and registry on one answer. Kept: the registry row, if the decision is that views are targeted; the guide then drops "architecture" from the exempt list and names views as targeted. If the decision is that views are exempt, the registry row changes to `Exempt` and the guide stays. A cheap choice, recorded in this issue's Decision section when made.

### Acceptance

- The guide and the registry agree on whether a view body over 120 lines draws a `SIZE` advisory.
- A test over the registry row and the guide text fails if they diverge, or the existing size tests cover the chosen behavior for `Architecture View`.
- `living-docs check docs` stays OK.
