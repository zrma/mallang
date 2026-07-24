#!/usr/bin/env python3
"""Validate active work packets and completed work artifacts."""

from __future__ import annotations

import os
import re
import sys
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
INDEX = DOCS / "TODO_INDEX.md"
ARTIFACTS = DOCS / "artifacts"
TODO_STATUSES = {"active", "deferred", "decision-required"}
STATUS_RE = re.compile(r"^Status: ([a-z-]+)(?:; .+)?$")
TODO_LINK_RE = re.compile(r"\((todo-[^)]+/spec\.md)\)")
WORK_ID_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
LEGACY_TODO_PATH_RE = re.compile(
    r"docs/todo-([a-z0-9]+(?:-[a-z0-9]+)*)"
)
ARTIFACT_PATH_RE = re.compile(
    r"docs/artifacts/([a-z0-9]+(?:-[a-z0-9]+)*)"
)
TEXT_SUFFIXES = {
    ".json",
    ".md",
    ".mlg",
    ".py",
    ".rs",
    ".sh",
    ".toml",
    ".yaml",
    ".yml",
}
SKIPPED_ROOTS = {".git", ".jj", "target"}


def fail(errors: list[str]) -> None:
    for error in errors:
        print(f"todo state error: {error}", file=sys.stderr)
    raise SystemExit(1)


def read_status(spec: Path, relative_spec: str, errors: list[str]) -> str | None:
    lines = spec.read_text(encoding="utf-8").splitlines()
    status_line = next((line for line in lines[:8] if line.startswith("Status:")), None)
    if status_line is None:
        errors.append(f"{relative_spec} is missing a Status line")
        return None
    match = STATUS_RE.fullmatch(status_line)
    if match is None:
        errors.append(f"{relative_spec} has invalid status `{status_line}`")
        return None
    return match.group(1)


def check_complete_spec(spec: Path, relative_spec: str, errors: list[str]) -> None:
    lines = spec.read_text(encoding="utf-8").splitlines()
    if any(line.startswith("- [ ]") for line in lines):
        errors.append(f"{relative_spec} is complete but has unchecked tasks")
    if any(
        re.match(r"^\| C[^|]*\| (todo|pending|blocked|in progress) \|", line)
        for line in lines
    ):
        errors.append(f"{relative_spec} is complete but has an unfinished checklist row")


def repository_text_files() -> list[Path]:
    paths = []
    for directory, directory_names, file_names in os.walk(ROOT):
        directory_names[:] = [
            name for name in directory_names if name not in SKIPPED_ROOTS
        ]
        parent = Path(directory)
        for file_name in file_names:
            path = parent / file_name
            if path.suffix in TEXT_SUFFIXES or path.name in {"AGENTS.md", "README.md"}:
                paths.append(path)
    return paths


def main() -> None:
    errors: list[str] = []
    todo_statuses: dict[str, str] = {}
    artifact_ids: set[str] = set()

    for todo_dir in sorted(DOCS.glob("todo-*")):
        if not todo_dir.is_dir():
            continue
        work_id = todo_dir.name.removeprefix("todo-")
        relative_dir = todo_dir.relative_to(ROOT).as_posix()
        if WORK_ID_RE.fullmatch(work_id) is None:
            errors.append(f"{relative_dir} has invalid work id {work_id}")
        spec = todo_dir / "spec.md"
        if not spec.is_file():
            errors.append(f"{relative_dir} is missing spec.md")
            continue

        relative_spec = f"{relative_dir}/spec.md"
        status = read_status(spec, relative_spec, errors)
        if status is None:
            continue
        if status not in TODO_STATUSES:
            errors.append(
                f"{relative_spec} has status {status}; completed work belongs in "
                f"docs/artifacts/{work_id}"
            )
            continue

        todo_statuses[relative_dir] = status
        if not (todo_dir / "open-questions.md").is_file():
            errors.append(f"{relative_dir} with status {status} is missing open-questions.md")

    if not ARTIFACTS.is_dir():
        errors.append("docs/artifacts is missing")
    else:
        for artifact_dir in sorted(ARTIFACTS.iterdir()):
            if not artifact_dir.is_dir():
                continue
            work_id = artifact_dir.name
            relative_dir = artifact_dir.relative_to(ROOT).as_posix()
            if WORK_ID_RE.fullmatch(work_id) is None:
                errors.append(f"{relative_dir} has invalid work id {work_id}")
            spec = artifact_dir / "spec.md"
            if not spec.is_file():
                errors.append(f"{relative_dir} is missing spec.md")
                continue
            if (DOCS / f"todo-{work_id}").exists():
                errors.append(f"work id {work_id} exists in both todo and artifacts")

            relative_spec = f"{relative_dir}/spec.md"
            status = read_status(spec, relative_spec, errors)
            if status is None:
                continue
            if status != "complete":
                errors.append(f"{relative_spec} has status {status}; expected complete")
                continue
            artifact_ids.add(work_id)
            check_complete_spec(spec, relative_spec, errors)

    if not INDEX.is_file():
        errors.append("docs/TODO_INDEX.md is missing")
        fail(errors)

    index_text = INDEX.read_text(encoding="utf-8")
    indexed_specs = set(TODO_LINK_RE.findall(index_text))
    for relative_path in indexed_specs:
        if not (DOCS / relative_path).is_file():
            errors.append(f"docs/TODO_INDEX.md links missing {relative_path}")

    for relative_dir, status in todo_statuses.items():
        index_path = f"{Path(relative_dir).name}/spec.md"
        if index_path not in indexed_specs:
            errors.append(
                f"{relative_dir}/spec.md has status {status} but is absent from docs/TODO_INDEX.md"
            )

    for path in repository_text_files():
        relative = path.relative_to(ROOT).as_posix()
        for line_number, line in enumerate(
            path.read_text(encoding="utf-8").splitlines(), start=1
        ):
            for match in LEGACY_TODO_PATH_RE.finditer(line):
                if match.group(1) in artifact_ids:
                    errors.append(
                        f"{relative}:{line_number} references completed work as "
                        f"docs/todo-{match.group(1)}"
                    )
            for match in ARTIFACT_PATH_RE.finditer(line):
                if match.group(1) not in artifact_ids:
                    errors.append(
                        f"{relative}:{line_number} references unknown artifact "
                        f"docs/artifacts/{match.group(1)}"
                    )

    if errors:
        fail(errors)

    counts = Counter(todo_statuses.values())
    summary = ", ".join(f"{status}={counts[status]}" for status in sorted(counts))
    print(
        f"work packet state: todo={len(todo_statuses)} ({summary}), "
        f"artifacts={len(artifact_ids)} (complete={len(artifact_ids)})"
    )


if __name__ == "__main__":
    main()
