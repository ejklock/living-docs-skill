"""Rank the in-force ADRs for every fixture query with each ranker and apply the fixed rule.

    uv run --locked --project evals/retrieval python evals/retrieval/replay.py --out DIR [--limit N]

Each scoring ranker's cut is calibrated on the calibration controls (seed 154 filler) only,
then recall at 3 and candidates per write are measured on the plants and the test controls
(seed 153 filler) at bundle sizes 60 and 240, each size judged apart.
"""

import argparse
import json
import time
from pathlib import Path
from typing import Callable

from bundle import Plant, bundle_records, plant_number
from calibrate import calibrate_cut, candidate_numbers, verdict, wilson
from lint_fixture import PLANT_FIELDS, load, query_text

FIXTURE = Path(__file__).resolve().parent / "fixture"
SIZES = (60, 240)
CALIBRATION_SEED = 154
TEST_SEED = 153
JUDGED = "static_table"
REFERENCE = "lexical"


def _interval(found: int, total: int) -> dict:
    low, high = wilson(found, total)
    return {"found": found, "total": total, "low": low, "high": high}


def _stats(plant_rows: list[dict], control_rows: list[dict]) -> dict:
    found = sum(row["plant"] in [number for number, _ in row["listed"]] for row in plant_rows)
    mean = sum(len(row["listed"]) for row in control_rows) / len(control_rows)
    return {"recall": _interval(found, len(plant_rows)), "mean_candidates": mean}


def _trials(plants: list[dict], test_controls: list[dict], records_for: Callable) -> list[tuple[dict, int, int | None, list]]:
    """Every measured trial: (item, size, plant number or None, the bundle)."""
    trials = []
    for size in SIZES:
        for item in plants:
            plant = Plant(*(item["plant"][field] for field in PLANT_FIELDS))
            trials.append((item, size, plant_number(size, item["position"]), records_for(size, item["position"], plant, TEST_SEED)))
        clean = records_for(size, None, None, TEST_SEED)
        trials += [(item, size, None, clean) for item in test_controls]
    return trials


def _measure(score: Callable, cut: float | None, trials: list) -> dict:
    rows = []
    for item, size, plant, adrs in trials:
        scored = score(query_text(item), adrs)
        if cut is None:
            listed = [[number, value] for number, value in scored][:3]
        else:
            by_number = dict(scored)
            listed = [[number, by_number[number]] for number in candidate_numbers(scored, cut)]
        rows.append({"query_id": item["id"], "size": size, "plant": plant, "listed": listed})
    sizes = {}
    for size in SIZES:
        at_size = [row for row in rows if row["size"] == size]
        sizes[str(size)] = _stats([r for r in at_size if r["plant"] is not None], [r for r in at_size if r["plant"] is None])
    pooled = _stats([r for r in rows if r["plant"] is not None], [r for r in rows if r["plant"] is None])
    return {"cut": cut, "sizes": sizes, "pooled": pooled, "trials": rows}


def _calibration_scores(score: Callable, controls: list[dict], records_for: Callable) -> list:
    return [
        score(query_text(item), records_for(size, None, None, CALIBRATION_SEED))
        for size in SIZES
        for item in controls
    ]


def _flat(stats: dict) -> dict:
    return {"found": stats["recall"]["found"], "total": stats["recall"]["total"], "mean_candidates": stats["mean_candidates"]}


def _verdicts(results: dict) -> dict:
    verdicts = {}
    for size in map(str, SIZES):
        passed, reason = verdict(_flat(results[JUDGED]["sizes"][size]), _flat(results[REFERENCE]["sizes"][size]))
        verdicts[size] = {"write_the_adr": passed, "reason": reason}
    verdicts["overall"] = "write the ADR" if all(verdicts[str(size)]["write_the_adr"] for size in SIZES) else "no ADR"
    return verdicts


def render(results: dict) -> str:
    lines = ["| ranker | cut | size | recall at 3 | 95% Wilson | candidates per write |", "|---|---|---|---|---|---|"]
    for name, ranker in results["rankers"].items():
        cut = "none" if ranker["cut"] is None else f"{ranker['cut']:.4f}"
        for size, stats in [*ranker["sizes"].items(), ("pooled", ranker["pooled"])]:
            recall = stats["recall"]
            lines.append(
                f"| {name} | {cut} | {size} | {recall['found']}/{recall['total']} "
                f"| [{recall['low']:.2f}, {recall['high']:.2f}] | {stats['mean_candidates']:.2f} |"
            )
    lines.append("")
    for size in map(str, SIZES):
        verdict_ = results["verdicts"][size]
        lines.append(f"{size}: write the ADR" if verdict_["write_the_adr"] else f"{size}: no ADR ({verdict_['reason']})")
    lines.append(f"overall: {results['verdicts']['overall']}")
    return "\n".join(lines)


def run_replay(plants: list[dict], controls: list[dict], rankers: dict[str, Callable], records_for: Callable, out_dir: Path) -> dict:
    """Calibrate, measure, judge and write `results.json` under `out_dir`.

    `rankers` holds the reference finder under `lexical` (no cut) and the scoring rankers;
    `records_for(size, position, plant, seed)` returns the in-force ADRs of a bundle.
    """
    calibration = [item for item in controls if item["split"] == "calibration"]
    test = [item for item in controls if item["split"] == "test"]
    lexical_lists = _calibration_scores(rankers[REFERENCE], calibration, records_for)
    lexical_mean = sum(len(scored) for scored in lexical_lists) / len(lexical_lists)
    trials = _trials(plants, test, records_for)
    measured = {}
    for name, score in rankers.items():
        if name == REFERENCE:
            measured[name] = _measure(score, None, trials)
        else:
            cut = calibrate_cut(_calibration_scores(score, calibration, records_for), lexical_mean)
            measured[name] = _measure(score, cut, trials)
    results = {"lexical_mean_candidates_on_calibration": lexical_mean, "rankers": measured}
    results["verdicts"] = _verdicts(measured)
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "results.json").write_text(json.dumps(results, indent=2))
    print(render(results))
    return results


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", required=True, type=Path, help="directory that receives results.json")
    parser.add_argument("--limit", type=int, help="keep the first N plants and N controls of each split (smoke run)")
    args = parser.parse_args()

    from rankers import load_rankers

    plants = load(FIXTURE, "plants.jsonl")
    controls = load(FIXTURE, "controls.jsonl")
    if args.limit is not None:
        plants = plants[: args.limit]
        controls = [c for split in ("calibration", "test") for c in [c for c in controls if c["split"] == split][: args.limit]]
    start = time.monotonic()
    run_replay(plants, controls, load_rankers(), lambda size, position, plant, seed: bundle_records(size, position, plant, seed), args.out)
    print(f"\nwall-clock {time.monotonic() - start:.1f}s for {len(plants)} plants and {len(controls)} controls")


if __name__ == "__main__":
    main()
