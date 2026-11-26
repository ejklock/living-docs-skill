from pathlib import Path

from lint_fixture import main


def test_the_committed_fixture_passes_the_lint():
    assert main([str(Path(__file__).resolve().parents[1] / "fixture")]) == 0
