#!/usr/bin/env python3
"""Measure the observational v0.8 compiler and runtime baseline."""

from __future__ import annotations

import argparse
from datetime import date
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import tempfile
import time
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
SCHEMA = "mallang.v08.performance-baseline.v1"
CASE_IDS = (
    "minimal-standalone",
    "cleanup-heavy-standalone",
    "local-dependency-project",
    "standard-library-cli",
)

CASES = (
    {
        "id": CASE_IDS[0],
        "input": "examples/first.mlg",
        "generated_c": "target/mallang/first.c",
    },
    {
        "id": CASE_IDS[1],
        "input": "examples/full-expression-cleanup.mlg",
        "generated_c": "target/mallang/full-expression-cleanup.c",
        "runtime_args": (),
    },
    {
        "id": CASE_IDS[2],
        "input": "examples/projects/local-deps/app",
        "generated_c": "examples/projects/local-deps/app/target/mallang/pathapp.c",
    },
    {
        "id": CASE_IDS[3],
        "input": "examples/projects/textstats",
        "generated_c": "examples/projects/textstats/target/mallang/textstats.c",
        "runtime_args": ("tests/fixtures/v06-reference-cli/input.txt",),
    },
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="measure or validate the observational Mallang v0.8 baseline"
    )
    parser.add_argument(
        "--compiler",
        type=Path,
        default=Path("target/release/mlg"),
        help="compiler executable, relative to the repository root by default",
    )
    parser.add_argument("--iterations", type=int, default=7)
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--check-baseline", type=Path)
    return parser.parse_args()


def fail(message: str) -> None:
    raise SystemExit(message)


def resolve_from_root(path: Path) -> Path:
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
        fail(f"command failed ({error.returncode}): {' '.join(command)}\n{stderr}")


def timed_run(command: list[str]) -> tuple[float, subprocess.CompletedProcess[bytes]]:
    started = time.perf_counter_ns()
    completed = run(command)
    elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000
    return elapsed_ms, completed


def median_ms(samples: list[float]) -> float:
    return round(statistics.median(samples), 3)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def normalized_os() -> str:
    names = {"Darwin": "macos", "Linux": "linux", "Windows": "windows"}
    return names.get(platform.system(), platform.system().lower())


def normalized_architecture() -> str:
    names = {"arm64": "aarch64", "AMD64": "x86_64", "x86_64": "x86_64"}
    return names.get(platform.machine(), platform.machine().lower())


def measure_case(
    compiler: Path,
    case: dict[str, Any],
    iterations: int,
    warmups: int,
    output_dir: Path,
) -> dict[str, Any]:
    case_id = case["id"]
    input_path = case["input"]
    generated_path = ROOT / case["generated_c"]
    binary_path = output_dir / case_id
    check_command = [str(compiler), "check", input_path]
    build_command = [str(compiler), "build", input_path, "-o", str(binary_path)]

    for _ in range(warmups):
        run(check_command)
        run(build_command)

    check_samples = [timed_run(check_command)[0] for _ in range(iterations)]
    build_samples: list[float] = []
    generated_hashes: set[str] = set()
    for _ in range(iterations):
        elapsed_ms, _ = timed_run(build_command)
        build_samples.append(elapsed_ms)
        if not generated_path.is_file():
            fail(f"compiler did not produce {case['generated_c']}")
        generated_hashes.add(sha256_bytes(generated_path.read_bytes()))

    if len(generated_hashes) != 1:
        fail(f"generated C changed between repeated builds for {case_id}")
    if not binary_path.is_file():
        fail(f"compiler did not produce a native binary for {case_id}")

    metrics: dict[str, Any] = {
        "check_wall_ms_median": median_ms(check_samples),
        "build_wall_ms_median": median_ms(build_samples),
        "generated_c_bytes": generated_path.stat().st_size,
        "generated_c_sha256": next(iter(generated_hashes)),
        "native_binary_bytes": binary_path.stat().st_size,
    }

    if "runtime_args" in case:
        runtime_command = [str(binary_path), *case["runtime_args"]]
        expected_stdout: bytes | None = None
        for _ in range(warmups):
            completed = run(runtime_command)
            if completed.stderr:
                fail(f"runtime emitted stderr during warmup for {case_id}")
            expected_stdout = completed.stdout

        runtime_samples: list[float] = []
        for _ in range(iterations):
            elapsed_ms, completed = timed_run(runtime_command)
            if completed.stderr:
                fail(f"runtime emitted stderr for {case_id}")
            if expected_stdout is None:
                expected_stdout = completed.stdout
            elif completed.stdout != expected_stdout:
                fail(f"runtime output changed between repeated runs for {case_id}")
            runtime_samples.append(elapsed_ms)

        if expected_stdout is None:
            fail(f"runtime produced no output sample for {case_id}")
        try:
            stdout_text = expected_stdout.decode("utf-8")
        except UnicodeDecodeError:
            fail(f"runtime output is not UTF-8 for {case_id}")
        metrics.update(
            {
                "runtime_wall_ms_median": median_ms(runtime_samples),
                "stdout_sha256": sha256_bytes(expected_stdout),
                "stdout_utf8": stdout_text,
            }
        )

    return {
        "id": case_id,
        "input": input_path,
        "metrics": metrics,
    }


def measure(args: argparse.Namespace) -> dict[str, Any]:
    if args.iterations < 1 or args.warmups < 0:
        fail("iterations must be positive and warmups must be non-negative")
    if args.output is None:
        fail("--output is required when measuring a baseline")

    compiler = resolve_from_root(args.compiler)
    if not compiler.is_file() or not os.access(compiler, os.X_OK):
        fail(f"compiler is not executable: {args.compiler}")
    compiler_version = run([str(compiler), "--version"]).stdout.decode("utf-8").strip()

    with tempfile.TemporaryDirectory(prefix="mallang-v08-baseline-") as temporary:
        output_dir = Path(temporary)
        cases = [
            measure_case(compiler, case, args.iterations, args.warmups, output_dir)
            for case in CASES
        ]

    return {
        "schema": SCHEMA,
        "policy": {
            "mode": "observational",
            "regression_thresholds": None,
            "decision_gate": "set thresholds only after supported-platform variance review",
        },
        "measurement": {
            "date": date.today().isoformat(),
            "compiler": compiler_version,
            "compiler_profile": "release",
            "clock": "perf_counter_ns",
            "iterations": args.iterations,
            "warmups": args.warmups,
            "host": {
                "os": normalized_os(),
                "architecture": normalized_architecture(),
            },
        },
        "cases": cases,
        "reproducibility": {
            "generated_c": {
                "status": "pass",
                "scope": "same compiler, input, options, and host",
            },
            "release_archive": {
                "status": "covered-by-gate",
                "command": "scripts/check-release-artifacts.sh",
            },
            "native_executable": {
                "status": "excluded",
                "reason": "host C compiler and toolchain output is outside the byte-identity contract",
            },
        },
    }


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(f"invalid v0.8 baseline: {message}")


def validate_baseline(document: Any) -> None:
    require(isinstance(document, dict), "root must be an object")
    require(document.get("schema") == SCHEMA, "schema mismatch")
    policy = document.get("policy")
    require(isinstance(policy, dict), "policy must be an object")
    require(policy.get("mode") == "observational", "policy must be observational")
    require(policy.get("regression_thresholds") is None, "thresholds must remain unset")

    measurement = document.get("measurement")
    require(isinstance(measurement, dict), "measurement must be an object")
    require(isinstance(measurement.get("compiler"), str), "compiler version is required")
    require(measurement.get("compiler_profile") == "release", "release profile is required")
    require(isinstance(measurement.get("iterations"), int), "iterations must be an integer")
    require(measurement.get("iterations", 0) > 0, "iterations must be positive")
    require(isinstance(measurement.get("warmups"), int), "warmups must be an integer")
    require(measurement.get("warmups", -1) >= 0, "warmups must be non-negative")
    host = measurement.get("host")
    require(isinstance(host, dict), "host must be an object")
    require(set(host) == {"os", "architecture"}, "host may only identify OS and architecture")

    cases = document.get("cases")
    require(isinstance(cases, list), "cases must be an array")
    require(all(isinstance(case, dict) for case in cases), "each case must be an object")
    require(tuple(case.get("id") for case in cases) == CASE_IDS, "case set or order mismatch")
    expected_inputs = tuple(case["input"] for case in CASES)
    require(tuple(case.get("input") for case in cases) == expected_inputs, "case input mismatch")
    for case in cases:
        metrics = case.get("metrics")
        require(isinstance(metrics, dict), f"{case.get('id')} metrics must be an object")
        for name in (
            "check_wall_ms_median",
            "build_wall_ms_median",
            "generated_c_bytes",
            "native_binary_bytes",
        ):
            value = metrics.get(name)
            require(
                isinstance(value, (int, float)) and value > 0,
                f"{case.get('id')} {name} must be positive",
            )
        digest = metrics.get("generated_c_sha256")
        require(
            isinstance(digest, str) and len(digest) == 64,
            f"{case.get('id')} generated C hash is invalid",
        )

    for case in (cases[1], cases[3]):
        metrics = case["metrics"]
        require(
            metrics.get("runtime_wall_ms_median", 0) > 0,
            f"{case['id']} runtime median is required",
        )
        require(
            isinstance(metrics.get("stdout_utf8"), str),
            f"{case['id']} runtime output is required",
        )
        require(
            len(metrics.get("stdout_sha256", "")) == 64,
            f"{case['id']} runtime hash is invalid",
        )

    reproducibility = document.get("reproducibility")
    require(isinstance(reproducibility, dict), "reproducibility must be an object")
    require(
        reproducibility.get("generated_c", {}).get("status") == "pass",
        "generated C identity must pass",
    )
    require(
        reproducibility.get("release_archive", {}).get("status") == "covered-by-gate",
        "release archive identity gate status mismatch",
    )
    require(
        reproducibility.get("release_archive", {}).get("command")
        == "scripts/check-release-artifacts.sh",
        "release archive gate mismatch",
    )
    require(
        reproducibility.get("native_executable", {}).get("status") == "excluded",
        "native executable identity must remain excluded",
    )


def main() -> None:
    args = parse_args()
    if args.check_baseline is not None:
        if args.output is not None:
            fail("--output and --check-baseline cannot be used together")
        baseline_path = resolve_from_root(args.check_baseline)
        try:
            document = json.loads(baseline_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            fail(f"failed to read baseline: {error}")
        validate_baseline(document)
        print("v0.8 observational baseline contract passed")
        return

    document = measure(args)
    validate_baseline(document)
    output_path = resolve_from_root(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(document, ensure_ascii=True, indent=2, sort_keys=False) + "\n",
        encoding="utf-8",
    )
    print(args.output)


if __name__ == "__main__":
    main()
