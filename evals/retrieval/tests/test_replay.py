import json

import pytest

from bundle import plant_number
from rare_words import Adr
from replay import run_replay

PLANT_FIELDS = {"title": "t", "description": "d", "context": "c", "decision": "do x"}


def plants(count):
    return [
        {
            "id": f"p{n:02d}",
            "position": "early" if n % 2 else "late",
            "query": {"title": f"plant-{n}", "description": "d", "body": "b"},
            "plant": PLANT_FIELDS,
        }
        for n in range(count)
    ]


def controls(per_split):
    items = []
    for split in ("calibration", "test"):
        for n in range(per_split):
            prefix = "cal" if split == "calibration" else "test"
            items.append({"id": f"c-{split}-{n}", "split": split, "query": {"title": f"{prefix}-{n}", "description": "d", "body": "b"}})
    return items


def make_records(seeds):
    def records_for(size, position, plant, seed):
        seeds.append((plant is not None, seed))
        marked = plant_number(size, position) if plant else None
        return [Adr(n, "t", "", "PLANT" if n == marked else "x") for n in range(1, size + 1)]

    return records_for


def lexical_lists_nothing(query, adrs):
    return []


def static_stub(finds_at, test_control_score=0.2):
    def score(query, adrs):
        if query.startswith("plant"):
            hit = finds_at[len(adrs)]
            return [(a.number, 0.9 if hit and a.body == "PLANT" else 0.1) for a in adrs]
        level = 0.3 if query.startswith("cal") else test_control_score
        return [(a.number, level) for a in adrs]

    return score


def replay(tmp_path, static, seeds=None, lexical=lexical_lists_nothing):
    seeds = [] if seeds is None else seeds
    rankers = {"lexical": lexical, "static_table": static}
    return run_replay(plants(20), controls(2), rankers, make_records(seeds), tmp_path)


def test_replay_calibrates_on_the_seed_154_controls_and_measures_on_seed_153(tmp_path):
    seeds = []
    replay(tmp_path, static_stub({60: True, 240: True}), seeds)
    assert (False, 154) in seeds and (False, 153) in seeds and (True, 153) in seeds
    assert (True, 154) not in seeds


def test_replay_writes_the_cut_recall_candidates_and_trial_rows(tmp_path):
    results = replay(tmp_path, static_stub({60: True, 240: True}))
    assert json.loads((tmp_path / "results.json").read_text()) == json.loads(json.dumps(results))
    static = results["rankers"]["static_table"]
    assert static["cut"] > 0.3
    assert static["sizes"]["60"]["recall"]["found"] == 20 and static["sizes"]["60"]["recall"]["total"] == 20
    assert 0.8 < static["sizes"]["60"]["recall"]["low"] < 1.0
    assert static["sizes"]["60"]["mean_candidates"] == 0
    assert static["pooled"]["recall"]["total"] == 40
    plant_rows = [row for row in static["trials"] if row["plant"] is not None and row["size"] == 60]
    assert len(plant_rows) == 20
    assert plant_rows[0]["query_id"] == "p00"
    assert plant_rows[0]["plant"] == plant_number(60, "late")
    assert plant_rows[0]["listed"] == [[plant_number(60, "late"), 0.9]]
    assert any(row["plant"] is None and row["query_id"].startswith("c-test") for row in static["trials"])
    assert results["rankers"]["lexical"]["cut"] is None


def test_replay_writes_the_ADR_only_when_the_static_table_wins_at_both_sizes(tmp_path):
    results = replay(tmp_path, static_stub({60: True, 240: True}))
    assert results["verdicts"]["60"]["write_the_adr"] and results["verdicts"]["240"]["write_the_adr"]
    assert results["verdicts"]["overall"] == "write the ADR"


def test_replay_reports_each_size_and_no_adr_when_the_static_table_wins_at_one(tmp_path, capsys):
    results = replay(tmp_path, static_stub({60: True, 240: False}))
    assert results["verdicts"]["60"]["write_the_adr"] is True
    assert results["verdicts"]["240"]["write_the_adr"] is False
    assert "recall" in results["verdicts"]["240"]["reason"]
    assert results["verdicts"]["overall"] == "no ADR"
    printed = capsys.readouterr().out
    assert "| static_table |" in printed
    assert "60: write the ADR" in printed and "240: no ADR" in printed and "overall: no ADR" in printed


def test_replay_names_the_candidates_clause_when_recall_wins_but_candidates_exceed(tmp_path):
    results = replay(tmp_path, static_stub({60: True, 240: True}, test_control_score=0.8))
    verdict = results["verdicts"]["60"]
    assert not verdict["write_the_adr"]
    assert "candidates" in verdict["reason"] and "recall" not in verdict["reason"]
    assert results["verdicts"]["overall"] == "no ADR"


def test_replay_keeps_the_cut_when_a_test_control_score_changes(tmp_path):
    low = replay(tmp_path, static_stub({60: True, 240: True}, test_control_score=0.2))
    high = replay(tmp_path, static_stub({60: True, 240: True}, test_control_score=0.8))
    assert low["rankers"]["static_table"]["cut"] == high["rankers"]["static_table"]["cut"]


def size_scaled_stub(query, adrs):
    if query.startswith("plant"):
        return [(a.number, 0.1) for a in adrs]
    if query.startswith("cal"):
        return [(a.number, len(adrs) / 1000 + a.number / 1_000_000) for a in adrs]
    return [(a.number, 0.2) for a in adrs]


def lexical_lists(on_calibration, on_test):
    def lexical(query, adrs):
        if query.startswith("plant"):
            return []
        return [(n, 1.0) for n in range(1, (on_calibration if query.startswith("cal") else on_test) + 1)]

    return lexical


def calibrated(tmp_path, on_test):
    return replay(tmp_path, size_scaled_stub, lexical=lexical_lists(1, on_test))


def test_replay_takes_the_lexical_calibration_mean_from_the_calibration_controls_only(tmp_path):
    few, many = calibrated(tmp_path, 0), calibrated(tmp_path, 3)
    assert few["lexical_mean_candidates_on_calibration"] == 1.0
    assert many["lexical_mean_candidates_on_calibration"] == 1.0
    assert few["rankers"]["static_table"]["cut"] == pytest.approx(0.240239)
    assert many["rankers"]["static_table"]["cut"] == pytest.approx(0.240239)


def test_replay_calibrates_one_cut_over_both_bundle_sizes(tmp_path):
    cut = calibrated(tmp_path, 1)["rankers"]["static_table"]["cut"]
    assert cut == pytest.approx(0.240239)
    assert cut != pytest.approx(0.060059)


def test_replay_takes_the_lexical_calibration_mean_over_both_bundle_sizes(tmp_path):
    def lexical(query, adrs):
        return [] if query.startswith("plant") else [(n, 1.0) for n in range(1, len(adrs) // 60 + 1)]

    results = replay(tmp_path, size_scaled_stub, lexical=lexical)
    assert results["lexical_mean_candidates_on_calibration"] == (1 + 4) / 2
