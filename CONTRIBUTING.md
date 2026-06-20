# Contributing to Living Docs

Thanks for your interest! Living Docs is a small, opinionated **agent skill** —
plain markdown that teaches an AI coding agent how to keep documentation a living
system. Contributions that sharpen the discipline, fix drift, or improve clarity
are very welcome.

This project **dogfoods its own rules**: changes to it follow the same five
invariants it preaches.

---

## Ways to contribute

- **Fix a bug or an inconsistency** in a `SKILL.md`, a `rules/` convention, or a
  `templates/` starter file.
- **Improve clarity** — tighten wording, fix a broken cross-link, correct a
  diagram that no longer matches the prose.
- **Add a harness install path** (a new tool's instruction location) in
  `install.sh`, the `Makefile`, and the README's Installation section.
- **Strengthen attribution** — if a practice is credited imprecisely or a source
  is missing, open a PR against `ATTRIBUTION.md` / `references/`. We never want
  to leave anyone uncredited.

For anything larger than a typo, **open an issue first** so we can align on the
change before you invest time.

---

## Repository layout

```
skills/
  living-docs/             the skill: SKILL.md + rules/ + templates/
  okf-knowledge-format/    the file format (OKF spec vendored verbatim)
  research-artifacts/      the research-note format
references/
  prior-art-landscape.md   sourced prior-art analysis (the attribution backbone)
assets/                    README images (hand-authored SVG, no build step)
install.sh                 multi-harness installer
Makefile                   convenience wrapper around install.sh
```

---

## The rules this repo holds itself to

1. **Docs-first / one home per fact.** Each concept lives in exactly one file.
   Cross-reference; don't duplicate prose between `SKILL.md`, `rules/`, and the
   README.
2. **Indexed or it doesn't exist.** A new `rules/` or `templates/` file must be
   referenced from the relevant `SKILL.md`.
3. **Supersede, never silently rewrite.** When you change a load-bearing
   convention, explain *why* in the PR description — don't just overwrite it.
4. **No structural change without its doc.** New behavior in `install.sh` →
   update the README **and** the `Makefile`. New skill capability → update its
   `SKILL.md`.
5. **Credit, don't claim.** This project instrumentalizes established practices.
   Any new technique must cite its originator in `ATTRIBUTION.md`.

---

## Do not hand-edit the vendored OKF spec

`skills/okf-knowledge-format/reference/SPEC.md` is a **verbatim copy** of the
upstream Open Knowledge Format spec. Never edit it by hand. To refresh it:

```bash
cd skills/okf-knowledge-format
./scripts/update-spec.sh        # re-pulls upstream, rewrites SPEC.source.md provenance
```

Then review the diff and update the `SKILL.md` rules only if conformance changed.

---

## Validating your change

There is no build step — everything is markdown and shell. Before opening a PR:

```bash
make check        # bash -n (install.sh + lint-docs.sh), dry-run every harness, lint the example corpus
make lint-docs    # just the docs-invariant check on examples/linkly/docs
```

`make check` runs the skill's checker (`skills/living-docs/scripts/lint-docs.sh`) against
[`examples/linkly/docs`](examples/) and verifies version sync — the project dogfoods its own
invariants. If you change a `templates/` or `rules/` shape, update the example to match and keep
it lint-clean.

If you touched `install.sh`, also try a real run into a throwaway directory:

```bash
./install.sh claude --dir /tmp/ld-test
```

Please also check that any link you added resolves and that any Mermaid diagram
you changed still renders on GitHub.

---

## Versioning & releasing

The project is versioned with **semver** and released automatically by GitHub Actions
(`.github/workflows/release.yml`).

The version is declared in `VERSION` **and** in each `skills/*/SKILL.md` frontmatter
(`version:`). That duplication is deliberate — consumers read the version from the skill they
load — and it is **gated**, not trusted: `scripts/check-version.sh` (run by `make check` and CI)
fails if the copies disagree. To cut a release:

```bash
# 1. bump the version everywhere, in one commit
#    edit VERSION and the three skills/*/SKILL.md `version:` lines to X.Y.Z
./scripts/check-version.sh X.Y.Z        # must print "Version OK"
make check                              # full local gate

git commit -am "chore: release vX.Y.Z"
git tag vX.Y.Z
git push origin main --tags
```

Pushing the `vX.Y.Z` tag triggers the release workflow: it re-verifies the tag matches the
declared version, lints, packages the skills into `living-docs-skill-vX.Y.Z.zip` (+ a SHA-256),
and publishes a GitHub Release with auto-generated notes. A tag that disagrees with `VERSION`
fails the release rather than shipping a mislabeled bundle.

## Commit & PR conventions

- Use **Conventional Commits**: `type: lowercase description`
  (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`). Most changes here are
  `docs:`.
- No AI attribution trailers in commit messages.
- Keep PRs focused — one concern per PR. Fill in *what* changed and *why*.
- Be kind and direct in review. Suggestions that are preferences should say so
  ("nit:" / "optional:").

---

## License of contributions

By contributing, you agree your contribution is licensed under the repository's
[MIT License](LICENSE). Vendored third-party content keeps its own upstream
license — see [`ATTRIBUTION.md`](ATTRIBUTION.md).
