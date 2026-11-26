"""Fixture lint: shape rules, then the rare-word rule over the seed bundles.

Exit 0 when the fixture is clean, 1 when a rule or a plant fails, 2 when a
file is missing or a line cannot be read.
"""

import json
import sys
from collections import Counter
from pathlib import Path

from bundle import Plant, bundle_records
from rare_words import rare_query_words

SIZES = (60, 240)
PLANTS = 40
CONTROLS = 40
FIXTURE = Path(__file__).resolve().parent / "fixture"

QUERY_FIELDS = ("title", "description", "body")
PLANT_FIELDS = ("title", "description", "context", "decision")
SHAPES = {
    "plants.jsonl": {"id": None, "type": None, "position": None, "query": QUERY_FIELDS, "plant": PLANT_FIELDS},
    "controls.jsonl": {"id": None, "split": None, "query": QUERY_FIELDS},
}


class FixtureError(Exception):
    """A file that is missing or a line that cannot be read."""


def _check_strings(value, fields, where):
    if fields is None:
        ok = isinstance(value, str) and value != ""
    else:
        ok = isinstance(value, dict) and all(_check_strings(value.get(f), None, where) for f in fields)
    if not ok:
        raise FixtureError(where)
    return True


def load(directory: Path, name: str) -> list[dict]:
    path = directory / name
    if not path.is_file():
        raise FixtureError(f"{name}: file is missing")
    items = []
    for number, line in enumerate(path.read_text().splitlines(), 1):
        where = f"{name}:{number}"
        try:
            item = json.loads(line)
        except json.JSONDecodeError as error:
            raise FixtureError(f"{where}: not JSON ({error.msg})") from None
        if not isinstance(item, dict):
            raise FixtureError(f"{where}: not a JSON object")
        for field, nested in SHAPES[name].items():
            _check_strings(item.get(field), nested, f"{where}: missing or empty field {field}")
        items.append(item)
    return items


def shape_failures(plants: list[dict], controls: list[dict]) -> list[str]:
    failures = []
    ids = Counter(item["id"] for item in plants + controls)
    failures += [f"duplicate id {id_}" for id_, count in sorted(ids.items()) if count > 1]
    if len(plants) != PLANTS:
        failures.append(f"plants: expected {PLANTS}, found {len(plants)}")
    if len(controls) != CONTROLS:
        failures.append(f"controls: expected {CONTROLS}, found {len(controls)}")
    if [p["id"] for p in plants] != [f"p{n:02d}" for n in range(1, PLANTS + 1)]:
        failures.append("ids: plants must be p01 to p40 in order")
    if [c["id"] for c in controls] != [f"c{n:02d}" for n in range(1, CONTROLS + 1)]:
        failures.append("ids: controls must be c01 to c40 in order")
    types = Counter(p["type"] for p in plants)
    if types != {"t1": PLANTS // 2, "t2": PLANTS // 2}:
        failures.append("types: expected 20 t1 and 20 t2")
    if Counter(p["position"] for p in plants) != {"early": PLANTS // 2, "late": PLANTS // 2}:
        failures.append("positions: expected 20 early and 20 late")
    for type_ in ("t1", "t2"):
        count = Counter(p["position"] for p in plants if p["type"] == type_)
        if count != {"early": PLANTS // 4, "late": PLANTS // 4}:
            failures.append(f"positions within {type_}: expected 10 early and 10 late")
    expected = ["calibration"] * (CONTROLS // 2) + ["test"] * (CONTROLS // 2)
    if [c["split"] for c in controls] != expected:
        failures.append("splits: expected 20 calibration and 20 test")
    return failures


def query_text(item: dict) -> str:
    query = item["query"]
    return "\n".join(query[f] for f in QUERY_FIELDS)


def plant_failures(plants: list[dict], records_for) -> list[str]:
    failures = []
    for item in plants:
        plant = Plant(*(item["plant"][f] for f in PLANT_FIELDS))
        for size in SIZES:
            rare = rare_query_words(query_text(item), records_for(size, item["position"], plant))
            failures += [f"{item['id']}: shares rare word '{w}' in the bundle of {size}" for w in sorted(rare)]
    return failures


def main(argv: list[str], records_for=bundle_records, out=sys.stdout, err=sys.stderr) -> int:
    directory = Path(argv[0]) if argv else FIXTURE
    try:
        plants = load(directory, "plants.jsonl")
        controls = load(directory, "controls.jsonl")
    except FixtureError as error:
        print(f"lint_fixture: {error}", file=err)
        return 2
    failures = shape_failures(plants, controls) + plant_failures(plants, records_for)
    for failure in failures:
        print(failure, file=out)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
