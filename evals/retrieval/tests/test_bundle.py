import tempfile

import pytest

from bundle import Plant, build_scale_repo, bundle_records, scale_setup_records

PLANT = Plant(
    "Zebra crossing rota",
    "Quokka handlers swap the walrus shift.",
    "Handlers asked for a fair rota.",
    "rotate the walrus shift between quokka handlers",
)


@pytest.fixture(scope="module")
def reference():
    return {
        (60, "early"): scale_setup_records(60, "early", PLANT),
        (240, "late"): scale_setup_records(240, "late", PLANT),
        (60, None): scale_setup_records(60),
    }


def test_the_plant_sits_at_its_position_with_its_texts(reference):
    early = {adr.number: adr for adr in reference[(60, "early")]}[3]
    late = {adr.number: adr for adr in reference[(240, "late")]}[238]
    for planted in (early, late):
        assert planted.title == PLANT.title
        assert planted.description == PLANT.description
        assert PLANT.context in planted.body
        assert f"We will {PLANT.decision}." in planted.body


def test_superseded_and_deprecated_records_are_left_out(reference):
    assert len(reference[(60, "early")]) == 60 - 60 // 8 - 60 // 12
    assert len(reference[(240, "late")]) == 240 - 240 // 8 - 240 // 12


def test_a_bundle_without_a_plant_holds_only_filler(reference):
    assert all(adr.title != PLANT.title for adr in reference[(60, None)])
    assert len(reference[(60, None)]) == 60 - 60 // 8 - 60 // 12


def test_the_same_inputs_return_the_same_records_twice():
    assert bundle_records(60, "early", PLANT) == bundle_records(60, "early", PLANT)


def test_building_without_a_scratch_repository_returns_what_the_setup_leaves(reference):
    assert bundle_records(60, "early", PLANT) == reference[(60, "early")]
    assert bundle_records(240, "late", PLANT) == reference[(240, "late")]
    assert bundle_records(60) == reference[(60, None)]


@pytest.fixture
def private_tmp(tmp_path, monkeypatch):
    private = tmp_path / "private"
    private.mkdir()
    monkeypatch.setenv("TMPDIR", str(private))
    monkeypatch.setattr(tempfile, "tempdir", None)
    return private


def test_no_scratch_directory_is_left_after_a_build(private_tmp):
    assert scale_setup_records(60, "early", PLANT)
    assert list(private_tmp.iterdir()) == []


def test_no_scratch_directory_is_left_when_a_build_fails_partway(private_tmp):
    parent = private_tmp / "parent"
    parent.mkdir()
    existing = parent / "repo"
    existing.mkdir()
    with pytest.raises(FileExistsError):
        build_scale_repo(existing, 60)
    assert list(parent.iterdir()) == [existing]
    assert list(private_tmp.iterdir()) == [parent]


def test_no_scratch_directory_is_left_when_the_binary_is_missing(private_tmp, monkeypatch):
    monkeypatch.setenv("LIVING_DOCS_BIN", "/nonexistent/living-docs")
    with pytest.raises(FileNotFoundError):
        scale_setup_records(60)
    assert list(private_tmp.iterdir()) == []
