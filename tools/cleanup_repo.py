#!/usr/bin/env python3
import sys
import os
import shutil
import fnmatch
import yaml
from pathlib import Path

ROOT = Path(__file__).parent.parent.absolute()

def globpaths(pattern):
    """Recursive glob respecting ** patterns"""
    matches = []
    for base, dirs, files in os.walk(ROOT):
        rel = os.path.relpath(base, ROOT)
        for name in dirs + files:
            p = os.path.join(rel, name) if rel != "." else name
            if fnmatch.fnmatchcase(p.replace("\\", "/"), pattern):
                matches.append(os.path.join(ROOT, p))
    return list(set(matches))

def main():
    dry = "--apply" not in sys.argv
    print(f"Running in {'DRY-RUN' if dry else 'APPLY'} mode")
    
    try:
        with open(ROOT / "repo.keep.yaml", "r", encoding="utf-8") as f:
            cfg = yaml.safe_load(f)
    except FileNotFoundError:
        print("Error: repo.keep.yaml not found")
        return 1
    
    # Build keep set
    keep = set()
    for k in cfg.get("keep", []):
        keep.update(globpaths(k))
    
    # Handle remove_globs
    remove = []
    excludes = []
    for g in cfg.get("remove_globs", []):
        if g.startswith("!"):
            excludes += globpaths(g[1:])
        else:
            remove += globpaths(g)
    
    # Filter out kept items and excluded items
    to_remove = []
    for path in remove:
        if any(path.startswith(k) for k in keep):
            continue
        if path in excludes:
            continue
        to_remove.append(path)
    
    # Dedupe and sort deepest first
    to_remove = sorted(set(to_remove), key=lambda p: (-p.count(os.sep), p))
    
    print(f"Found {len(to_remove)} items to remove")
    
    for p in to_remove:
        rel_path = os.path.relpath(p, ROOT)
        if os.path.exists(p):
            print(f"{'would remove:' if dry else 'removing:'} {rel_path}")
            if not dry:
                try:
                    if os.path.isdir(p):
                        shutil.rmtree(p, ignore_errors=True)
                    elif os.path.isfile(p):
                        os.remove(p)
                except Exception as e:
                    print(f"  Error removing {rel_path}: {e}")
        else:
            print(f"  (not found) {rel_path}")
    
    if dry:
        print("\nRun with --apply to actually remove files")
    else:
        print(f"\nRemoved {len(to_remove)} items")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
