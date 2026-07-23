#!/usr/bin/env python3
"""Create a deterministic Mallang release archive from an exact staging tree."""

from __future__ import annotations

import argparse
import gzip
import io
import os
from pathlib import Path, PurePosixPath
import re
import tarfile
import tempfile


ROOT_NAME_RE = re.compile(
    r"mallang-v[0-9]+\.[0-9]+\.[0-9]+"
    r"(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?-"
    r"(?:aarch64-apple-darwin|x86_64-unknown-linux-gnu)"
)
FILES = (
    (PurePosixPath("bin/mlg"), 0o755),
    (PurePosixPath("bin/mlgc"), 0o755),
    (PurePosixPath("LICENSE-MIT"), 0o644),
    (PurePosixPath("LICENSE-APACHE"), 0o644),
    (PurePosixPath("README.md"), 0o644),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-dir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--root-name", required=True)
    return parser.parse_args()


def tar_info(name: str, mode: int, *, directory: bool, size: int = 0) -> tarfile.TarInfo:
    info = tarfile.TarInfo(name)
    info.type = tarfile.DIRTYPE if directory else tarfile.REGTYPE
    info.mode = mode
    info.size = size
    info.mtime = 0
    info.uid = 0
    info.gid = 0
    info.uname = "root"
    info.gname = "root"
    return info


def validate_source(source_dir: Path) -> None:
    if not source_dir.is_dir() or source_dir.is_symlink():
        raise SystemExit(f"release staging directory is invalid: {source_dir}")

    expected = {Path(*relative.parts) for relative, _ in FILES}
    actual: set[Path] = set()
    for path in source_dir.rglob("*"):
        relative = path.relative_to(source_dir)
        if path.is_symlink():
            raise SystemExit(f"release staging tree contains a symlink: {relative}")
        if path.is_file():
            actual.add(relative)
        elif not path.is_dir():
            raise SystemExit(f"release staging tree contains a special file: {relative}")

    if actual != expected:
        missing = sorted(str(path) for path in expected - actual)
        extra = sorted(str(path) for path in actual - expected)
        raise SystemExit(f"release staging file set mismatch: missing={missing}, extra={extra}")


def write_archive(source_dir: Path, output: Path, root_name: str) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=output.parent, delete=False) as raw:
        temporary = Path(raw.name)
        try:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, compresslevel=9, mtime=0) as zipped:
                with tarfile.open(fileobj=zipped, mode="w", format=tarfile.USTAR_FORMAT) as archive:
                    archive.addfile(tar_info(f"{root_name}/", 0o755, directory=True))
                    archive.addfile(tar_info(f"{root_name}/bin/", 0o755, directory=True))
                    for relative, mode in FILES:
                        data = (source_dir / Path(*relative.parts)).read_bytes()
                        info = tar_info(
                            str(PurePosixPath(root_name) / relative),
                            mode,
                            directory=False,
                            size=len(data),
                        )
                        archive.addfile(info, io.BytesIO(data))
            os.replace(temporary, output)
        except BaseException:
            temporary.unlink(missing_ok=True)
            raise


def main() -> None:
    args = parse_args()
    if not ROOT_NAME_RE.fullmatch(args.root_name):
        raise SystemExit(f"invalid release archive root name: {args.root_name}")

    source_dir = args.source_dir.resolve()
    output = args.output.resolve()
    if output.name != f"{args.root_name}.tar.gz":
        raise SystemExit(
            f"release archive filename must be {args.root_name}.tar.gz, got {output.name}"
        )

    validate_source(source_dir)
    write_archive(source_dir, output, args.root_name)


if __name__ == "__main__":
    main()
