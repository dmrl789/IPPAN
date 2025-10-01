#!/usr/bin/env python3
import os, re, sys, difflib, pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
WF = ROOT / ".github" / "workflows"
changes = []

def normalize_doc_start(txt: str) -> str:
    # remove leading '---' with optional BOM/whitespace
    return re.sub(r'^\ufeff?\s*---\s*\n', '', txt, count=1, flags=re.M)

def fix_inline_brackets(txt: str) -> str:
    # Convert inline lists with inner spaces --> no inner spaces (least invasive)
    def repl(m):
        inner = m.group(1)
        compact = re.sub(r'\s*,\s*', ', ', re.sub(r'^\s+|\s+$', '', inner))
        return f'[{compact}]'
    return re.sub(r'\[(.*?)\]', repl, txt)

def process_file(p: pathlib.Path):
    original = p.read_text(encoding="utf-8")
    fixed = original
    fixed = normalize_doc_start(fixed)
    fixed = fix_inline_brackets(fixed)
    if fixed != original:
        p.write_text(fixed, encoding="utf-8")
        diff = ''.join(difflib.unified_diff(
            original.splitlines(True), fixed.splitlines(True),
            fromfile=str(p), tofile=str(p)))
        changes.append(diff)

for yml in WF.glob('*.yml'):
    process_file(yml)
for yaml in WF.glob('*.yaml'):
    process_file(yaml)

if not changes:
    print("NO_CHANGES")
    sys.exit(0)

patch = '\n'.join(changes)
with open("fix-workflows.patch", "w", encoding="utf-8") as f:
    f.write(patch)
print("WROTE: fix-workflows.patch")
