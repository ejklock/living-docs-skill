"""Generates a synthetic Living Docs bundle for scaling measurements.

Usage: gen-bundle.py <example docs dir> <output dir> <records per type>

Copies the example bundle, then writes that many ADRs and issues from its
templates; every tenth ADR is superseded by the next. Run `living-docs index`
and `living-docs fmt` on the output before measuring, so `check` passes.
"""

import re
import shutil
import sys
from pathlib import Path

src = Path(sys.argv[1])
out = Path(sys.argv[2])
per_type = int(sys.argv[3])

shutil.rmtree(out, ignore_errors=True)
shutil.copytree(src, out)
adr_tpl = (src / "adr/0002-sqlite-store.md").read_text()
issue_tpl = (src / "issues/0001-implement-shorten-endpoint.md").read_text()
for p in list((out / "adr").glob("0*.md")) + list((out / "issues").glob("0*.md")):
    p.unlink()


def adr(n):
    title = f"Decision number {n} about storage topic {n % 37}"
    text = adr_tpl.replace("title: SQLite store for minted links", f"title: {title}")
    text = text.replace("# 0002. SQLite store for minted links", f"# {n:04d}. {title}")
    text = re.sub(r"supersedes: 0001\n", "", text)
    if n % 10 == 1 and n < per_type:
        text = text.replace("status: Accepted\n", f"status: Superseded\nsuperseded_by: {n + 1:04d}\n")
    if n % 10 == 2:
        text = text.replace("status: Accepted\n", f"status: Accepted\nsupersedes: {n - 1:04d}\n")
    return text


def issue(n):
    title = f"Work item {n} for area {n % 23}"
    text = issue_tpl.replace("title: Implement shorten + redirect endpoints", f"title: {title}")
    text = text.replace("## Implement shorten + redirect endpoints", f"## {title}")
    return text.replace("status: open", "status: closed" if n % 3 == 0 else "status: open")


for n in range(1, per_type + 1):
    (out / "adr" / f"{n:04d}-decision-{n}.md").write_text(adr(n))
    (out / "issues" / f"{n:04d}-work-item-{n}.md").write_text(issue(n))

renames = {
    "0001-in-memory-store.md": "0001-decision-1.md",
    "0002-sqlite-store.md": "0002-decision-2.md",
    "0001-implement-shorten-endpoint.md": "0001-work-item-1.md",
}
for p in out.rglob("*.md"):
    text = p.read_text()
    for old, new in renames.items():
        text = text.replace(old, new)
    p.write_text(text)
