#!/usr/bin/env python3
"""Validate the canonical state of repository work packets."""

from __future__ import annotations

import re
import sys
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
INDEX = DOCS / "TODO_INDEX.md"
ALLOWED_STATUSES = {"active", "complete", "deferred", "decision-required"}
STATUS_RE = re.compile(r"^Status: ([a-z-]+)(?:; .+)?$")
TODO_LINK_RE = re.compile(r"\((todo-[^)]+/spec\.md)\)")


def fail(errors: list[str]) -> None:
    for error in errors:
        print(f"todo state error: {error}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    errors: list[str] = []
    statuses: dict[str, str] = {}

    for todo_dir in sorted(DOCS.glob("todo-*")):
        if not todo_dir.is_dir():
            continue
        relative_dir = todo_dir.relative_to(ROOT).as_posix()
        spec = todo_dir / "spec.md"
        if not spec.is_file():
            errors.append(f"{relative_dir} is missing spec.md")
            continue

        lines = spec.read_text(encoding="utf-8").splitlines()
        status_line = next((line for line in lines[:8] if line.startswith("Status:")), None)
        if status_line is None:
            errors.append(f"{relative_dir}/spec.md is missing a Status line")
            continue
        match = STATUS_RE.fullmatch(status_line)
        if match is None or match.group(1) not in ALLOWED_STATUSES:
            errors.append(
                f"{relative_dir}/spec.md has invalid status `{status_line}`; "
                f"expected one of {sorted(ALLOWED_STATUSES)}"
            )
            continue

        status = match.group(1)
        statuses[relative_dir] = status
        if status != "complete" and not (todo_dir / "open-questions.md").is_file():
            errors.append(f"{relative_dir} with status {status} is missing open-questions.md")
        if status == "complete":
            if any(line.startswith("- [ ]") for line in lines):
                errors.append(f"{relative_dir}/spec.md is complete but has unchecked tasks")
            if any(re.match(r"^\| C[^|]*\| (todo|pending|blocked|in progress) \|", line) for line in lines):
                errors.append(f"{relative_dir}/spec.md is complete but has an unfinished checklist row")

    if not INDEX.is_file():
        errors.append("docs/TODO_INDEX.md is missing")
        fail(errors)

    index_text = INDEX.read_text(encoding="utf-8")
    indexed_specs = set(TODO_LINK_RE.findall(index_text))
    for relative_path in indexed_specs:
        if not (DOCS / relative_path).is_file():
            errors.append(f"docs/TODO_INDEX.md links missing {relative_path}")

    for relative_dir, status in statuses.items():
        if status == "complete":
            continue
        index_path = f"{Path(relative_dir).name}/spec.md"
        if index_path not in indexed_specs:
            errors.append(
                f"{relative_dir}/spec.md has status {status} but is absent from docs/TODO_INDEX.md"
            )

    if errors:
        fail(errors)

    counts = Counter(statuses.values())
    summary = ", ".join(f"{status}={counts[status]}" for status in sorted(counts))
    print(f"todo state: {len(statuses)} specs ({summary})")


if __name__ == "__main__":
    main()
