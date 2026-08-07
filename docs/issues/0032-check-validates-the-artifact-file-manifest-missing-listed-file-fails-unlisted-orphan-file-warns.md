---
type: Issue
title: "check validates the artifact file manifest: missing listed file fails, unlisted orphan file warns"
description: "Teach check to validate an artifact's files manifest: fail on a missing listed file, warn on an unlisted orphan file."
status: open
timestamp: 2026-08-07T14:38:11Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## check validates the artifact file manifest

Implements [ADR 0034](/adr/0034-artifacts-are-directory-bundle-records-a-canonical-readme-plus-a-validated-file-manifest-on-a-new-bundle-identity.md). Second slice: make the `files:` manifest a gate signal, giving [issue 0024](/issues/0024-doc-code-pairing-for-living-docs-commit-trailers-covers-based-drift-detection-and-executable-acceptance.md) its first concrete drift instrument.

After issue 0031 an artifact bundle exists with a `files:` list, but nothing checks it. This slice teaches `check` to validate the manifest against the bundle directory.

### Scope

Included: a `Bundle` arm in `check` that, for each artifact record, reads the `README.md` `files:` list and (1) FAILS when a listed file is absent from the bundle directory, (2) WARNS when the directory holds a file — other than `README.md` — that the manifest does not list. The arm learns the variant from the registry, so a second `Bundle`-identity row needs no edit inside `check`.

Explicitly kept: the existing invariant and size checks are unchanged; the manifest check is additive.

Explicitly out: deriving the list from disk (the manifest stays authored, per ADR 0034), and any index/search behavior (issue 0033).

### Acceptance

- An artifact whose `files:` names a file present in its directory passes `check`; naming an absent file fails `check` with a message pointing at the record and the missing file.
- A file added to the bundle directory but not listed in `files:` produces a warning (not a failure) naming the orphan.
- Fitness function: a test adds a second `Bundle`-identity row to a fixture registry and asserts the manifest check applies to it with no change inside `check` — the registry-driven shape from issue 0018.
- `living-docs check docs` and `living-docs check examples/linkly/docs` stay green.

### Plan

Add the manifest reader and the `Bundle` arm in `check`, keyed off the registry identity. Cover the fail, warn, and pass paths with fixture bundles; pin the registry-driven fitness function.
