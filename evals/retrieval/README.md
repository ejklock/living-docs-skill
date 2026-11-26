# Retrieval fixture

Forty plant pairs and forty controls for the offline retrieval measure of the scale eval, plus the checks that keep the fixture honest.

Run it from the repository root:

```sh
make build            # the parity test runs the binary
make test-retrieval   # the fixture lint, then the pytest suite
```

`uv` runs everything from `pyproject.toml` and `uv.lock` with `pytest` pinned; nothing is installed beyond it. `LIVING_DOCS_BIN` names the binary (default `target/release/living-docs`); a missing binary fails the parity test with a message naming `make build`.

## Fixture

Every plant and control is a software engineering or product decision, the domain of the filler ADRs, and has its own topic, none of them one of the ten concerns in `evals/scale/concerns.tsv`.

`fixture/plants.jsonl`, one object per line:

- `id` `p01` to `p40`; `type` `t1` (the query contradicts the plant) or `t2` (the plant decides two clauses and the query reverses one), 20 each; `position` `early` or `late` (ADR 3 or size minus 2), 20 each and 10 each within a type.
- `query`: `title`, `description` and `body` as an author would pass them to `write new adr`; the body holds Context, Decision and Consequences in the shape `scale_adr` writes.
- `plant`: `title`, `description`, `context` and `decision`, the fields `manifest.awk` takes; `decision` is a lowercase verb phrase that follows "We will ".

`fixture/controls.jsonl`: `id` `c01` to `c40`, `split` (`calibration` for `c01` to `c20`, `test` for `c21` to `c40`) and `query`.

## Lint

`python lint_fixture.py [DIR]` (default `fixture/`) checks the shape rules, then builds the seed 153 bundle of 60 and of 240 ADRs with each plant at its position and fails a plant whose query shares any word that is rare in that bundle. Words and rarity follow the finder of `living-docs write new adr`: lowercased ASCII letter runs of four or more letters, minus its stop list; rare means in at most max(2, 3% of the in-force ADRs) records, the new record not counted.

The rule is deliberately strict: a plant's query fails when it shares a rare word with any in-force record, the plant included, not only with its own plant.

Exit 0 when clean; 1 with one line per failure (`p07: shares rare word 'x' in the bundle of 60`, or the broken shape rule); 2 for a missing file or an unreadable line, naming the file and line.

## Modules

- `rare_words.py`: the finder, ported from `living-docs-core/src/commands/effective/candidates.rs`. A parity test holds it equal to the binary.
- `bundle.py`: `bundle_records` composes the in-force ADRs of a seed bundle from the scale manifest; `scale_setup_records` builds the same bundle with `scale_setup` and the real CLI, and a test holds the two equal.
