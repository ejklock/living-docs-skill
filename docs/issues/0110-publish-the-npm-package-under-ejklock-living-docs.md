---
type: Issue
title: Publish the npm package under @ejklock/living-docs
description: Rename the npm wrapper to the owner's scope, update the install lines, and publish it from the release workflow through npm trusted publishing with no token.
status: open
timestamp: 2026-11-09T23:40:11Z
---

## 0110. Publish the npm package under @ejklock/living-docs

## Publish the npm package under @ejklock/living-docs

Implements [ADR 0077](/adr/0077-the-npm-package-is-published-under-the-scoped-name-ejklock-living-docs-because-the-unscoped-name-belongs-to-another-author.md). Answers GitHub issue #84: the unscoped `living-docs` name on npm belongs to another author, so the package from [issue 0061](/issues/0061-npm-package-living-docs-wraps-the-release-binary-and-the-release-workflow-publishes-it.md) ships under the owner's scope.

### Scope

- `npm/package.json`: `name` becomes `@ejklock/living-docs`; `bin` stays `living-docs`.
- `npm/README.md` and `README.md`: the install and run lines name the scope (`npm install -g @ejklock/living-docs`, `npx @ejklock/living-docs ...`).
- `scripts/tests/npm/run.sh`: the fixture's installed package path follows the scoped install location `node_modules/@ejklock/living-docs`.
- `.github/workflows/release.yml` job `publish-npm`: stages each version through trusted publishing with `npm stage publish` (npm 11.15.0 or later, Node 22.14 or later, `id-token: write`), with no `NPM_TOKEN` and no secret gate; the owner approves each staged version with 2FA.
- The two unpinned actions in `publish-npm` (`actions/checkout`, `actions/setup-node`) are pinned by full commit SHA, the rule every other job of the workflow already follows.
- Out of scope: the shim, the postinstall, and a guard that skips an already-published version.

### Decision

- The `publish-npm` job authenticates through npm trusted publishing (OIDC from GitHub Actions); no npm token is stored in the repository. Not taken: a granular token with 2FA bypass as the `NPM_TOKEN` secret, which npm's token page itself flags as a security risk for CI and points to trusted publishing instead (owner's choice, 2026-11-09).
- The first version of `@ejklock/living-docs` is published by the owner from a workstation with 2FA, because npm configures a trusted publisher from the settings of an existing package; the trusted publisher (repository `ejklock/living-docs-skill`, workflow `release.yml`) is configured right after.
- The job no longer skips when a secret is absent: a release whose npm publish fails shows a failed job instead of a green skip.
- The job stages each version with `npm stage publish` and the trusted publisher leaves "Allow npm publish" unchecked, so no version goes live until the owner approves it with 2FA (`npm stage approve <stage-id>` or the Staged Packages tab). Not taken: direct `npm publish` from CI, which npm's trusted publisher form marks as not recommended because anyone able to run `release.yml` would publish with no human 2FA (owner's choice, 2026-11-09).

### Acceptance

- `npm pack --dry-run` in `npm/` names `@ejklock/living-docs` and ships the same files as before.
- No unscoped install or run line (`npx living-docs`, `npm install -g living-docs`) remains outside `docs/`, whose records only mention the names.
- `release.yml` references no `NPM_TOKEN` and its `publish-npm` job stages with `npm stage publish` on npm 11.15.0 or later with `id-token: write`.
- `make check` passes.
- Owner actions: publish the first version from a workstation, configure the trusted publisher on npmjs.com, answer GitHub issue #84.

### Plan

- S1: the package packs as `@ejklock/living-docs` and every install line names the scope | files: npm/package.json, npm/README.md, README.md, scripts/tests/npm/run.sh | after: none
- S2: `publish-npm` publishes through trusted publishing with no npm token and no secret gate, its actions pinned by SHA | files: .github/workflows/release.yml | after: none
