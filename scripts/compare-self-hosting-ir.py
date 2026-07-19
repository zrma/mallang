#!/usr/bin/env python3

import argparse
import difflib
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class NormalizedIr:
    header: str
    order: list[str]
    functions: dict[str, list[str]]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Compare normalized self-hosting IR one function at a time."
    )
    parser.add_argument("expected", type=Path)
    parser.add_argument("actual", type=Path)
    parser.add_argument("--max-diff-lines", type=int, default=80)
    return parser.parse_args()


def read_normalized_ir(path: Path) -> NormalizedIr:
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    if not lines or not lines[0].startswith("IR|"):
        raise ValueError(f"{path}: normalized IR must start with IR|")

    order: list[str] = []
    functions: dict[str, list[str]] = {}
    current_name = ""
    for line in lines[1:]:
        if line.startswith("FUNCTION|"):
            fields = line.split("|", 2)
            if len(fields) < 3 or not fields[1]:
                raise ValueError(f"{path}: malformed FUNCTION record")
            current_name = fields[1]
            if current_name in functions:
                raise ValueError(f"{path}: duplicate function {current_name}")
            order.append(current_name)
            functions[current_name] = []
        if not current_name:
            raise ValueError(f"{path}: record appears before the first FUNCTION")
        functions[current_name].append(line)

    return NormalizedIr(lines[0].rstrip("\n"), order, functions)


def limited_diff(expected: list[str], actual: list[str], limit: int) -> list[str]:
    if limit == 0:
        return []
    diff = list(
        difflib.unified_diff(
            expected,
            actual,
            fromfile="stage0",
            tofile="stage1",
            n=3,
        )
    )
    if len(diff) <= limit:
        return diff
    return diff[:limit] + [f"... diff truncated after {limit} lines\n"]


def main() -> int:
    args = parse_args()
    if args.max_diff_lines < 0:
        print("--max-diff-lines must be non-negative", file=sys.stderr)
        return 2

    try:
        expected = read_normalized_ir(args.expected)
        actual = read_normalized_ir(args.actual)
    except (OSError, UnicodeError, ValueError) as error:
        print(f"self-hosting IR comparison failed: {error}", file=sys.stderr)
        return 2

    expected_names = set(expected.order)
    actual_names = set(actual.order)
    shared_names = expected_names & actual_names
    mismatches = [
        name
        for name in expected.order
        if name in shared_names and expected.functions[name] != actual.functions[name]
    ]
    missing_actual = [name for name in expected.order if name not in actual_names]
    unexpected_actual = [name for name in actual.order if name not in expected_names]
    order_matches = expected.order == actual.order
    header_matches = expected.header == actual.header
    matching = len(shared_names) - len(mismatches)

    print(
        "self-hosting compiler IR parity: "
        f"expected={len(expected.order)} actual={len(actual.order)} "
        f"matching={matching} mismatching={len(mismatches)} "
        f"missing={len(missing_actual)} unexpected={len(unexpected_actual)} "
        f"order={'match' if order_matches else 'mismatch'}"
    )

    if header_matches and order_matches and not mismatches:
        print("self-hosting compiler IR matches Stage0")
        return 0

    if not header_matches:
        print(
            f"header mismatch: stage0={expected.header!r} stage1={actual.header!r}",
            file=sys.stderr,
        )
    if missing_actual:
        print(f"first missing function: {missing_actual[0]}", file=sys.stderr)
    if unexpected_actual:
        print(f"first unexpected function: {unexpected_actual[0]}", file=sys.stderr)
    if not order_matches and not missing_actual and not unexpected_actual:
        first_order_mismatch = next(
            index
            for index, (left, right) in enumerate(zip(expected.order, actual.order))
            if left != right
        )
        print(
            "first function order mismatch: "
            f"index={first_order_mismatch} "
            f"stage0={expected.order[first_order_mismatch]} "
            f"stage1={actual.order[first_order_mismatch]}",
            file=sys.stderr,
        )
    if mismatches:
        first = mismatches[0]
        print(f"first mismatching function: {first}", file=sys.stderr)
        for line in limited_diff(
            expected.functions[first], actual.functions[first], args.max_diff_lines
        ):
            sys.stderr.write(line)

    return 1


if __name__ == "__main__":
    raise SystemExit(main())
