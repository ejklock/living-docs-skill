import re
from pathlib import Path

import pytest

import rare_words
from rare_words import Adr, Candidate, candidates, qualifying, rare_cut, rare_query_words, words

CANDIDATES_RS = Path(__file__).resolve().parents[3] / "living-docs-core" / "src" / "commands" / "effective" / "candidates.rs"


def adr(number, text, title="t", description="d"):
    return Adr(number, title, description, text)


def test_words_are_lowercased_ascii_runs_of_four_or_more():
    assert words("Alpha BETA cat gamma") == {"alpha", "beta", "gamma"}


def test_a_hyphen_a_digit_and_a_non_ascii_letter_each_end_a_run():
    assert words("five-second v2alpha cafébar naïve") == {"five", "second", "alpha"}


def test_stop_words_are_dropped_in_any_case():
    assert words("With WHICH would Zebra") == {"zebra"}


def test_rare_cut_is_three_percent_rounded_down_and_never_below_two():
    assert [rare_cut(n) for n in (0, 48, 60, 99, 100, 190, 240)] == [2, 2, 2, 2, 3, 5, 7]


def test_a_word_in_exactly_the_cut_many_records_is_rare_and_one_more_is_not():
    docs = [adr(n, "") for n in range(1, 101)]
    at_cut = [d._replace(body="zebra") if d.number <= 3 else d for d in docs]
    above = [d._replace(body="zebra") if d.number <= 4 else d for d in docs]
    assert rare_query_words("zebra", at_cut) == {"zebra": 3}
    assert rare_query_words("zebra", above) == {}


def test_a_word_in_no_record_is_not_shared():
    assert rare_query_words("zebra", [adr(1, "other")]) == {}


def test_a_title_a_description_and_a_body_each_count_for_one_record():
    docs = [Adr(1, "zebra", "zebra", "zebra"), adr(2, "quokka")]
    assert rare_query_words("Zebra quokka", docs) == {"zebra": 1, "quokka": 1}


def test_candidates_need_two_shared_rare_words():
    docs = [adr(1, "zebra quokka"), adr(2, "zebra")]
    assert candidates("zebra quokka", docs) == [Candidate(1, "t", ("quokka", "zebra"))]


def test_candidates_are_capped_at_three_ordered_by_shared_count_then_number():
    docs = [
        adr(1, "zebra quokka"),
        adr(2, "zebra quokka walrus"),
        adr(3, "zebra quokka"),
        adr(4, "zebra quokka"),
        adr(5, "zebra quokka walrus"),
    ] + [adr(n, "filler") for n in range(6, 200)]
    query = "zebra quokka walrus"
    assert len(qualifying(query, docs)) == 5
    assert [c.number for c in candidates(query, docs)] == [2, 5, 1]


def test_a_word_present_only_in_a_title_counts_for_that_record():
    docs = [Adr(1, "zebra quokka", "d", "other"), adr(2, "filler")]
    assert rare_query_words("zebra quokka", docs) == {"zebra": 1, "quokka": 1}
    assert candidates("zebra quokka", docs) == [Candidate(1, "zebra quokka", ("quokka", "zebra"))]


def rust_stop_words():
    if not CANDIDATES_RS.is_file():
        pytest.fail(f"{CANDIDATES_RS} is missing: the stop list is read from it")
    found = re.search(r'const STOP_WORDS: &str = "(.*?)";', CANDIDATES_RS.read_text(), re.S)
    if found is None:
        pytest.fail(f"no STOP_WORDS string constant in {CANDIDATES_RS}")
    return set(found.group(1).replace("\\\n", " ").split())


def test_the_stop_list_equals_the_one_in_the_rust_finder():
    expected = rust_stop_words()
    actual = set(rare_words.STOP_WORDS)
    missing, extra = sorted(expected - actual), sorted(actual - expected)
    assert not missing and not extra, f"missing from Python: {missing}; extra in Python: {extra}"
