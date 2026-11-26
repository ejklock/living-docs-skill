"""The four rankers behind one shape: `score(query, adrs) -> [(record number, score)]`.

Every ranker but the lexical one scores every record; the lexical one returns the records
the binary lists, most shared words first.
"""

import os
from pathlib import Path
from typing import Callable

import bm25s
import numpy as np

from rare_words import Adr, candidates

MODELS_DIR = Path(os.environ.get("XDG_DATA_HOME") or Path.home() / ".local/share") / "living-docs-evals" / "models"
os.environ.setdefault("HF_HOME", str(MODELS_DIR))

STATIC_TABLE_REPO = "minishlab/potion-base-8M"
STATIC_TABLE_REVISION = "bf8b056651a2c21b8d2565580b8569da283cab23"
MINILM_REPO = "sentence-transformers/all-MiniLM-L6-v2"
MINILM_REVISION = "1110a243fdf4706b3f48f1d95db1a4f5529b4d41"

Scores = list[tuple[int, float]]
Ranker = Callable[[str, list[Adr]], Scores]


def record_text(adr: Adr) -> str:
    return f"{adr.title}\n{adr.description}\n{adr.body}"


def lexical_score(query: str, adrs: list[Adr]) -> Scores:
    """The records the binary's rare-word finder lists, scored by the rare words they share."""
    return [(c.number, float(len(c.shared))) for c in candidates(query, adrs)]


def bm25_score(query: str, adrs: list[Adr]) -> Scores:
    """BM25 over title, description and body, English stop words, no stemmer."""
    corpus = bm25s.tokenize([record_text(a) for a in adrs], stopwords="en", stemmer=None, show_progress=False)
    retriever = bm25s.BM25()
    retriever.index(corpus, show_progress=False)
    terms = bm25s.tokenize(query, stopwords="en", stemmer=None, return_ids=False, show_progress=False)[0]
    values = retriever.get_scores(terms) if terms else np.zeros(len(adrs))
    return [(adr.number, float(value)) for adr, value in zip(adrs, values)]


def _cosine(left: np.ndarray, right: np.ndarray) -> float:
    norms = float(np.linalg.norm(left)) * float(np.linalg.norm(right))
    return float(left @ right) / norms if norms else 0.0


def embedding_ranker(encode: Callable[[list[str]], np.ndarray]) -> Ranker:
    """Cosine similarity of the query and each record under `encode`, which embeds a list of texts."""
    cache: dict[str, np.ndarray] = {}

    def score(query: str, adrs: list[Adr]) -> Scores:
        texts = [query, *(record_text(adr) for adr in adrs)]
        missing = list(dict.fromkeys(text for text in texts if text not in cache))
        if missing:
            cache.update(zip(missing, np.asarray(encode(missing), dtype=np.float64)))
        return [(adr.number, _cosine(cache[query], cache[text])) for adr, text in zip(adrs, texts[1:])]

    return score


def quantize_table(table: np.ndarray) -> np.ndarray:
    """int8 rows under one symmetric scale: the largest magnitude maps to 127."""
    largest = float(np.abs(table).max())
    scale = 127.0 / largest if largest else 1.0
    return np.rint(table * scale).astype(np.int8)


def sum_rows(token_ids: list[int], table: np.ndarray) -> np.ndarray:
    """The integer sum of a text's token rows."""
    return table[token_ids].sum(axis=0, dtype=np.int64) if token_ids else np.zeros(table.shape[1], dtype=np.int64)


def integer_dot(left: np.ndarray, right: np.ndarray) -> int:
    return int(np.dot(left, right))


def static_table_ranker(model) -> Ranker:
    """Integer arithmetic over a static token table: int8 rows, int64 sums and dot products.

    `model` supplies `tokenize(texts) -> token id lists` and `embedding`, the float table.
    The only float step is the final division by the product of the two Euclidean norms.
    """
    table = quantize_table(np.asarray(model.embedding))
    cache: dict[str, np.ndarray] = {}

    def total(text: str) -> np.ndarray:
        if text not in cache:
            cache[text] = sum_rows(model.tokenize([text])[0], table)
        return cache[text]

    def score(query: str, adrs: list[Adr]) -> Scores:
        left = total(query)
        left_square = integer_dot(left, left)
        found = []
        for adr in adrs:
            right = total(record_text(adr))
            denominator = (left_square * integer_dot(right, right)) ** 0.5
            found.append((adr.number, integer_dot(left, right) / denominator if denominator else 0.0))
        return found

    return score


def _snapshot(repo: str, revision: str) -> str:
    from huggingface_hub import snapshot_download

    return snapshot_download(repo, revision=revision)


def load_static_table_ranker() -> Ranker:
    from model2vec import StaticModel

    model = StaticModel.from_pretrained(_snapshot(STATIC_TABLE_REPO, STATIC_TABLE_REVISION), force_download=False)
    if model.token_mapping is not None or model.weights is not None:
        raise RuntimeError("the static table has a token mapping or weights: a plain row sum would not match its encoder")
    return static_table_ranker(model)


def load_minilm_ranker() -> Ranker:
    from sentence_transformers import SentenceTransformer

    model = SentenceTransformer(_snapshot(MINILM_REPO, MINILM_REVISION), device="cpu")
    return embedding_ranker(lambda texts: model.encode(texts, show_progress_bar=False))


def load_rankers() -> dict[str, Ranker]:
    """The four rankers by name; the two models come from the pinned revisions in the local cache."""
    return {
        "lexical": lexical_score,
        "bm25": bm25_score,
        "static_table": load_static_table_ranker(),
        "minilm": load_minilm_ranker(),
    }
