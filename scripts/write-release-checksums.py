#!/usr/bin/env python3
"""Write SHA256SUMS for validated Mallang release archive names."""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import re


SUPPORTED_TARGETS = {
    "aarch64-apple-darwin",
    "x86_64-unknown-linux-gnu",
}
ARCHIVE_RE = re.compile(
    r"mallang-v(?P<version>[0-9]+\.[0-9]+\.[0-9]+"
    r"(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?)-"
    r"(?P<target>aarch64-apple-darwin|x86_64-unknown-linux-gnu)\.tar\.gz"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--require-all-targets", action="store_true")
    parser.add_argument("archives", nargs="+", type=Path)
    return parser.parse_args()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main() -> None:
    args = parse_args()
    archives: list[tuple[str, str, Path]] = []
    versions: set[str] = set()
    targets: set[str] = set()
    names: set[str] = set()

    for raw_path in args.archives:
        path = raw_path.resolve()
        if not path.is_file() or path.is_symlink():
            raise SystemExit(f"release archive is not a regular file: {raw_path}")
        match = ARCHIVE_RE.fullmatch(path.name)
        if match is None:
            raise SystemExit(f"invalid release archive filename: {path.name}")
        version = match.group("version")
        target = match.group("target")
        if path.name in names:
            raise SystemExit(f"duplicate release archive filename: {path.name}")
        if target in targets:
            raise SystemExit(f"duplicate release target: {target}")
        names.add(path.name)
        versions.add(version)
        targets.add(target)
        archives.append((path.name, sha256(path), path))

    if len(versions) != 1:
        raise SystemExit(f"release archives have mismatched versions: {sorted(versions)}")
    if args.require_all_targets and targets != SUPPORTED_TARGETS:
        missing = sorted(SUPPORTED_TARGETS - targets)
        extra = sorted(targets - SUPPORTED_TARGETS)
        raise SystemExit(f"release target set mismatch: missing={missing}, extra={extra}")

    output = args.output.resolve()
    if output.name != "SHA256SUMS":
        raise SystemExit(f"checksum filename must be SHA256SUMS, got {output.name}")
    output.parent.mkdir(parents=True, exist_ok=True)
    contents = "".join(
        f"{digest}  {name}\n" for name, digest, _ in sorted(archives)
    )
    output.write_text(contents, encoding="ascii", newline="\n")


if __name__ == "__main__":
    main()
