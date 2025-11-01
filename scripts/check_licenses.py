#!/usr/bin/env python3
"""Validate that required third-party license texts are present."""

from __future__ import annotations

import csv
import json
from pathlib import Path
import sys

try:  # Python 3.11+
    import tomllib
except ModuleNotFoundError as exc:  # pragma: no cover - fallback for <3.11
    raise SystemExit("Python 3.11+ is required for tomllib") from exc


CARGO_ABOUT_JSON = Path("target/legal/cargo-about.json")
GO_LICENSES_CSV = Path("target/legal/go-licenses.csv")
LICENSE_MAP_TOML = Path("docs/legal/license-files.toml")


def parse_license_expression(expr: str) -> set[str]:
    tokens = (
        expr.replace("(", " ")
        .replace(")", " ")
        .replace("AND", " ")
        .replace("OR", " ")
        .split()
    )
    normalized = set()
    for token in tokens:
        cleaned = token.strip()
        if not cleaned or cleaned.upper() == "WITH":
            continue
        normalized.add(cleaned)
    return normalized


def collect_rust_licenses() -> set[str]:
    if not CARGO_ABOUT_JSON.exists():
        print(
            f"expected {CARGO_ABOUT_JSON} to exist; run `cargo about generate` first",
            file=sys.stderr,
        )
        raise SystemExit(1)
    data = json.loads(CARGO_ABOUT_JSON.read_text(encoding="utf-8"))
    licenses: set[str] = set()
    for entry in data.get("crates", []):
        if not isinstance(entry, dict):
            continue
        expr = entry.get("license") or entry.get("license-expression")
        if not expr:
            continue
        licenses |= parse_license_expression(expr)
    return licenses


def collect_go_licenses() -> set[str]:
    if not GO_LICENSES_CSV.exists():
        print(
            f"expected {GO_LICENSES_CSV} to exist; run `go-licenses report` first",
            file=sys.stderr,
        )
        raise SystemExit(1)
    licenses: set[str] = set()
    with GO_LICENSES_CSV.open(encoding="utf-8") as fh:
        reader = csv.reader(fh)
        for row in reader:
            if len(row) < 3:
                continue
            license_id = row[2].strip()
            if license_id:
                licenses.add(license_id)
    return licenses


def load_license_map() -> dict[str, list[Path]]:
    if not LICENSE_MAP_TOML.exists():
        print(f"Missing license map: {LICENSE_MAP_TOML}", file=sys.stderr)
        raise SystemExit(1)
    data = tomllib.loads(LICENSE_MAP_TOML.read_text(encoding="utf-8"))
    raw_map = data.get("licenses") or {}
    mapping: dict[str, list[Path]] = {}
    for license_id, value in raw_map.items():
        if isinstance(value, str):
            mapping[license_id] = [Path(value)]
        else:
            mapping[license_id] = [Path(path) for path in value]
    return mapping


def main() -> int:
    license_map = load_license_map()
    detected = collect_rust_licenses() | collect_go_licenses()

    missing_ids = sorted(license_id for license_id in detected if license_id not in license_map)
    if missing_ids:
        print(
            "No license text registered for:\n- " + "\n- ".join(missing_ids),
            file=sys.stderr,
        )
        return 1

    missing_files: list[str] = []
    for license_id in sorted(detected):
        for path in license_map[license_id]:
            if not path.exists():
                missing_files.append(f"{license_id}: {path}")

    if missing_files:
        print(
            "License map references missing files:\n- " + "\n- ".join(missing_files),
            file=sys.stderr,
        )
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
