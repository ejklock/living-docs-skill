"""Seed bundles of the scale eval, as lists of in-force ADR records.

`bundle_records` composes the records from the scale manifest alone, which is
fast enough to lint a fixture across many plants; `scale_setup_records` builds
the same bundle with the real CLI and reads the files it leaves, and a test
holds the two equal.
"""

import json
import os
import re
import shutil
import subprocess
import tempfile
from collections.abc import Iterator
from contextlib import contextmanager
from pathlib import Path
from typing import NamedTuple

from rare_words import Adr

ROOT = Path(__file__).resolve().parents[2]
SCALE = ROOT / "evals" / "scale"
SCENARIOS = ROOT / "evals" / "scenarios"
POSITIONS = ("early", "late")

_CONSEQUENCES = """## Consequences

**Easier / gained:**
- One agreed answer.

**Harder / accepted trade-offs:**
- Changing it later means a new decision."""


class Plant(NamedTuple):
    title: str
    description: str
    context: str
    decision: str


def plant_number(size: int, position: str) -> int:
    """The ADR number the scale eval gives a plant: 3 when early, size - 2 when late."""
    if position not in POSITIONS:
        raise ValueError(f"position must be one of {POSITIONS}, got {position!r}")
    return 3 if position == "early" else size - 2


def manifest_lines(size: int, position: str | None, plant: Plant | None, seed: int) -> list[list[str]]:
    """The tab-split lines `manifest.awk` prints for these inputs."""
    if (position is None) != (plant is None):
        raise ValueError("a position and a plant go together")
    pos = 0 if plant is None else plant_number(size, position)
    command = [
        "awk", "-v", f"seed={seed}", "-v", f"size={size}", "-v", f"plant_pos={pos}",
        "-v", "plant=" + ("\t".join(plant) if plant else ""),
        "-f", str(SCALE / "manifest.awk"), str(SCALE / "concerns.tsv"), str(SCALE / "components.txt"),
    ]  # fmt: skip
    done = subprocess.run(command, capture_output=True, text=True, check=True, env={**os.environ, "LC_ALL": "C"})
    return [line.split("\t") for line in done.stdout.splitlines()]


def adr_body(context: str, decision: str) -> str:
    """The body `scale_adr` writes."""
    return f"## Context\n\n{context}\n\n## Decision\n\nWe will {decision}.\n\n{_CONSEQUENCES}"


def bundle_records(size: int, position: str | None = None, plant: Plant | None = None, seed: int = 153) -> list[Adr]:
    """The in-force ADRs of the seed bundle, without running the CLI."""
    lines = manifest_lines(size, position, plant, seed)
    retired = {int(line[1]) for line in lines if line[0] == "deprecate"}
    retired |= {int(line[1]) for line in lines if line[0] == "supersede"}
    return [
        Adr(int(n), title, description, adr_body(context, decision))
        for op, n, title, description, context, decision in (line for line in lines if line[0] == "adr")
        if int(n) not in retired
    ]  # fmt: skip


def _scalar(raw: str) -> str:
    raw = raw.strip()
    if raw.startswith('"'):
        return json.loads(raw)
    if raw.startswith("'"):
        return raw[1:-1].replace("''", "'")
    return raw


def read_adr_file(path: Path) -> tuple[Adr, str]:
    """The record a file holds and its status."""
    head, _, rest = path.read_text().removeprefix("---\n").partition("\n---\n")
    fields = dict(re.findall(r"^(\w+): (.*)$", head, re.M))
    number = int(path.name[:4])
    body = re.sub(r"\A\s*# \d+\. .*\n", "", rest).strip()
    adr = Adr(number, _scalar(fields["title"]), _scalar(fields["description"]), body)
    return adr, _scalar(fields["status"])


def living_docs_binary() -> Path:
    """The binary named by LIVING_DOCS_BIN, `target/release/living-docs` by default."""
    binary = Path(os.environ.get("LIVING_DOCS_BIN", "target/release/living-docs"))
    binary = binary if binary.is_absolute() else ROOT / binary
    if not binary.is_file():
        raise FileNotFoundError(f"{binary} is missing: run `make build` first")
    return binary


@contextmanager
def scratch_dir(parent: Path | None = None) -> Iterator[Path]:
    """A new empty directory under `parent` (default the temporary directory), removed on exit."""
    path = Path(tempfile.mkdtemp(dir=parent))
    try:
        yield path
    finally:
        shutil.rmtree(path)


def build_scale_repo(dest: Path, size: int, position: str | None = None, plant: Plant | None = None, seed: int = 153) -> None:
    """Run `scale_setup` into `dest`, a new directory, with the binary named by LIVING_DOCS_BIN."""
    binary = living_docs_binary()
    with scratch_dir(dest.parent) as scratch:
        (scratch / "bin").mkdir()
        (scratch / "bin" / "living-docs").symlink_to(binary)
        scenario = scratch / "scenario"
        scenario.mkdir()
        shutil.copy(SCENARIOS / "common.sh", scratch)
        shutil.copy(SCALE / "lib.sh", scratch)
        lines = manifest_lines(size, position, plant, seed)
        (scenario / "manifest.tsv").write_text("".join("\t".join(line) + "\n" for line in lines))
        dest.mkdir()
        env = {**os.environ, "PATH": f"{scratch / 'bin'}:{os.environ['PATH']}", "LC_ALL": "C"}
        env |= {"GIT_AUTHOR_NAME": "eval", "GIT_AUTHOR_EMAIL": "eval@example.test"}
        env |= {"GIT_COMMITTER_NAME": "eval", "GIT_COMMITTER_EMAIL": "eval@example.test"}
        script = f'set -euo pipefail; . "{scratch}/lib.sh"; git init -q; scale_setup "{scenario}"'
        subprocess.run(["bash", "-c", script], cwd=dest, env=env, check=True, capture_output=True, text=True)


def repo_records(repo: Path) -> list[Adr]:
    """The in-force ADRs a repository holds under docs/adr."""
    read = (read_adr_file(path) for path in sorted((repo / "docs" / "adr").glob("[0-9][0-9][0-9][0-9]-*.md")))
    return [adr for adr, status in read if status not in ("Superseded", "Deprecated")]


def scale_setup_records(size: int, position: str | None = None, plant: Plant | None = None, seed: int = 153) -> list[Adr]:
    """The in-force ADRs `scale_setup` leaves in a scratch repository, removed afterwards."""
    with scratch_dir() as parent:
        repo = parent / "repo"
        build_scale_repo(repo, size, position, plant, seed)
        return repo_records(repo)
