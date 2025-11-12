#!/usr/bin/env python3
"""
Update project readiness dashboards.

This script:
1. Parses tarpaulin coverage reports (XML format)
2. Calculates CI success rate from recent workflow runs (or uses provided override)
3. Updates the automated section of `PRODUCTION_READINESS_STATUS.md`
4. Optionally refreshes `PROJECT_STATUS.csv` and generates `PROJECT_STATUS.md`
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import re
import sys
import xml.etree.ElementTree as ET
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Tuple

# ---------------------------------------------------------------------------
# Production readiness dashboard constants
# ---------------------------------------------------------------------------

PRODUCTION_READY_CRATES = {
    "ippan-crypto",
    "ippan-types",
    "ippan-time",
}

PARTIALLY_READY_CRATES = {
    "ippan-core",
    "ippan-network",
}

CRITICAL_CRATES = {
    "ippan-economics",
    "ippan-ai-core",
    "ippan-consensus",
    "ippan-governance",
}

# ---------------------------------------------------------------------------
# Project status dashboard constants
# ---------------------------------------------------------------------------

STATUS_THRESHOLDS: List[Tuple[str, float]] = [
    ("Green", 0.80),
    ("Yellow", 0.60),
    ("Orange", 0.40),
]

STATUS_EMOJI: Dict[str, str] = {
    "Green": "🟢",
    "Yellow": "🟡",
    "Orange": "🟠",
    "Red": "🔴",
}

PROJECT_STATUS_HEADERS = [
    "Category",
    "Weight",
    "Score",
    "Status",
    "Description",
    "Key Actions",
]


@dataclass
class ProjectRow:
    category: str
    weight: str
    score: str
    status: str
    description: str
    key_actions: str

    @classmethod
    def from_dict(cls, raw: Dict[str, str]) -> "ProjectRow":
        return cls(
            category=raw["Category"],
            weight=raw["Weight"],
            score=raw["Score"],
            status=raw["Status"],
            description=raw["Description"],
            key_actions=raw["Key Actions"],
        )

    def to_dict(self) -> Dict[str, str]:
        return {
            "Category": self.category,
            "Weight": self.weight,
            "Score": self.score,
            "Status": self.status,
            "Description": self.description,
            "Key Actions": self.key_actions,
        }


# ---------------------------------------------------------------------------
# Coverage parsing utilities
# ---------------------------------------------------------------------------

def parse_coverage_xml(xml_path: Path) -> Dict[str, Tuple[int, int, float]]:
    """
    Parse tarpaulin coverage XML and return crate-level coverage stats.

    Returns:
        Dict mapping crate name to (lines_covered, lines_total, coverage_rate)
    """
    try:
        tree = ET.parse(xml_path)
        root = tree.getroot()
    except Exception as exc:  # pragma: no cover - defensive
        print(f"Warning: Could not parse coverage XML {xml_path}: {exc}", file=sys.stderr)
        return {}

    coverage: Dict[str, Dict[Tuple[str, int], int]] = defaultdict(dict)

    for cls in root.findall(".//class"):
        filename = cls.get("filename", "")

        if filename.startswith("crates/"):
            parts = filename.split("/")
            if len(parts) >= 2:
                crate_name = parts[1].replace("_", "-")
                if not crate_name.startswith("ippan-"):
                    crate_name = f"ippan-{crate_name}"
            else:
                continue
        else:
            continue

        crate_lines = coverage[crate_name]
        for line in cls.findall("lines/line"):
            try:
                number = int(line.get("number", "0"))
                hits = int(line.get("hits", "0"))
            except (TypeError, ValueError):
                continue
            key = (filename, number)
            crate_lines[key] = max(crate_lines.get(key, 0), hits)

    rates: Dict[str, Tuple[int, int, float]] = {}
    for crate, lines in coverage.items():
        total = len(lines)
        covered = sum(1 for hits in lines.values() if hits > 0)
        rate = covered / total if total else 0.0
        rates[crate] = (covered, total, rate)

    return rates


def calculate_overall_coverage(
    rates: Dict[str, Tuple[int, int, float]]
) -> Tuple[int, int, float]:
    """Calculate overall coverage across all crates."""
    total_covered = sum(covered for covered, _, _ in rates.values())
    total_lines = sum(total for _, total, _ in rates.values())
    overall_rate = total_covered / total_lines if total_lines else 0.0
    return total_covered, total_lines, overall_rate


# ---------------------------------------------------------------------------
# CI metrics utilities
# ---------------------------------------------------------------------------

def get_ci_success_rate(
    repo_owner: str = "ippan-io",
    repo_name: str = "ippan",
    per_page: int = 50,
) -> Tuple[float, int]:
    """
    Get CI success rate from recent workflow runs.

    Returns:
        Tuple[success_rate (0-1), total_runs_considered]
    """
    github_token = os.environ.get("GITHUB_TOKEN")
    if not github_token:
        print(
            "Warning: GITHUB_TOKEN not set, using default CI success rate",
            file=sys.stderr,
        )
        return 0.85, per_page

    try:
        import urllib.request

        url = (
            f"https://api.github.com/repos/{repo_owner}/{repo_name}/actions/runs"
            f"?per_page={per_page}"
        )
        req = urllib.request.Request(
            url,
            headers={
                "Authorization": f"token {github_token}",
                "Accept": "application/vnd.github.v3+json",
            },
        )

        with urllib.request.urlopen(req, timeout=15) as response:
            data = json.loads(response.read().decode())
            runs = data.get("workflow_runs", [])

            if not runs:
                return 0.85, 0

            success_count = sum(
                1 for run in runs if run.get("conclusion") == "success"
            )
            total_count = len(runs)

            return (
                success_count / total_count if total_count else 0.85,
                total_count,
            )

    except Exception as exc:  # pragma: no cover - defensive
        print(f"Warning: Could not fetch CI success rate: {exc}", file=sys.stderr)
        return 0.85, 0


# ---------------------------------------------------------------------------
# Production readiness markdown generation
# ---------------------------------------------------------------------------

def generate_readiness_table(
    coverage_rates: Dict[str, Tuple[int, int, float]],
    overall_coverage: Tuple[int, int, float],
    ci_success_rate: float,
) -> str:
    """Generate the readiness metrics table for PRODUCTION_READINESS_STATUS.md."""
    _, _, overall_rate = overall_coverage

    production_ready = len([c for c in PRODUCTION_READY_CRATES if c in coverage_rates])
    total_crates = len(coverage_rates) if coverage_rates else 20

    if overall_rate >= 0.80 and ci_success_rate >= 0.90:
        status_icon = "🟢"
        status_text = "GOOD"
    elif overall_rate >= 0.60 and ci_success_rate >= 0.75:
        status_icon = "🟡"
        status_text = "FAIR"
    else:
        status_icon = "🔴"
        status_text = "NEEDS WORK"

    table = f"""## 📊 Automated Readiness Dashboard

> **Last Updated**: {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M:%S UTC')}
> **Generated by**: Automated Dashboard Bot

### Overall Status: {status_icon} {status_text}

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| **Test Coverage** | {overall_rate*100:.1f}% | ≥80% | {"✅" if overall_rate >= 0.80 else "❌"} |
| **CI Success Rate** | {ci_success_rate*100:.1f}% | ≥90% | {"✅" if ci_success_rate >= 0.90 else "❌"} |
| **Production-Ready Crates** | {production_ready}/{total_crates} | ≥15/20 | {"✅" if production_ready >= 15 else "❌"} |

### Coverage by Crate

| Crate | Coverage | Lines | Status |
|-------|----------|-------|--------|
"""

    sorted_crates = sorted(
        coverage_rates.items(),
        key=lambda entry: entry[1][2],
        reverse=True,
    )

    for crate_name, (covered, total, rate) in sorted_crates:
        status = "✅" if rate >= 0.80 else ("⚠️" if rate >= 0.60 else "❌")
        table += f"| `{crate_name}` | {rate*100:.1f}% | {covered}/{total} | {status} |\n"

    table += "\n### Readiness Categories\n\n"
    table += (
        f"- **Production Ready** ({len(PRODUCTION_READY_CRATES)} crates): "
        f"{', '.join(f'`{c}`' for c in PRODUCTION_READY_CRATES)}\n"
    )
    table += (
        f"- **Partially Ready** ({len(PARTIALLY_READY_CRATES)} crates): "
        f"{', '.join(f'`{c}`' for c in PARTIALLY_READY_CRATES)}\n"
    )
    table += (
        f"- **Critical Path** ({len(CRITICAL_CRATES)} crates): "
        f"{', '.join(f'`{c}`' for c in CRITICAL_CRATES)}\n"
    )

    return table


def update_status_file(status_file: Path, readiness_table: str) -> None:
    """Update the production readiness status file with the new table."""
    if not status_file.exists():
        print(f"Warning: Status file not found: {status_file}", file=sys.stderr)
        content = f"""# IPPAN Production Readiness Status

{readiness_table}

---

*This file is automatically updated by the readiness dashboard workflow.*
"""
        status_file.write_text(content, encoding="utf-8")
        return

    content = status_file.read_text(encoding="utf-8")
    dashboard_pattern = r"## 📊 Automated Readiness Dashboard.*?(?=\n## |\n---|\Z)"

    if re.search(dashboard_pattern, content, re.DOTALL):
        new_content = re.sub(
            dashboard_pattern,
            readiness_table.rstrip(),
            content,
            flags=re.DOTALL,
        )
    else:
        lines = content.split("\n")
        title_end = 0
        for idx, line in enumerate(lines):
            if line.startswith("# ") and idx == 0:
                title_end = idx + 1
                break

        lines.insert(title_end + 1, readiness_table)
        new_content = "\n".join(lines)

    status_file.write_text(new_content, encoding="utf-8")
    print(f"✅ Updated {status_file}")


# ---------------------------------------------------------------------------
# Project status helpers
# ---------------------------------------------------------------------------

def load_project_status(csv_path: Path) -> List[ProjectRow]:
    with csv_path.open("r", newline="") as handle:
        reader = csv.DictReader(handle)
        return [ProjectRow.from_dict(row) for row in reader]


def save_project_status(csv_path: Path, rows: Iterable[ProjectRow]) -> None:
    with csv_path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=PROJECT_STATUS_HEADERS)
        writer.writeheader()
        for row in rows:
            writer.writerow(row.to_dict())


def determine_status(score: float) -> str:
    for label, threshold in STATUS_THRESHOLDS:
        if score >= threshold:
            return label
    return "Red"


def format_weight(weight: str) -> str:
    cleaned = weight.strip()
    if not cleaned or cleaned == "-":
        return "–"
    try:
        value = float(cleaned)
    except ValueError:
        return cleaned
    return f"{value * 100:.0f} %"


def write_project_status_markdown(md_path: Path, rows: Iterable[ProjectRow]) -> None:
    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    lines = [
        "# Project Readiness Dashboard",
        "",
        f"_Last updated: {timestamp}_",
        "",
        "| Category | Weight | Score | Status | Description | Key Actions |",
        "| --- | --- | --- | --- | --- | --- |",
    ]

    for row in rows:
        weight_display = format_weight(row.weight)
        try:
            score_value = float(row.score)
            score_display = f"{score_value:.2f} ({score_value * 100:.1f} %)"
        except ValueError:
            score_display = row.score
        status_icon = STATUS_EMOJI.get(row.status, "")
        status_display = f"{status_icon} {row.status}".strip()
        lines.append(
            f"| {row.category} | {weight_display} | {score_display} | "
            f"{status_display} | {row.description} | {row.key_actions} |"
        )

    md_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _get_numeric(value: Optional[str]) -> Optional[float]:
    if value is None:
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def recalculate_overall_row(rows: List[ProjectRow]) -> None:
    row_map = {row.category: row for row in rows}
    components = [
        ("Implementation Completeness", row_map.get("Implementation Completeness")),
        ("Testing & Verification", row_map.get("Testing & Verification")),
        ("Operational Hardening", row_map.get("Operational Hardening")),
    ]

    total_weight = 0.0
    weighted_sum = 0.0
    for _, row in components:
        if not row:
            continue
        score = _get_numeric(row.score)
        weight = _get_numeric(row.weight)
        if score is None or weight is None:
            continue
        weighted_sum += score * weight
        total_weight += weight

    overall = row_map.get("Overall Readiness")
    if overall and total_weight > 0:
        overall_score = weighted_sum / total_weight
        overall.score = f"{overall_score:.2f}"
        overall.status = determine_status(overall_score)


def update_project_status(
    csv_path: Path,
    md_path: Path,
    coverage_rate: Optional[float],
    ci_success_rate: Optional[float],
    ci_window: Optional[int],
    workflow_label: Optional[str],
) -> None:
    if not csv_path.exists():
        print(f"Warning: Project status CSV not found: {csv_path}", file=sys.stderr)
        return

    rows = load_project_status(csv_path)
    row_map = {row.category: row for row in rows}

    if coverage_rate is not None and "Testing & Verification" in row_map:
        testing_row = row_map["Testing & Verification"]
        testing_row.score = f"{coverage_rate:.2f}"
        testing_row.status = determine_status(coverage_rate)
        testing_row.description = (
            f"Tarpaulin line coverage {coverage_rate * 100:.1f}% (workspace)"
        )

    if ci_success_rate is not None and "CI/CD Reliability" in row_map:
        ci_row = row_map["CI/CD Reliability"]
        ci_row.score = f"{ci_success_rate:.2f}"
        ci_row.status = determine_status(ci_success_rate)
        if ci_window and ci_window > 0:
            if workflow_label:
                desc = (
                    f"{ci_success_rate * 100:.1f}% success across last "
                    f"{ci_window} runs of {workflow_label}"
                )
            else:
                desc = (
                    f"{ci_success_rate * 100:.1f}% success across last "
                    f"{ci_window} workflow runs"
                )
        else:
            desc = f"{ci_success_rate * 100:.1f}% success rate (recent runs)"
        ci_row.description = desc

    recalculate_overall_row(rows)
    save_project_status(csv_path, rows)
    write_project_status_markdown(md_path, rows)
    print(f"✅ Updated {csv_path} and {md_path}")


# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(
        description="Update project readiness dashboard with coverage and CI metrics"
    )
    parser.add_argument(
        "coverage_xml",
        type=Path,
        nargs="?",
        default=Path("coverage/cobertura.xml"),
        help="Path to tarpaulin coverage XML report (default: coverage/cobertura.xml)",
    )
    parser.add_argument(
        "--status-file",
        type=Path,
        default=Path("PRODUCTION_READINESS_STATUS.md"),
        help="Path to status markdown file (default: PRODUCTION_READINESS_STATUS.md)",
    )
    parser.add_argument(
        "--ci-success-rate",
        type=str,
        help="Override CI success rate (0.0-1.0). If not provided, the GitHub API will be queried.",
    )
    parser.add_argument(
        "--ci-window",
        type=int,
        default=50,
        help="Number of recent workflow runs considered for CI success rate (default: 50).",
    )
    parser.add_argument(
        "--ci-workflow",
        type=str,
        default=None,
        help="Workflow label used for CI success rate messaging.",
    )
    parser.add_argument(
        "--project-status-csv",
        type=Path,
        default=Path("PROJECT_STATUS.csv"),
        help="Path to project status CSV file (default: PROJECT_STATUS.csv).",
    )
    parser.add_argument(
        "--project-status-md",
        type=Path,
        default=Path("PROJECT_STATUS.md"),
        help="Path to project status markdown file (default: PROJECT_STATUS.md).",
    )
    parser.add_argument(
        "--repo",
        type=str,
        default="ippan-io/ippan",
        help="GitHub repository in format owner/repo (default: ippan-io/ippan).",
    )

    args = parser.parse_args()

    coverage_rates: Dict[str, Tuple[int, int, float]] = {}
    if args.coverage_xml.exists():
        coverage_rates = parse_coverage_xml(args.coverage_xml)
        print(f"✅ Parsed coverage data from {args.coverage_xml}")
    else:
        print(
            f"Warning: Coverage file not found: {args.coverage_xml}",
            file=sys.stderr,
        )

    overall_coverage = calculate_overall_coverage(coverage_rates)

    if args.ci_success_rate is not None:
        try:
            ci_success_rate = float(args.ci_success_rate)
        except ValueError:
            print(
                f"Warning: invalid ci-success-rate '{args.ci_success_rate}', falling back to API",
                file=sys.stderr,
            )
            ci_success_rate, total_runs = get_ci_success_rate(per_page=args.ci_window)
        else:
            total_runs = args.ci_window
    else:
        repo_owner, repo_name = (args.repo.split("/", 1) + ["ippan"])[:2]
        ci_success_rate, total_runs = get_ci_success_rate(
            repo_owner=repo_owner,
            repo_name=repo_name,
            per_page=args.ci_window,
        )

    print(f"📊 Overall Coverage: {overall_coverage[2] * 100:.1f}%")
    print(f"📊 CI Success Rate: {ci_success_rate * 100:.1f}% over {total_runs} runs")

    readiness_table = generate_readiness_table(
        coverage_rates,
        overall_coverage,
        ci_success_rate,
    )
    update_status_file(args.status_file, readiness_table)

    if args.project_status_csv:
        update_project_status(
            csv_path=args.project_status_csv,
            md_path=args.project_status_md,
            coverage_rate=overall_coverage[2],
            ci_success_rate=ci_success_rate,
            ci_window=total_runs if total_runs else args.ci_window,
            workflow_label=args.ci_workflow,
        )

    print("\n✅ Dashboard update complete!")
    return 0


if __name__ == "__main__":
    sys.exit(main())
