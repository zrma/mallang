#!/usr/bin/env python3
"""Validate the v1 rule inventory and its conformance evidence map."""

from __future__ import annotations

import json
import os
import re
import sys
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
CONTRACT = ROOT / "docs" / "V1_LANGUAGE_CONTRACT.md"
MANIFEST = ROOT / "docs" / "conformance" / "v1-rules.json"
COMPATIBILITY = ROOT / "docs" / "COMPATIBILITY.md"
MIGRATION = ROOT / "docs" / "MIGRATION_V1.md"
RULE_RE = re.compile(r"^\| `(V1-[A-Z]+-[0-9]{3})` \|")
PROFILE_RE = re.compile(r"^[a-z][a-z0-9-]*$")
TEST_RE = re.compile(r"^[a-z][a-z0-9_]*$")


def fail(message: str) -> None:
    print(f"v1 conformance map error: {message}", file=sys.stderr)
    raise SystemExit(1)


def repo_path(value: object, *, kind: str) -> Path:
    if not isinstance(value, str) or not value:
        fail(f"{kind} evidence path must be a non-empty string")
    path = Path(value)
    if path.is_absolute() or ".." in path.parts:
        fail(f"{kind} evidence path must be repository-relative: {value}")
    resolved = ROOT / path
    if not resolved.is_file():
        fail(f"{kind} evidence path does not exist: {value}")
    return resolved


def rust_sources() -> str:
    paths = sorted((ROOT / "src").rglob("*.rs")) + sorted((ROOT / "tests").rglob("*.rs"))
    return "\n".join(path.read_text(encoding="utf-8") for path in paths)


def main() -> None:
    contract_text = CONTRACT.read_text(encoding="utf-8")
    contract_rules = [
        match.group(1)
        for line in contract_text.splitlines()
        if (match := RULE_RE.match(line))
    ]
    duplicate_contract = sorted(
        rule for rule, count in Counter(contract_rules).items() if count != 1
    )
    if duplicate_contract:
        fail(f"contract rule definitions are not unique: {', '.join(duplicate_contract)}")
    if not contract_rules:
        fail("contract has no rule definitions")

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if manifest.get("schema") != "mallang.v1.conformance.v1":
        fail("unexpected manifest schema")
    profiles = manifest.get("profiles")
    if not isinstance(profiles, list) or not profiles:
        fail("manifest profiles must be a non-empty list")

    source_text = rust_sources()
    profile_ids: list[str] = []
    assigned_rules: list[str] = []
    evidence_count = 0

    for profile in profiles:
        if not isinstance(profile, dict):
            fail("each profile must be an object")
        profile_id = profile.get("id")
        if not isinstance(profile_id, str) or not PROFILE_RE.fullmatch(profile_id):
            fail(f"invalid profile id: {profile_id!r}")
        profile_ids.append(profile_id)
        if not isinstance(profile.get("summary"), str) or not profile["summary"].strip():
            fail(f"profile {profile_id} has no summary")

        rules = profile.get("rules")
        if not isinstance(rules, list) or not rules or not all(isinstance(rule, str) for rule in rules):
            fail(f"profile {profile_id} has no rules")
        assigned_rules.extend(rules)

        evidence = profile.get("evidence")
        if not isinstance(evidence, list) or not evidence:
            fail(f"profile {profile_id} has no evidence")
        evidence_count += len(evidence)
        for item in evidence:
            if not isinstance(item, dict):
                fail(f"profile {profile_id} evidence must be an object")
            kind = item.get("kind")
            if kind == "script":
                path = repo_path(item.get("path"), kind=kind)
                if "scripts" not in path.relative_to(ROOT).parts:
                    fail(f"script evidence is outside scripts/: {path.relative_to(ROOT)}")
                if not os.access(path, os.X_OK):
                    fail(f"script evidence is not executable: {path.relative_to(ROOT)}")
            elif kind == "fixture":
                path = repo_path(item.get("path"), kind=kind)
                if path.suffix != ".mlg":
                    fail(f"fixture evidence is not a .mlg file: {path.relative_to(ROOT)}")
            elif kind == "rust-test":
                symbol = item.get("symbol")
                if not isinstance(symbol, str) or not TEST_RE.fullmatch(symbol):
                    fail(f"invalid Rust test symbol in profile {profile_id}: {symbol!r}")
                if re.search(rf"\bfn\s+{re.escape(symbol)}\s*\(", source_text) is None:
                    fail(f"Rust test symbol not found: {symbol}")
            elif kind == "command":
                command = item.get("command")
                if not isinstance(command, str) or not command.strip():
                    fail(f"profile {profile_id} has an empty command")
                if command.startswith("/") or "/Users/" in command:
                    fail(f"profile {profile_id} command exposes a local absolute path")
            else:
                fail(f"profile {profile_id} has unsupported evidence kind: {kind!r}")

    duplicate_profiles = sorted(
        profile for profile, count in Counter(profile_ids).items() if count != 1
    )
    if duplicate_profiles:
        fail(f"duplicate profile ids: {', '.join(duplicate_profiles)}")

    duplicate_assignments = sorted(
        rule for rule, count in Counter(assigned_rules).items() if count != 1
    )
    if duplicate_assignments:
        fail(f"rules assigned more than once: {', '.join(duplicate_assignments)}")
    contract_set = set(contract_rules)
    assigned_set = set(assigned_rules)
    missing = sorted(contract_set - assigned_set)
    unknown = sorted(assigned_set - contract_set)
    if missing:
        fail(f"unmapped contract rules: {', '.join(missing)}")
    if unknown:
        fail(f"manifest contains unknown rules: {', '.join(unknown)}")

    compatibility_text = COMPATIBILITY.read_text(encoding="utf-8")
    for heading in (
        "## Version model",
        "## 1.x guarantees",
        "## Release classes",
        "## Deprecation",
        "## Soundness and security exception",
        "## Pre-1.0 and freeze policy",
    ):
        if heading not in compatibility_text:
            fail(f"compatibility policy heading is missing: {heading}")
    if not MIGRATION.is_file():
        fail("v1 migration guide is missing")

    print(
        "v1 conformance map passed: "
        f"rules={len(contract_rules)} profiles={len(profiles)} evidence={evidence_count}"
    )


if __name__ == "__main__":
    main()
