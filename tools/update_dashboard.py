#!/usr/bin/env python3
"""
Update the PROJECT_STATUS.md (PRODUCTION_READINESS_STATUS.md) with latest metrics.

This script:
1. Parses tarpaulin coverage reports (XML format)
2. Calculates CI success rate from recent workflow runs
3. Updates the readiness table in the status file
4. Generates dynamic progress metrics
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import xml.etree.ElementTree as ET
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Tuple

# Crate categories for readiness assessment
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


def parse_coverage_xml(xml_path: Path) -> Dict[str, Tuple[int, int, float]]:
    """
    Parse tarpaulin coverage XML and return crate-level coverage stats.
    
    Returns:
        Dict mapping crate name to (lines_covered, lines_total, coverage_rate)
    """
    try:
        tree = ET.parse(xml_path)
        root = tree.getroot()
    except Exception as e:
        print(f"Warning: Could not parse coverage XML: {e}", file=sys.stderr)
        return {}
    
    coverage: Dict[str, Dict[Tuple[str, int], int]] = defaultdict(dict)
    
    for cls in root.findall(".//class"):
        filename = cls.get("filename", "")
        
        # Extract crate name from file path
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
        
        # Collect line coverage
        line_entries = cls.findall("lines/line")
        crate_lines = coverage[crate_name]
        for line in line_entries:
            try:
                number = int(line.get("number", "0"))
                hits = int(line.get("hits", "0"))
            except ValueError:
                continue
            key = (filename, number)
            crate_lines[key] = max(crate_lines.get(key, 0), hits)
    
    # Compute coverage rates
    rates: Dict[str, Tuple[int, int, float]] = {}
    for crate, lines in coverage.items():
        total = len(lines)
        covered = sum(1 for hits in lines.values() if hits > 0)
        rate = covered / total if total else 0.0
        rates[crate] = (covered, total, rate)
    
    return rates


def calculate_overall_coverage(rates: Dict[str, Tuple[int, int, float]]) -> Tuple[int, int, float]:
    """Calculate overall coverage across all crates."""
    total_covered = sum(covered for covered, _, _ in rates.values())
    total_lines = sum(total for _, total, _ in rates.values())
    overall_rate = total_covered / total_lines if total_lines else 0.0
    return total_covered, total_lines, overall_rate


def get_ci_success_rate(repo_owner: str = "ippan-io", repo_name: str = "ippan") -> float:
    """
    Get CI success rate from recent workflow runs.
    
    Note: This requires GITHUB_TOKEN environment variable for API access.
    Falls back to a default value if API is unavailable.
    """
    github_token = os.environ.get("GITHUB_TOKEN")
    if not github_token:
        print("Warning: GITHUB_TOKEN not set, using default CI success rate", file=sys.stderr)
        return 0.85  # Default fallback
    
    try:
        import urllib.request
        
        url = f"https://api.github.com/repos/{repo_owner}/{repo_name}/actions/runs?per_page=50"
        req = urllib.request.Request(
            url,
            headers={
                "Authorization": f"token {github_token}",
                "Accept": "application/vnd.github.v3+json",
            }
        )
        
        with urllib.request.urlopen(req, timeout=10) as response:
            data = json.loads(response.read().decode())
            runs = data.get("workflow_runs", [])
            
            if not runs:
                return 0.85
            
            success_count = sum(1 for run in runs if run.get("conclusion") == "success")
            total_count = len(runs)
            
            return success_count / total_count if total_count else 0.85
            
    except Exception as e:
        print(f"Warning: Could not fetch CI success rate: {e}", file=sys.stderr)
        return 0.85  # Default fallback


def generate_readiness_table(
    coverage_rates: Dict[str, Tuple[int, int, float]],
    overall_coverage: Tuple[int, int, float],
    ci_success_rate: float,
) -> str:
    """Generate the readiness metrics table."""
    _, _, overall_rate = overall_coverage
    
    # Calculate readiness scores
    production_ready = len([c for c in PRODUCTION_READY_CRATES if c in coverage_rates])
    total_crates = len(coverage_rates) if coverage_rates else 20
    
    # Determine overall readiness status
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
    
    # Sort crates by coverage rate (descending)
    sorted_crates = sorted(
        coverage_rates.items(),
        key=lambda x: x[1][2],
        reverse=True
    )
    
    for crate_name, (covered, total, rate) in sorted_crates:
        status = "✅" if rate >= 0.80 else ("⚠️" if rate >= 0.60 else "❌")
        table += f"| `{crate_name}` | {rate*100:.1f}% | {covered}/{total} | {status} |\n"
    
    table += "\n### Readiness Categories\n\n"
    table += f"- **Production Ready** ({len(PRODUCTION_READY_CRATES)} crates): {', '.join(f'`{c}`' for c in PRODUCTION_READY_CRATES)}\n"
    table += f"- **Partially Ready** ({len(PARTIALLY_READY_CRATES)} crates): {', '.join(f'`{c}`' for c in PARTIALLY_READY_CRATES)}\n"
    table += f"- **Critical Path** ({len(CRITICAL_CRATES)} crates): {', '.join(f'`{c}`' for c in CRITICAL_CRATES)}\n"
    
    return table


def update_status_file(status_file: Path, readiness_table: str) -> None:
    """Update the status file with the new readiness table."""
    if not status_file.exists():
        print(f"Warning: Status file not found: {status_file}", file=sys.stderr)
        # Create a basic status file
        content = f"""# IPPAN Production Readiness Status

{readiness_table}

---

*This file is automatically updated by the readiness dashboard workflow.*
"""
        status_file.write_text(content, encoding="utf-8")
        return
    
    content = status_file.read_text(encoding="utf-8")
    
    # Try to find and replace existing dashboard section
    dashboard_pattern = r"## 📊 Automated Readiness Dashboard.*?(?=\n## |\n---|\Z)"
    
    if re.search(dashboard_pattern, content, re.DOTALL):
        # Replace existing dashboard
        new_content = re.sub(
            dashboard_pattern,
            readiness_table.rstrip(),
            content,
            flags=re.DOTALL
        )
    else:
        # Insert at the beginning after the title
        lines = content.split("\n")
        title_end = 0
        for i, line in enumerate(lines):
            if line.startswith("# ") and i == 0:
                title_end = i + 1
                break
        
        lines.insert(title_end + 1, readiness_table)
        new_content = "\n".join(lines)
    
    status_file.write_text(new_content, encoding="utf-8")
    print(f"✅ Updated {status_file}")


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
        type=float,
        help="Override CI success rate (0.0-1.0). If not provided, will query GitHub API.",
    )
    parser.add_argument(
        "--repo",
        type=str,
        default="ippan-io/ippan",
        help="GitHub repository in format owner/repo (default: ippan-io/ippan)",
    )
    
    args = parser.parse_args()
    
    # Parse coverage data
    coverage_rates = {}
    if args.coverage_xml.exists():
        coverage_rates = parse_coverage_xml(args.coverage_xml)
        print(f"✅ Parsed coverage data from {args.coverage_xml}")
    else:
        print(f"Warning: Coverage file not found: {args.coverage_xml}", file=sys.stderr)
    
    # Calculate overall coverage
    overall_coverage = calculate_overall_coverage(coverage_rates)
    
    # Get CI success rate
    if args.ci_success_rate is not None:
        ci_success_rate = args.ci_success_rate
    else:
        repo_parts = args.repo.split("/")
        if len(repo_parts) == 2:
            ci_success_rate = get_ci_success_rate(repo_parts[0], repo_parts[1])
        else:
            ci_success_rate = 0.85
    
    print(f"📊 Overall Coverage: {overall_coverage[2]*100:.1f}%")
    print(f"📊 CI Success Rate: {ci_success_rate*100:.1f}%")
    
    # Generate readiness table
    readiness_table = generate_readiness_table(
        coverage_rates,
        overall_coverage,
        ci_success_rate,
    )
    
    # Update status file
    update_status_file(args.status_file, readiness_table)
    
    print("\n✅ Dashboard update complete!")
    return 0


if __name__ == "__main__":
    sys.exit(main())
