import json
import shutil
import subprocess
import tempfile
from pathlib import Path

import pytest

from bundle import Plant, build_scale_repo, scratch_dir, living_docs_binary, repo_records
from rare_words import candidates, qualifying

FIXTURE = Path(__file__).resolve().parents[1] / "fixture"
PLANT = Plant(
    "Argon2id hashing for passwords",
    "Stored passwords are hashed with Argon2id before they reach the database.",
    "A leaked database must not expose anyone's password.",
    "hash every stored password with Argon2id before it is written to the database",
)

# Four records sharing two rare words each, pairwise, so each word occurs in exactly two
# records; a fifth record alone holds three more, and a sixth holds two rare words in its
# title alone. With the 60 ADRs of the scale bundle the listing repository holds 66.
EXTRA = [
    ("Alpha ledger", "zebra ocelot"),
    ("Beta ledger", "zebra tapir"),
    ("Gamma ledger", "ocelot pangolin"),
    ("Delta ledger", "tapir pangolin"),
    ("Epsilon ledger", "narwhal marmot lemur"),
    ("Vicuna capybara", "filler"),
]
LISTED = [
    "zebra ocelot tapir pangolin",
    "zebra ocelot tapir pangolin narwhal marmot lemur",
    "zebra ocelot",
    "Argon2id hashed passwords database",
    "narwhal marmot",
    "vicuna capybara",
]
UNLISTED = ["zebra", "gibberish nonsense words", "with cat and also the", "ocelot narwhal"]


def binary():
    try:
        return living_docs_binary()
    except FileNotFoundError as error:
        pytest.fail(str(error))


def add_adr(repo, title, text):
    body = f"## Context\n\n{text}\n\n## Decision\n\nWe will {text}.\n\n## Consequences\n\n- One answer.\n"
    made = subprocess.run(
        [str(binary()), "write", "new", "adr", title, "--description", "d", "--body", "-", "--json"],
        cwd=repo, input=body, capture_output=True, text=True, check=True,
    )  # fmt: skip
    number = Path(json.loads(made.stdout)["path"]).name[:4]
    subprocess.run([str(binary()), "write", "set", f"adr/{number}", "status", "Accepted"], cwd=repo, check=True, capture_output=True)


@pytest.fixture(scope="module")
def scratch():
    with scratch_dir() as path:
        yield path


@pytest.fixture(scope="module")
def repo(scratch):
    """The seed bundle of 60 ADRs with a plant, plus the extra records: 66 ADRs."""
    built = scratch / "listing"
    built.mkdir()
    build_scale_repo(built / "repo", 60, "early", PLANT)
    for title, text in EXTRA:
        add_adr(built / "repo", title, text)
    return built / "repo"


@pytest.fixture(scope="module")
def plain_repo(scratch):
    """The plain seed bundle of 60 ADRs, no plant and no extra record."""
    built = scratch / "plain"
    built.mkdir()
    build_scale_repo(built / "repo", 60)
    return built / "repo"


def binary_candidates(repo, query):
    with tempfile.TemporaryDirectory() as parent:
        copy = Path(parent) / "copy"
        shutil.copytree(repo, copy)
        done = subprocess.run(
            [str(binary()), "write", "new", "adr", query["title"], "--description", query["description"], "--body", "-", "--json"],
            cwd=copy, input=query["body"], capture_output=True, text=True, check=True,
        )  # fmt: skip
    listed = json.loads(done.stdout).get("candidates", [])
    return [(c["number"], c["title"], tuple(c["shared"])) for c in listed]


def python_candidates(records, query):
    text = "\n".join((query["title"], query["description"], query["body"]))
    return [tuple(c) for c in candidates(text, records)]


def as_query(words):
    return {"title": "New decision", "description": "A decision", "body": words}


def test_the_python_finder_lists_what_the_binary_lists_for_queries_that_match(repo):
    records = repo_records(repo)
    for words in LISTED:
        query = as_query(words)
        expected = binary_candidates(repo, query)
        assert expected, words
        assert python_candidates(records, query) == expected, words


def test_a_query_with_more_than_three_qualifying_records_is_capped_in_the_same_order(repo):
    records = repo_records(repo)
    query = as_query("zebra ocelot tapir pangolin")
    assert len(qualifying("\n".join(query.values()), records)) == 4
    assert [c[0] for c in binary_candidates(repo, query)] == [c.number for c in qualifying("\n".join(query.values()), records)[:3]]


def test_the_python_finder_lists_nothing_where_the_binary_lists_nothing(repo):
    records = repo_records(repo)
    for words in UNLISTED:
        query = as_query(words)
        assert binary_candidates(repo, query) == []
        assert python_candidates(records, query) == []


def test_the_python_finder_agrees_with_the_binary_on_every_test_control_in_the_plain_bundle(plain_repo):
    records = repo_records(plain_repo)
    assert len(list((plain_repo / "docs" / "adr").glob("[0-9][0-9][0-9][0-9]-*.md"))) == 60
    controls = [json.loads(line) for line in (FIXTURE / "controls.jsonl").read_text().splitlines()]
    tests = [c for c in controls if c["split"] == "test"]
    assert len(tests) == 20
    for control in tests:
        assert python_candidates(records, control["query"]) == binary_candidates(plain_repo, control["query"]), control["id"]


def test_words_shared_only_with_a_title_are_listed_by_both(repo):
    query = as_query("vicuna capybara")
    expected = binary_candidates(repo, query)
    assert [c[2] for c in expected] == [("capybara", "vicuna")]
    assert python_candidates(repo_records(repo), query) == expected


def test_a_missing_binary_fails_naming_the_build_target(monkeypatch):
    monkeypatch.setenv("LIVING_DOCS_BIN", "/nonexistent/living-docs")
    with pytest.raises(pytest.fail.Exception, match="make build"):
        binary()
