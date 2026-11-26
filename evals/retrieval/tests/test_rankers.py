import numpy as np
import pytest

from rankers import (
    bm25_score,
    embedding_ranker,
    lexical_score,
    quantize_table,
    static_table_ranker,
    sum_rows,
)
from rare_words import Adr, candidates


def corpus(*texts):
    return [Adr(number, f"title {number}", "", text) for number, text in enumerate(texts, 1)]


def scored(scores):
    return dict(scores)


class FakeTable:
    """Tokens are the letters a to d, one table row each."""

    embedding = np.array([[0.5, 0.0], [0.0, 0.5], [-0.5, 0.25], [0.25, 0.25]], dtype=np.float32)

    def tokenize(self, texts):
        return [[ord(c) - ord("a") for c in text if c in "abcd"] for text in texts]


def test_static_table_quantizes_the_largest_magnitude_to_127():
    quantized = quantize_table(np.array([[0.5, -0.5], [0.25, 0.0]], dtype=np.float32))
    assert quantized.dtype == np.int8
    assert quantized[0].tolist() == [127, -127]
    assert quantized[1].tolist() == [64, 0]


def test_static_table_quantizes_an_all_zero_table_to_zeros():
    assert quantize_table(np.zeros((2, 2), dtype=np.float32)).tolist() == [[0, 0], [0, 0]]


def test_static_table_sums_token_rows_as_integers():
    table = quantize_table(FakeTable.embedding)
    total = sum_rows([0, 0, 1], table)
    assert total.dtype == np.int64
    assert total.tolist() == [254, 127]
    assert sum_rows([], table).tolist() == [0, 0]
    assert sum_rows([], table).dtype == np.int64


def test_static_table_scores_the_identical_record_highest():
    adrs = [Adr(1, "aab", "", ""), Adr(2, "cdd", "", ""), Adr(3, "bbb", "", "")]
    score = static_table_ranker(FakeTable())
    ranked = scored(score("aab\n\n", adrs))
    assert ranked[1] == pytest.approx(1.0)
    assert ranked[1] > ranked[2] and ranked[1] > ranked[3]


def test_static_table_scores_an_empty_token_list_as_zero():
    score = static_table_ranker(FakeTable())
    adrs = [Adr(1, "aab", "", ""), Adr(2, "zzz", "", "")]
    assert scored(score("zzz\n\n", adrs)) == {1: 0.0, 2: 0.0}
    assert scored(score("aab\n\n", adrs))[2] == 0.0


def test_static_table_computes_dot_products_without_floats():
    from rankers import integer_dot

    left, right = sum_rows([0, 1], quantize_table(FakeTable.embedding)), sum_rows([0], quantize_table(FakeTable.embedding))
    dot = integer_dot(left, right)
    assert isinstance(dot, (int, np.integer))
    assert dot == 127 * 127


def test_ranker_bm25_scores_the_record_sharing_query_terms_highest():
    adrs = corpus("apples oranges bananas", "database migrations rollback", "frontend widgets styling")
    scores = scored(bm25_score("database rollback plan", adrs))
    assert scores[2] > scores[1] and scores[2] > scores[3]
    assert scores[1] == 0 and scores[3] == 0


def test_ranker_bm25_scores_a_query_of_stop_words_as_zero():
    adrs = corpus("apples oranges", "database migrations")
    assert scored(bm25_score("the of and", adrs)) == {1: 0.0, 2: 0.0}


def test_ranker_returns_one_deterministic_score_per_record():
    adrs = corpus("apples oranges bananas", "database migrations rollback", "frontend widgets")
    for score in (bm25_score, static_table_ranker(FakeTable()), embedding_ranker(lambda texts: np.array([[len(t), 1.0] for t in texts]))):
        first, second = score("database rollback", adrs), score("database rollback", adrs)
        assert first == second
        assert [number for number, _ in first] == [1, 2, 3]


def test_ranker_lexical_returns_exactly_the_finder_list():
    adrs = [Adr(n, f"unrelated {n}", "", "filler words everywhere") for n in range(1, 8)]
    adrs[2] = Adr(3, "zymurgy brewing", "", "zymurgy brewing kettle wort")
    query = "zymurgy brewing kettle wort"
    listed = candidates(query, adrs)
    assert listed and [(c.number, float(len(c.shared))) for c in listed] == lexical_score(query, adrs)


def test_ranker_embedding_scores_cosine_similarity_from_a_stub_model():
    vectors = {"query": [1.0, 0.0], "title 1\n\nnear": [2.0, 0.0], "title 2\n\nfar": [0.0, 3.0]}
    adrs = [Adr(1, "title 1", "", "near"), Adr(2, "title 2", "", "far")]
    score = embedding_ranker(lambda texts: np.array([vectors[t] for t in texts]))
    assert scored(score("query", adrs)) == {1: pytest.approx(1.0), 2: pytest.approx(0.0)}


def test_ranker_bm25_drops_english_stop_words_from_records_and_query():
    adrs = corpus("the the the apples", "oranges bananas")
    assert scored(bm25_score("the", adrs)) == {1: 0.0, 2: 0.0}


def test_ranker_bm25_ignores_stop_words_when_measuring_record_length():
    adrs = corpus("the the the apples", "apples")
    scores = scored(bm25_score("apples", adrs))
    assert scores[1] == pytest.approx(scores[2])
    assert scores[1] > 0
