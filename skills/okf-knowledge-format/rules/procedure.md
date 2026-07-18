# OKF procedure

## Procedure

### Author a concept
1. Copy `templates/concept.md`. Set a non-empty, descriptive `type` (hard rule 2).
2. Fill recommended fields (`title`, `description`, and `resource` if it maps to a real asset). Add `tags`/`timestamp` if useful.
3. Write the body in structural markdown; use `# Schema`/`# Examples` where they fit. Cross-link with bundle-relative `/…` paths.
4. Add a `# References` section for any externally-sourced claim.

### Maintain a directory
1. Keep `index.md` listing the directory's concepts, descriptions mirroring each concept's frontmatter `description` (`templates/index.md`).
2. If the scope tracks history, append a dated entry to `log.md` (newest first, ISO 8601 dates — `templates/log.md`).
3. Declare `okf_version: "0.1"` in the bundle-root `index.md` frontmatter only.

### Check conformance
Walk the four hard rules: every non-reserved `.md` has parseable frontmatter; every block has non-empty `type`; reserved files follow §6/§7; root `index.md` is the only `index.md` with frontmatter.

---

## Keeping the spec current

The spec is **vendored** (verbatim) at `reference/SPEC.md` with provenance in `reference/SPEC.source.md`. To pull the latest from GitHub:

```bash
skills/okf-knowledge-format/scripts/update-spec.sh          # default ref: main
skills/okf-knowledge-format/scripts/update-spec.sh v0.2     # a tag/branch/commit
```

The script overwrites `reference/SPEC.md`, rewrites `reference/SPEC.source.md` (URL, ref, retrieval time, sha256), and reports whether the content changed. **If it changed, review the diff and reconcile the Hard rules / field tables above** before committing — this SKILL.md must not drift from the vendored spec. The vendored copy is the offline source of truth; the script is the only sanctioned way to update it.
