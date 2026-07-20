# OKF conformance (§9)

These five hard rules define OKF conformance. They summarize the vendored spec at
`reference/SPEC.md` §9 — when a detail is ambiguous, the vendored spec wins. If
`scripts/update-spec.sh` reports a changed spec, reconcile these rules against the new §9.

1. **Every non-reserved `.md` file has a parseable YAML frontmatter block** delimited by `---` on
   its own line at the top and a closing `---`.
2. **Every frontmatter block has a non-empty `type` field.** `type` is the only required field.
   Everything else is optional.
3. **Reserved filenames are reserved.** `index.md` (directory listing, §6) and `log.md` (update
   history, §7) must follow their defined structure and must **not** be used for concept documents.
4. **`index.md` carries no frontmatter** — the sole exception is the bundle-root `index.md`, which
   MAY declare `okf_version: "0.1"` (§11).
5. **Consume permissively.** Never reject a bundle for missing optional fields, unknown `type`
   values, unknown extra keys, broken cross-links, or a missing `index.md`. OKF stays useful as
   bundles grow and get partially agent-generated.
