"""Calibration and the decision rule for the retrieval replay.

A scored list is `(record number, score)` for every in-force record of one write.
"""

import math
from statistics import NormalDist

MAX_LISTED = 3
Z = NormalDist().inv_cdf(0.975)  # two-sided 95% normal quantile

Scores = list[tuple[int, float]]
Interval = tuple[float, float]


def wilson(found: int, total: int) -> Interval:
    """The 95% Wilson score interval of `found` out of `total`."""
    if total <= 0:
        raise ValueError("a Wilson interval needs at least one trial")
    p = found / total
    denominator = 1 + Z * Z / total
    centre = (p + Z * Z / (2 * total)) / denominator
    half = Z * math.sqrt(p * (1 - p) / total + Z * Z / (4 * total * total)) / denominator
    return max(0.0, centre - half), min(1.0, centre + half)


def whole_above(interval: Interval, other: Interval) -> bool:
    """True only when the lower bound of `interval` is strictly above the upper bound of `other`."""
    return interval[0] > other[1]


def candidate_numbers(scores: Scores, cut: float) -> list[int]:
    """Records scoring at or above the cut: at most three, best score first, then lowest number."""
    kept = sorted((item for item in scores if item[1] >= cut), key=lambda item: (-item[1], item[0]))
    return [number for number, _ in kept[:MAX_LISTED]]


def mean_candidates(controls: list[Scores], cut: float) -> float:
    """The mean number of candidates one write gets over the controls at this cut."""
    return sum(len(candidate_numbers(scores, cut)) for scores in controls) / len(controls)


def calibrate_cut(controls: list[Scores], lexical_mean: float) -> float:
    """The lowest score whose mean candidates over the controls is at most the lexical mean.

    No score qualifying puts the cut just above the highest score, where nothing is listed.
    """
    if not controls:
        raise ValueError("calibration needs at least one control")
    distinct = sorted({score for scores in controls for _, score in scores})
    for score in distinct:
        if mean_candidates(controls, score) <= lexical_mean:
            return score
    return math.nextafter(distinct[-1], math.inf) if distinct else 0.0


def verdict(ranker: dict, lexical: dict) -> tuple[bool, str]:
    """Whether a ranker clears the rule against the lexical finder, and the clauses it fails.

    Each side holds `found`, `total` and `mean_candidates`.
    """
    failed = []
    if not whole_above(wilson(ranker["found"], ranker["total"]), wilson(lexical["found"], lexical["total"])):
        failed.append("recall at 3 interval is not wholly above the lexical one")
    if ranker["mean_candidates"] > lexical["mean_candidates"]:
        failed.append("candidates per write exceed the lexical mean")
    return not failed, "; ".join(failed)
