#!/usr/bin/env python3
"""
Generate a Markdown coverage summary from a Cobertura XML report.

The script aggregates coverage per crate (based on file paths) and writes a
`coverage-summary.md` file that can be uploaded as a CI artifact. Target crates
can enforce a minimum coverage threshold, causing a non-zero exit code when
they fall below the requirement.
"""

from __future__ import annotations

import argparse
import sys
import xml.etree.ElementTree as ET
from collections import defaultdict
from pathlib import Path
from typing import Dict, Tuple


CRATE_NAME_MAP = {
    "storage": "ippan-storage",
    "validator_resolution": "ippan-validator-resolution",
    "l1_handle_anchors": "ippan-l1-handle-anchors",
    "rpc": "ippan-rpc",
}

TARGET_CRATES = {
    "ippan-storage",
    "ippan-validator-resolution",
    "ippan-l1-handle-anchors",
    "ippan-rpc",
}


def crate_from_filename(filename: str) -> str | None:
    if filename.startswith("crates/"):
        parts = filename.split("/")
        if len(parts) >= 2:
            crate_key = parts[1]
            return CRATE_NAME_MAP.get(crate_key, crate_key)
    return None


def load_coverage(xml_path: Path) -> Dict[str, Dict[Tuple[str, int], int]]:
    tree = ET.parse(xml_path)
    root = tree.getroot()

    coverage: Dict[str, Dict[Tuple[str, int], int]] = defaultdict(dict)

    for cls in root.findall(".//class"):
        filename = cls.get("filename", "")
        crate = crate_from_filename(filename)
        if not crate:
            continue

        line_entries = cls.findall("lines/line")
        crate_lines = coverage[crate]
        for line in line_entries:
            try:
                number = int(line.get("number", "0"))
                hits = int(line.get("hits", "0"))
            except ValueError:
                continue
            key = (filename, number)
            crate_lines[key] = max(crate_lines.get(key, 0), hits)

    return coverage


def compute_rates(coverage: Dict[str, Dict[Tuple[str, int], int]]) -> Dict[str, Tuple[int, int, float]]:
    rates: Dict[str, Tuple[int, int, float]] = {}
    for crate, lines in coverage.items():
        total = len(lines)
        covered = sum(1 for hits in lines.values() if hits > 0)
        rate = covered / total if total else 0.0
        rates[crate] = (covered, total, rate)
    return rates


def format_percent(value: float) -> str:
    return f"{value * 100:.2f}%"


def write_summary(
    output_path: Path,
    rates: Dict[str, Tuple[int, int, float]],
    overall: Tuple[int, int, float],
    threshold: float,
) -> None:
    lines = ["# Coverage Summary", "", "| Scope | Coverage | Covered | Lines |", "| --- | --- | --- | --- |"]

    overall_cov, overall_total_rate = overall[0], overall[1]
    lines.append(
        "| overall | {rate} | {covered} | {total} |".format(
            rate=format_percent(overall[2]), covered=overall_cov, total=overall_total_rate
        )
    )

    for crate in sorted(rates.keys()):
        covered, total, rate = rates[crate]
        lines.append(
            "| {crate} | {rate} | {covered} | {total} |".format(
                crate=crate,
                rate=format_percent(rate),
                covered=covered,
                total=total,
            )
        )

    lines.append("")
    lines.append(f"Minimum required coverage for target crates: {format_percent(threshold)}")

    output_path.write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate coverage summary")
    parser.add_argument("cobertura", type=Path, help="Path to cobertura XML report")
    parser.add_argument("output", type=Path, help="Path to write coverage-summary.md")
    parser.add_argument(
        "--threshold",
        type=float,
        default=0.80,
        help="Minimum coverage required for target crates (default: 0.80)",
    )
    args = parser.parse_args()

    coverage = load_coverage(args.cobertura)
    rates = compute_rates(coverage)

    all_lines: Dict[Tuple[str, int], int] = {}
    for crate_lines in coverage.values():
        for key, hits in crate_lines.items():
            all_lines[key] = max(all_lines.get(key, 0), hits)

    overall_total = len(all_lines)
    overall_covered = sum(1 for hits in all_lines.values() if hits > 0)
    overall_rate = overall_covered / overall_total if overall_total else 0.0

    write_summary(args.output, rates, (overall_covered, overall_total, overall_rate), args.threshold)

    failures = []
    for crate in TARGET_CRATES:
        covered, total, rate = rates.get(crate, (0, 0, 0.0))
        if total == 0 or rate < args.threshold:
            failures.append((crate, format_percent(rate), total))

    if failures:
        for crate, rate_str, total in failures:
            print(
                f"Coverage threshold not met for {crate}: {rate_str} across {total} instrumented lines",
                file=sys.stderr,
            )
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
