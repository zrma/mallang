#!/usr/bin/env python3
"""Measure the observational large-project Mallang check baseline."""

from __future__ import annotations

import argparse
from datetime import date
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
SCHEMA = "mallang.self-hosted-check-performance.v1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="measure Stage0 and self-hosted check latency"
    )
    parser.add_argument(
        "--compiler",
        type=Path,
        default=Path("target/release/mlg"),
        help="mlg driver path, relative to the repository root by default",
    )
    parser.add_argument(
        "--input",
        type=Path,
        default=Path("bootstrap/compiler"),
        help="project input, relative to the repository root by default",
    )
    parser.add_argument("--iterations", type=int, default=7)
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--max-self-ms", type=float)
    parser.add_argument("--output", type=Path)
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else ROOT / path


def run(command: list[str]) -> subprocess.CompletedProcess[bytes]:
    try:
        return subprocess.run(
            command,
            cwd=ROOT,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except subprocess.CalledProcessError as error:
        stderr = error.stderr.decode("utf-8", errors="replace").strip()
        raise SystemExit(
            f"command failed ({error.returncode}): {' '.join(command)}\n{stderr}"
        ) from error


def measure(
    compiler: Path,
    input_arg: str,
    implementation: str,
    warmups: int,
    iterations: int,
) -> dict[str, Any]:
    command = [
        str(compiler),
        "--compiler",
        implementation,
        "check",
        input_arg,
    ]
    expected_stdout = f"{input_arg}: ok\n".encode()
    for _ in range(warmups):
        completed = run(command)
        require_clean_result(completed, expected_stdout, implementation)

    samples = []
    for _ in range(iterations):
        started = time.perf_counter_ns()
        completed = run(command)
        elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000
        require_clean_result(completed, expected_stdout, implementation)
        samples.append(elapsed_ms)

    return {
        "implementation": implementation,
        "wall_ms_median": round(statistics.median(samples), 3),
        "wall_ms_min": round(min(samples), 3),
        "wall_ms_max": round(max(samples), 3),
        "samples_ms": [round(sample, 3) for sample in samples],
    }


def require_clean_result(
    completed: subprocess.CompletedProcess[bytes],
    expected_stdout: bytes,
    implementation: str,
) -> None:
    if completed.stdout != expected_stdout:
        raise SystemExit(f"{implementation} check stdout changed")
    if completed.stderr:
        raise SystemExit(f"{implementation} check emitted stderr")


def source_inventory(project: Path) -> dict[str, int]:
    source_root = project / "src"
    sources = sorted(source_root.rglob("*.mlg"))
    if not sources:
        raise SystemExit(f"project has no Mallang sources: {project}")
    return {
        "files": len(sources),
        "lines": sum(path.read_bytes().count(b"\n") for path in sources),
        "bytes": sum(path.stat().st_size for path in sources),
    }


def main() -> int:
    args = parse_args()
    if args.iterations < 1 or args.warmups < 0:
        raise SystemExit("iterations must be positive and warmups must be non-negative")
    if args.max_self_ms is not None and args.max_self_ms <= 0:
        raise SystemExit("--max-self-ms must be positive")

    compiler = resolve(args.compiler)
    project = resolve(args.input)
    if not compiler.is_file() or not os.access(compiler, os.X_OK):
        raise SystemExit(f"compiler is not executable: {args.compiler}")
    if not project.is_dir():
        raise SystemExit(f"input is not a project directory: {args.input}")

    input_arg = os.path.relpath(project, ROOT)
    version = run([str(compiler), "--version", "--verbose"])
    if version.stderr:
        raise SystemExit("compiler version command emitted stderr")

    results = [
        measure(
            compiler,
            input_arg,
            implementation,
            args.warmups,
            args.iterations,
        )
        for implementation in ("stage0", "self")
    ]
    payload = {
        "schema": SCHEMA,
        "policy": "observational",
        "measurement": {
            "date": date.today().isoformat(),
            "compiler": version.stdout.decode("utf-8").strip().splitlines()[0],
            "profile": "release",
            "clock": "perf_counter_ns",
            "warmups": args.warmups,
            "iterations": args.iterations,
            "host": {
                "os": platform.system().lower(),
                "architecture": platform.machine().lower(),
            },
        },
        "input": {
            "path": input_arg,
            **source_inventory(project),
        },
        "results": results,
    }
    output = json.dumps(payload, indent=2) + "\n"
    if args.output is None:
        sys.stdout.write(output)
    else:
        resolve(args.output).write_text(output, encoding="utf-8")

    self_median = next(
        result["wall_ms_median"]
        for result in results
        if result["implementation"] == "self"
    )
    if args.max_self_ms is not None and self_median > args.max_self_ms:
        raise SystemExit(
            f"self-hosted check median {self_median:.3f} ms exceeds "
            f"{args.max_self_ms:.3f} ms"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
