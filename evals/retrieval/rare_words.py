"""Rare-word finder of `living-docs write new adr`, ported for offline use.

The constants and the stop list mirror the Rust finder literally; a parity
test against the binary keeps the two from drifting.
"""

import re
from typing import NamedTuple

MIN_WORD_LEN = 4
MIN_SHARED = 2
MAX_LISTED = 3

STOP_WORDS = frozenset(
    """about after again against also always among because been before
    being between both cannot could does done down during each either else even ever every
    from further have having here hers herself himself into itself just keep like made make
    many more most much must never none once only other others otherwise ours over same shall
    should since some such than that their theirs them then there these they this those though
    through thus under until upon used uses using very want wants were what when whenever
    where whether which while whom whose will with within without would your yours""".split()
)

_RUN = re.compile(r"[A-Za-z]+")


class Adr(NamedTuple):
    number: int
    title: str
    description: str
    body: str


class Candidate(NamedTuple):
    number: int
    title: str
    shared: tuple[str, ...]


def words(text: str) -> frozenset[str]:
    """Lowercased ASCII letter runs of four or more letters that are not stop words."""
    found = (run.lower() for run in _RUN.findall(text) if len(run) >= MIN_WORD_LEN)
    return frozenset(word for word in found if word not in STOP_WORDS)


def rare_cut(total: int) -> int:
    """The most records a word may occur in and still be rare."""
    return max(total * 3 // 100, 2)


def _hits(query_words: frozenset[str], adr: Adr) -> frozenset[str]:
    return query_words & words(f"{adr.title}\n{adr.description}\n{adr.body}")


def _document_frequency(query_words: frozenset[str], adrs: list[Adr]) -> tuple[list[frozenset[str]], dict[str, int]]:
    per_adr = [_hits(query_words, adr) for adr in adrs]
    frequency = {word: sum(word in hit for hit in per_adr) for word in query_words}
    return per_adr, frequency


def rare_query_words(query: str, adrs: list[Adr]) -> dict[str, int]:
    """Query words that occur in at least one record and are rare, with their document frequency."""
    _, frequency = _document_frequency(words(query), adrs)
    cut = rare_cut(len(adrs))
    return {word: count for word, count in frequency.items() if 0 < count <= cut}


def qualifying(query: str, adrs: list[Adr]) -> list[Candidate]:
    """Every record sharing enough rare words, before the listing cap, in listing order."""
    query_words = words(query)
    per_adr, frequency = _document_frequency(query_words, adrs)
    cut = rare_cut(len(adrs))
    rare = {word for word, count in frequency.items() if count <= cut}
    found = []
    for adr, hit in zip(adrs, per_adr):
        shared = tuple(sorted(hit & rare))
        if len(shared) >= MIN_SHARED:
            found.append(Candidate(adr.number, adr.title, shared))
    found.sort(key=lambda c: (-len(c.shared), c.number))
    return found


def candidates(query: str, adrs: list[Adr]) -> list[Candidate]:
    """The records the binary lists: at most three, most shared words first."""
    return qualifying(query, adrs)[:MAX_LISTED]
