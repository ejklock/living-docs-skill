import io
import json
import subprocess
import sys
from pathlib import Path

import pytest

from bundle import Plant
from lint_fixture import main
from rare_words import Adr

HERE = Path(__file__).resolve().parents[1]


def tag(n: int) -> str:
    return "qx" + chr(97 + n // 26) + chr(97 + n % 26)


def plant_line(n, type_, position, body=None):
    return {
        "id": f"p{n:02d}",
        "type": type_,
        "position": position,
        "query": {"title": f"Title {tag(n)}", "description": f"Desc {tag(n)}", "body": body or f"Body {tag(n)}"},
        "plant": {"title": "T", "description": "D", "context": "C", "decision": f"do {tag(n + 50)}"},
    }


def control_line(n):
    split = "calibration" if n <= 20 else "test"
    return {"id": f"c{n:02d}", "split": split, "query": {"title": "T", "description": "D", "body": f"Body {tag(n + 100)}"}}


def valid_plants():
    return [plant_line(n, "t1" if n <= 20 else "t2", "early" if (n - 1) % 20 < 10 else "late") for n in range(1, 41)]


def valid_controls():
    return [control_line(n) for n in range(1, 41)]


def write_fixture(tmp_path, plants=None, controls=None):
    directory = tmp_path / "fixture"
    directory.mkdir()
    for name, lines in (("plants.jsonl", plants or valid_plants()), ("controls.jsonl", controls or valid_controls())):
        (directory / name).write_text("".join(json.dumps(line) + "\n" for line in lines))
    return directory


def filler(size, holders=0, word="zebra", text="filler"):
    """`size` records of which the first `holders` mention `word`."""
    return [Adr(n, "t", "d", f"{text} {word}" if n <= holders else text) for n in range(1, size + 1)]


def run(directory, records_for):
    out, err = io.StringIO(), io.StringIO()
    code = main([str(directory)], records_for=records_for, out=out, err=err)
    return code, out.getvalue() + err.getvalue()


def zebra_plants(body):
    plants = valid_plants()
    plants[0] = plant_line(1, "t1", "early", body)
    return plants


def clean(size, position, plant):
    return filler(size)


def test_a_fixture_whose_queries_share_no_word_passes(tmp_path):
    assert run(write_fixture(tmp_path), clean) == (0, "")


@pytest.mark.parametrize("holders, passes", [(2, False), (3, True)])
def test_a_shared_word_at_the_cut_fails_and_one_above_it_passes(tmp_path, holders, passes):
    directory = write_fixture(tmp_path, zebra_plants("a zebra"))
    code, text = run(directory, lambda size, position, plant: filler(size, holders if size == 60 else 0))
    assert (code == 0) is passes
    if not passes:
        assert "p01" in text and "zebra" in text and "60" in text


def test_a_word_rare_at_sixty_but_not_at_two_hundred_forty_names_sixty(tmp_path):
    directory = write_fixture(tmp_path, zebra_plants("a zebra"))
    code, text = run(directory, lambda size, position, plant: filler(size, 2 if size == 60 else 9))
    assert code == 1
    assert text.splitlines() == ["p01: shares rare word 'zebra' in the bundle of 60"]


def test_a_word_rare_only_at_two_hundred_forty_names_that_size(tmp_path):
    directory = write_fixture(tmp_path, zebra_plants("a zebra"))
    code, text = run(directory, lambda size, position, plant: filler(size, 5 if size == 60 else 7))
    assert code == 1
    assert text.splitlines() == ["p01: shares rare word 'zebra' in the bundle of 240"]


def test_a_stop_word_and_a_three_letter_word_are_not_shared(tmp_path):
    directory = write_fixture(tmp_path, zebra_plants("with cat"))
    assert run(directory, lambda s, p, pl: filler(s, 1, "with cat"))[0] == 0


def test_a_word_differing_only_by_case_is_shared(tmp_path):
    directory = write_fixture(tmp_path, zebra_plants("a ZEBRA"))
    assert run(directory, lambda s, p, pl: filler(s, 1))[0] == 1


def test_the_bundle_is_built_with_the_plant_and_its_position(tmp_path):
    seen = []
    directory = write_fixture(tmp_path)

    def records_for(size, position, plant):
        seen.append((size, position, plant))
        return filler(size)

    run(directory, records_for)
    assert len(seen) == 80
    assert {size for size, _, _ in seen} == {60, 240}
    assert seen[0] == (60, "early", Plant("T", "D", "C", f"do {tag(51)}"))


def mutate(lines, index, **changes):
    lines = [dict(line) for line in lines]
    lines[index].update(changes)
    return lines


@pytest.mark.parametrize(
    "plants, controls, rule",
    [
        (mutate(valid_plants(), 1, id="p01"), None, "duplicate id p01"),
        (valid_plants()[:39], None, "plants: expected 40, found 39"),
        (None, valid_controls() + [control_line(41)], "controls: expected 40, found 41"),
        (mutate(valid_plants(), 0, type="t2"), None, "types: expected 20 t1 and 20 t2"),
        (mutate(valid_plants(), 0, position="late"), None, "positions: expected 20 early and 20 late"),
        (mutate(mutate(valid_plants(), 0, position="late"), 30, position="early"), None, "positions within t1"),
        (None, mutate(valid_controls(), 0, split="test"), "splits: expected 20 calibration and 20 test"),
    ],
)
def test_a_broken_shape_rule_exits_one_naming_the_rule(tmp_path, plants, controls, rule):
    code, text = run(write_fixture(tmp_path, plants, controls), clean)
    assert code == 1
    assert rule in text


def test_a_missing_file_exits_two_naming_it(tmp_path):
    directory = write_fixture(tmp_path)
    (directory / "controls.jsonl").unlink()
    code, text = run(directory, clean)
    assert code == 2
    assert "controls.jsonl" in text


def test_a_line_that_is_not_json_exits_two_naming_file_and_line(tmp_path):
    directory = write_fixture(tmp_path)
    lines = (directory / "plants.jsonl").read_text().splitlines()
    lines[6] = "{not json"
    (directory / "plants.jsonl").write_text("\n".join(lines) + "\n")
    code, text = run(directory, clean)
    assert code == 2
    assert "plants.jsonl:7" in text


@pytest.mark.parametrize("field", ["query", "plant", "type", "id"])
def test_a_line_missing_a_field_exits_two_naming_file_and_line(tmp_path, field):
    plants = valid_plants()
    del plants[2][field]
    code, text = run(write_fixture(tmp_path, plants), clean)
    assert code == 2
    assert "plants.jsonl:3" in text


def test_an_empty_string_exits_two(tmp_path):
    plants = valid_plants()
    plants[4]["query"]["body"] = ""
    code, text = run(write_fixture(tmp_path, plants), clean)
    assert code == 2
    assert "plants.jsonl:5" in text


def test_the_command_exits_two_for_a_missing_fixture_directory(tmp_path):
    done = subprocess.run(
        [sys.executable, str(HERE / "lint_fixture.py"), str(tmp_path / "none")],
        capture_output=True,
        text=True,
    )
    assert done.returncode == 2
    assert "plants.jsonl" in done.stderr
