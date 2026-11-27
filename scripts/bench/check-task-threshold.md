# check task threshold

The record count at which `living-docs check` takes 2 s, Claude Code's default
task poll interval. `living-docs mcp` answers a `check` call as a task from this
many records in force, when the client declared the Tasks extension.

Produced by `scripts/bench/check-task-threshold.sh` with the release binary; each
time is the median of three `check` runs over a bundle that
`scripts/bench/gen-bundle.py` grew from `examples/linkly/docs`, after
`manage index` and `manage fmt`.

- host: AMD Ryzen 7 5700G with Radeon Graphics (14 CPUs visible), Linux
- binary: living-docs 0.34.1
- date: 2026-11-27

| per type | in-force records | median check (s) |
|---|---|---|
| 50 | 85 | 0.071 |
| 100 | 163 | 0.132 |
| 200 | 320 | 0.244 |
| 400 | 633 | 0.490 |
| 800 | 1260 | 0.959 |
| 1600 | 2513 | 1.927 |
| 3200 | 5020 | 3.823 |
| 2400 | 3766 | 2.872 |
| 2000 | 3140 | 2.393 |
| 1800 | 2826 | 2.175 |
| 1700 | 2670 | 2.050 |
| 1650 | 2591 | 1.962 |

threshold: 2670 in-force records (the smallest measured count whose median
reaches 2 s; 2591 stayed under it). The curve is linear, about 0.77 ms per
in-force record. Re-run the script when `check` gets faster or slower, and on
the CI runner when one is set up for it.
