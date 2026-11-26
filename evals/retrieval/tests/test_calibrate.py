import pytest

from calibrate import (
    candidate_numbers,
    calibrate_cut,
    mean_candidates,
    verdict,
    whole_above,
    wilson,
)


def rounded(interval):
    return tuple(round(bound, 2) for bound in interval)


def test_wilson_matches_the_shell_aggregate():
    assert rounded(wilson(11, 12)) == (0.65, 0.99)
    assert rounded(wilson(0, 36)) == (0.0, 0.10)
    assert rounded(wilson(20, 20)) == (0.84, 1.0)


def test_wilson_refuses_an_empty_sample():
    with pytest.raises(ValueError):
        wilson(0, 0)


def test_wilson_interval_is_wholly_above_only_when_the_bounds_clear():
    assert whole_above((0.5, 0.9), (0.1, 0.4))
    assert not whole_above((0.4, 0.9), (0.1, 0.4))
    assert not whole_above((0.3, 0.9), (0.1, 0.4))


def test_candidates_order_by_score_then_number_and_cap_at_three():
    scores = [(5, 0.9), (2, 0.9), (7, 0.8), (1, 0.7), (9, 0.1)]
    assert candidate_numbers(scores, 0.7) == [2, 5, 7]
    assert candidate_numbers(scores, 0.75) == [2, 5, 7]
    assert candidate_numbers(scores, 0.95) == []


def test_candidates_count_every_record_tied_at_the_cut_before_the_cap():
    scores = [(4, 0.5), (3, 0.5), (2, 0.5), (1, 0.5), (8, 0.2)]
    assert candidate_numbers(scores, 0.5) == [1, 2, 3]
    assert mean_candidates([scores], 0.5) == 3


def test_calibrate_cut_sits_above_every_score_when_the_lexical_finder_lists_nothing():
    controls = [[(1, 0.9), (2, 0.4)], [(1, 0.2), (2, 0.1)]]
    cut = calibrate_cut(controls, lexical_mean=0)
    assert cut > 0.9
    assert mean_candidates(controls, cut) == 0


def test_calibrate_cut_takes_the_score_whose_mean_equals_the_lexical_mean():
    controls = [[(1, 0.9), (2, 0.4)], [(1, 0.8), (2, 0.3)]]
    assert mean_candidates(controls, 0.8) == 1
    assert calibrate_cut(controls, lexical_mean=1) == 0.8


def test_calibrate_cut_refuses_the_score_that_pushes_the_mean_over_the_lexical_mean():
    controls = [[(1, 0.9), (2, 0.4)], [(1, 0.8), (2, 0.3)]]
    assert mean_candidates(controls, 0.4) == 1.5
    assert calibrate_cut(controls, lexical_mean=1.4) == 0.8
    assert calibrate_cut(controls, lexical_mean=0.5) == 0.9


def test_calibrate_cut_applies_the_cap_to_tied_scores():
    controls = [[(n, 0.5) for n in range(1, 6)]]
    assert calibrate_cut(controls, lexical_mean=3) == 0.5
    assert calibrate_cut(controls, lexical_mean=2.9) > 0.5


def test_calibrate_cut_lies_above_an_all_equal_score_unless_that_mean_fits():
    controls = [[(n, 0.5) for n in range(1, 3)], [(n, 0.5) for n in range(1, 3)]]
    assert calibrate_cut(controls, lexical_mean=1) > 0.5
    assert calibrate_cut(controls, lexical_mean=2) == 0.5


def test_calibrate_cut_refuses_an_empty_calibration_set():
    with pytest.raises(ValueError):
        calibrate_cut([], lexical_mean=1)


def test_verdict_needs_recall_wholly_above_and_no_more_candidates():
    lexical = {"found": 0, "total": 40, "mean_candidates": 0.5}
    winner = {"found": 30, "total": 40, "mean_candidates": 0.5}
    assert verdict(winner, lexical) == (True, "")

    wordy = {"found": 30, "total": 40, "mean_candidates": 0.6}
    passed, reason = verdict(wordy, lexical)
    assert not passed and "candidates" in reason and "recall" not in reason

    close = {"found": 2, "total": 40, "mean_candidates": 0.5}
    passed, reason = verdict(close, {"found": 1, "total": 40, "mean_candidates": 0.5})
    assert not passed and "recall" in reason and "candidates" not in reason
