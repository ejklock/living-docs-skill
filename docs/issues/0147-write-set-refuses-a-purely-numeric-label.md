---
type: Issue
title: write set refuses a purely numeric label
description: Decide whether write set quotes a numeric label or keeps refusing it, and prove the choice
status: closed
timestamp: 2026-11-16T20:26:45Z
---

## 0147. write set refuses a purely numeric label

### Scope

`living-docs write set <issue> labels 12` (and `"x, 12"`) exits 2 because the strict flow list leaves integers plain for `blocked_by`, where they are issue numbers, and a label `12` would read back as a number. The previous release refused it the same way. Found by the review of issue 0135.

### Decision

- Quote integers in `labels` only, so `labels 12` writes `labels: ["12"]` and reads back the string; `blocked_by` keeps integers plain as issue numbers. The previous release refused numeric labels, so no existing record changes bytes (owner, 2026-11-16). Not taken: keep refusing and document it in the issue-workflow rule.

### Acceptance

- `write set <issue> labels 12` either writes `labels: ["12"]` and reads back the string, or the rule documents the refusal; a test proves the chosen behavior; existing records keep their bytes.

### Outcome

- `write set` writes labels through `flow_strings`, which quotes integers, so `labels 12` writes `labels: ["12"]` and reads back the string; every label the previous release accepted keeps its bytes (33 inputs compared) and `blocked_by` is unchanged. Proof: `cli/tests/set_labels_numeric.rs`, `record/tests/quoting.rs`.
