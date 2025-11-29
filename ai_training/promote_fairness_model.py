#!/usr/bin/env python3
"""
IPPAN Fairness Model Promotion Tool

Promotes a trained model to runtime location and updates config/dlc.toml,
but REFUSES if the new model hash matches the currently pinned hash
(unless --allow-same-hash is set).

This prevents accidentally creating "fake" new versions when the model hasn't changed.
"""

import argparse
import re
import shutil
import sys
from pathlib import Path
from typing import Optional, Tuple

try:
    from blake3 import blake3
except ImportError:
    print("ERROR: 'blake3' package required. Install with: pip install blake3==0.4.1", file=sys.stderr)
    sys.exit(1)


def compute_blake3_hash(file_path: Path) -> str:
    """Compute BLAKE3 hash of file bytes."""
    with open(file_path, "rb") as f:
        data = f.read()
    return blake3(data).hexdigest()


def find_section_bounds(lines: list[str], section_name: str) -> Optional[Tuple[int, int]]:
    """
    Find start and end line indices of a TOML section.
    
    Returns (start_line, end_line) where end_line is exclusive.
    """
    start_pattern = re.compile(rf"^\s*\[{re.escape(section_name)}\]\s*$")
    section_pattern = re.compile(r"^\s*\[.*\]\s*$")
    
    start_idx = None
    for i, line in enumerate(lines):
        if start_pattern.match(line):
            start_idx = i
            break
    
    if start_idx is None:
        return None
    
    # Find end of section (next [section] or end of file)
    end_idx = len(lines)
    for i in range(start_idx + 1, len(lines)):
        if section_pattern.match(lines[i]) and not lines[i].strip().startswith(f"[{section_name}"):
            end_idx = i
            break
    
    return (start_idx, end_idx)


def extract_hash_from_config(config_path: Path) -> Optional[str]:
    """
    Extract current pinned hash from config/dlc.toml [dgbdt.model] section.
    
    Looks for either 'expected_hash' or 'hash_b3' keys.
    """
    with open(config_path, "r", encoding="utf-8") as f:
        lines = f.readlines()
    
    bounds = find_section_bounds(lines, "dgbdt.model")
    if bounds is None:
        return None
    
    start, end = bounds
    section_lines = lines[start:end]
    
    # Look for expected_hash or hash_b3
    hash_patterns = [
        re.compile(r'expected_hash\s*=\s*"([^"]+)"'),
        re.compile(r'hash_b3\s*=\s*"([^"]+)"'),
    ]
    
    for line in section_lines:
        for pattern in hash_patterns:
            match = pattern.search(line)
            if match:
                return match.group(1)
    
    return None


def update_config_section(
    config_path: Path,
    model_path: str,
    new_hash: str,
    dry_run: bool = False,
) -> None:
    """
    Update [dgbdt.model] section in config file.
    
    - Sets 'path' to model_path (forward slashes)
    - Sets 'expected_hash' or 'hash_b3' to new_hash (preserves existing key name)
    """
    with open(config_path, "r", encoding="utf-8") as f:
        lines = f.readlines()
        original_content = "".join(lines)
    
    bounds = find_section_bounds(lines, "dgbdt.model")
    if bounds is None:
        raise ValueError(f"Section [dgbdt.model] not found in {config_path}")
    
    start, end = bounds
    section_lines = lines[start:end]
    
    # Determine which hash key to use (prefer existing, default to expected_hash)
    hash_key = "expected_hash"
    for line in section_lines:
        if 'hash_b3' in line:
            hash_key = "hash_b3"
            break
    
    # Update or insert path and hash
    updated_section = []
    path_updated = False
    hash_updated = False
    
    for line in section_lines:
        if re.match(r'^\s*path\s*=', line):
            updated_section.append(f'path = "{model_path}"\n')
            path_updated = True
        elif re.match(rf'^\s*{re.escape(hash_key)}\s*=', line):
            updated_section.append(f'{hash_key} = "{new_hash}"\n')
            hash_updated = True
        else:
            updated_section.append(line)
    
    # Insert missing keys
    if not path_updated:
        updated_section.insert(1, f'path = "{model_path}"\n')
    if not hash_updated:
        updated_section.append(f'{hash_key} = "{new_hash}"\n')
    
    # Reconstruct file
    new_lines = lines[:start] + updated_section + lines[end:]
    new_content = "".join(new_lines)
    
    if dry_run:
        print(f"[DRY-RUN] Would update {config_path}:")
        print("  Changes in [dgbdt.model] section:")
        for line in updated_section:
            if line.strip() and not line.strip().startswith("#"):
                print(f"    {line.rstrip()}")
    else:
        with open(config_path, "w", encoding="utf-8", newline="") as f:
            f.write(new_content)
        print(f"✓ Updated {config_path}")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Promote fairness model to runtime with hash guard",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Promote v3 model (will fail if hash unchanged)
  python ai_training/promote_fairness_model.py \\
    --model ai_training/ippan_d_gbdt_v3.json \\
    --runtime-dest crates/ai_registry/models/ippan_d_gbdt_v3.json

  # Dry run to see what would change
  python ai_training/promote_fairness_model.py \\
    --model ai_training/ippan_d_gbdt_v3.json \\
    --runtime-dest crates/ai_registry/models/ippan_d_gbdt_v3.json \\
    --dry-run

  # Override guard (use with caution)
  python ai_training/promote_fairness_model.py \\
    --model ai_training/ippan_d_gbdt_v3.json \\
    --runtime-dest crates/ai_registry/models/ippan_d_gbdt_v3.json \\
    --allow-same-hash
        """,
    )
    parser.add_argument(
        "--model",
        type=Path,
        required=True,
        help="Path to trained model JSON file",
    )
    parser.add_argument(
        "--runtime-dest",
        type=str,
        required=True,
        help="Destination path under crates/ai_registry/models/ (use forward slashes)",
    )
    parser.add_argument(
        "--config",
        type=Path,
        default=Path("config/dlc.toml"),
        help="Path to config file (default: config/dlc.toml)",
    )
    parser.add_argument(
        "--allow-same-hash",
        action="store_true",
        help="Allow promotion even if hash matches current pinned hash",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would change without making changes",
    )
    
    args = parser.parse_args()
    
    # Validate model file exists
    if not args.model.exists():
        print(f"ERROR: Model file not found: {args.model}", file=sys.stderr)
        sys.exit(1)
    
    # Validate runtime dest is under crates/ai_registry/models/
    runtime_dest_str = args.runtime_dest.replace("\\", "/")
    if not runtime_dest_str.startswith("crates/ai_registry/models/"):
        print(
            f"ERROR: Runtime destination must be under crates/ai_registry/models/",
            f"Got: {args.runtime_dest}",
            file=sys.stderr,
            sep="\n",
        )
        sys.exit(1)
    
    # Validate config exists
    if not args.config.exists():
        print(f"ERROR: Config file not found: {args.config}", file=sys.stderr)
        sys.exit(1)
    
    # Compute new hash
    print(f"Computing BLAKE3 hash of {args.model}...")
    new_hash = compute_blake3_hash(args.model)
    print(f"New model hash: {new_hash}")
    
    # Extract current hash from config
    current_hash = extract_hash_from_config(args.config)
    if current_hash is None:
        print(
            f"WARNING: No hash found in {args.config} [dgbdt.model] section.",
            "Searched for keys: expected_hash, hash_b3",
            file=sys.stderr,
            sep="\n",
        )
        print("Proceeding with promotion (no current hash to compare)...")
    else:
        print(f"Current pinned hash: {current_hash}")
        
        # Guard check
        if new_hash.lower() == current_hash.lower():
            if not args.allow_same_hash:
                print("", file=sys.stderr)
                print("=" * 70, file=sys.stderr)
                print("REFUSING promotion: hash unchanged.", file=sys.stderr)
                print("", file=sys.stderr)
                print("You did not produce a new model. The hash matches the currently", file=sys.stderr)
                print("pinned model in config. This prevents creating fake 'new' versions.", file=sys.stderr)
                print("", file=sys.stderr)
                print("Options:", file=sys.stderr)
                print("  1. Train with different data/parameters to get a new hash", file=sys.stderr)
                print("  2. Do not bump the version number", file=sys.stderr)
                print("  3. Use --allow-same-hash to override (use with caution)", file=sys.stderr)
                print("=" * 70, file=sys.stderr)
                sys.exit(2)
            else:
                print("WARNING: Hash unchanged, but --allow-same-hash is set. Proceeding...")
    
    # Validate runtime dest directory exists
    runtime_dest_path = Path(args.runtime_dest)
    runtime_dest_path.parent.mkdir(parents=True, exist_ok=True)
    
    # Copy model file
    if args.dry_run:
        print(f"[DRY-RUN] Would copy {args.model} -> {runtime_dest_path}")
    else:
        shutil.copy2(args.model, runtime_dest_path)
        print(f"[OK] Copied model to {runtime_dest_path}")
    
    # Update config
    update_config_section(
        args.config,
        runtime_dest_str,
        new_hash,
        dry_run=args.dry_run,
    )
    
    # Summary
    print("")
    print("=" * 70)
    print("Promotion Summary")
    print("=" * 70)
    print(f"New hash:     {new_hash}")
    if current_hash:
        print(f"Old hash:     {current_hash}")
        print(f"Hash changed: {new_hash.lower() != current_hash.lower()}")
    print(f"Runtime path: {runtime_dest_str}")
    print(f"Config file:  {args.config}")
    if args.dry_run:
        print("Mode:         DRY-RUN (no changes made)")
    else:
        print("Mode:         PROMOTED")
    print("=" * 70)


if __name__ == "__main__":
    main()

