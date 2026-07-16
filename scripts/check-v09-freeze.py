#!/usr/bin/env python3
"""Audit changes since v0.8.0 against the approved v0.9 freeze classes."""

from __future__ import annotations

import subprocess
import sys
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
BASE = "v0.8.0"

RELEASE_FILES = {
    ".github/workflows/ci.yml",
    "Cargo.lock",
    "Cargo.toml",
    "scripts/check-release-helpers.sh",
    "scripts/check-v09-acceptance.sh",
    "scripts/check-v09-freeze.py",
    "scripts/verify-v0-rc.sh",
}
CONFORMANCE_FILES = {
    "scripts/check-test-workflow.sh",
    "scripts/check-v1-conformance.py",
    "scripts/check-v1-migration.sh",
    "scripts/check.sh",
}


def git_paths(*args: str) -> set[str]:
    result = subprocess.run(
        ["git", *args],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
    )
    return {
        path.decode("utf-8")
        for path in result.stdout.split(b"\0")
        if path
    }


def classify(path: str) -> str | None:
    if path in {"README.md", "ROADMAP.md", "SPEC.md"} or path.startswith("docs/"):
        return "documentation"
    if path in RELEASE_FILES:
        return "release"
    if path in CONFORMANCE_FILES or path.startswith("tests/fixtures/v1-migration/"):
        return "conformance"
    if path.startswith("tests/fixtures/project-test-empty/"):
        return "conformance"
    if path == "scripts/check-v09-dogfood.sh" or path.startswith(
        "examples/projects/textstats/"
    ):
        return "dogfood"
    return None


def main() -> int:
    subprocess.run(
        ["git", "rev-parse", "--verify", f"refs/tags/{BASE}^{{commit}}"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    subprocess.run(
        ["git", "merge-base", "--is-ancestor", BASE, "HEAD"],
        cwd=ROOT,
        check=True,
    )

    changed = git_paths(
        "diff", "--name-only", "--diff-filter=ACDMRTUXB", "-z", BASE, "--"
    )
    changed.update(git_paths("ls-files", "--others", "--exclude-standard", "-z"))
    if not changed:
        print(f"v0.9 freeze audit found no changes after {BASE}", file=sys.stderr)
        return 1

    compiler_changes = sorted(path for path in changed if path.startswith("src/"))
    if compiler_changes:
        print("v0.9 freeze audit found compiler source changes:", file=sys.stderr)
        for path in compiler_changes:
            print(f"  {path}", file=sys.stderr)
        return 1

    unclassified = sorted(path for path in changed if classify(path) is None)
    if unclassified:
        print("v0.9 freeze audit found unclassified changes:", file=sys.stderr)
        for path in unclassified:
            print(f"  {path}", file=sys.stderr)
        return 1

    counts = Counter(classify(path) for path in changed)
    summary = " ".join(f"{name}={counts[name]}" for name in sorted(counts))
    print(
        f"v0.9 freeze audit passed: compiler=0 changed={len(changed)} {summary}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
