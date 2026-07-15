#!/usr/bin/env python3
import argparse
import json
import sys


SCHEMA = "mallang.diagnostic.v1"
STAGES = {
    "cli",
    "input",
    "frontend",
    "package",
    "link",
    "semantic",
    "ir",
    "backend",
    "native",
}


def fail(message):
    print(f"diagnostic consumer: {message}", file=sys.stderr)
    raise SystemExit(1)


def validate_position(position, label):
    if not isinstance(position, dict) or set(position) != {"line", "column"}:
        fail(f"{label} has unexpected fields")
    if not all(type(position[key]) is int and position[key] >= 1 for key in position):
        fail(f"{label} must use positive integer line and column")


def validate_record(record):
    required = {"schema", "severity", "stage", "message"}
    allowed = required | {"source"}
    if not required.issubset(record) or not set(record).issubset(allowed):
        fail("record has missing or unexpected top-level fields")
    if record["schema"] != SCHEMA:
        fail(f"unsupported schema {record['schema']!r}")
    if record["severity"] != "error":
        fail("v1 consumer expects error severity")
    if record["stage"] not in STAGES:
        fail(f"unknown stage {record['stage']!r}")
    if not isinstance(record["message"], str) or not record["message"]:
        fail("message must be a non-empty string")

    source = record.get("source")
    if source is None:
        return
    if not isinstance(source, dict) or not {"path"}.issubset(source):
        fail("source must contain a path")
    if not set(source).issubset({"path", "span"}):
        fail("source has unexpected fields")
    if not isinstance(source["path"], str) or not source["path"]:
        fail("source path must be a non-empty string")
    span = source.get("span")
    if span is None:
        return
    expected_span = {"byte_start", "byte_end", "start", "end"}
    if not isinstance(span, dict) or set(span) != expected_span:
        fail("span has unexpected fields")
    if type(span["byte_start"]) is not int or type(span["byte_end"]) is not int:
        fail("byte offsets must be integers")
    if span["byte_start"] < 0 or span["byte_end"] < span["byte_start"]:
        fail("byte offsets are not ordered")
    validate_position(span["start"], "span start")
    validate_position(span["end"], "span end")


def render_human(record):
    source = record.get("source")
    if source is None:
        return record["message"]
    span = source.get("span")
    if span is None:
        return f"{source['path']}: {record['message']}"
    start = span["start"]
    return f"{source['path']}:{start['line']}:{start['column']}: {record['message']}"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--expect-stage", choices=sorted(STAGES))
    parser.add_argument("--expect-count", type=int, default=1)
    parser.add_argument("--expect-path")
    parser.add_argument("--render-human", action="store_true")
    args = parser.parse_args()

    records = []
    for line_number, line in enumerate(sys.stdin, start=1):
        if not line.endswith("\n"):
            fail(f"line {line_number} is not newline-terminated")
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            fail(f"line {line_number} is not JSON: {error}")
        if not isinstance(record, dict):
            fail(f"line {line_number} is not an object")
        validate_record(record)
        records.append(record)

    if len(records) != args.expect_count:
        fail(f"expected {args.expect_count} records, got {len(records)}")
    if args.expect_stage and any(record["stage"] != args.expect_stage for record in records):
        fail(f"expected every record to use stage {args.expect_stage!r}")
    if args.expect_path and any(
        record.get("source", {}).get("path") != args.expect_path for record in records
    ):
        fail(f"expected every record to use path {args.expect_path!r}")
    if args.render_human:
        for record in records:
            print(render_human(record))


if __name__ == "__main__":
    main()
