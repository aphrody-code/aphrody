#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Scan the aphrody repo for broken *relative* Markdown links.

A doc that links to a file that no longer exists is the #1 trust-killer for a
reader skimming the repo (the README "30-second test"). This is a CI-able gate:
it exits non-zero when any relative link in a tracked Markdown file points at a
path that does not exist on disk.

What it checks:
  - `[label](relative/path.md)` and `[label](./path)` style links.
  - Anchors (`#frag`) are stripped before resolving (`file.md#section` -> file.md).
  - External links (http/https/mailto/file:) and bare anchors (`#x`) are skipped.

Usage:
  scripts/scan-doc-links.py                 # scan README + root *.md + docs/**.md
  scripts/scan-doc-links.py --all           # scan every *.md in the repo (slower)
  scripts/scan-doc-links.py path/a.md b.md  # scan specific files
  scripts/scan-doc-links.py --json          # machine-readable output

Exit code: 0 = no broken links, 1 = broken links found, 2 = bad invocation.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys

# Repo root = parent of this script's directory (scripts/..).
REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

LINK_RE = re.compile(r"\[[^\]]*\]\(([^)]+)\)")
SKIP_PREFIXES = ("http://", "https://", "mailto:", "file:", "#", "<")
# Directories never worth scanning for first-party docs.
PRUNE_DIRS = {".git", "node_modules", "target", "dist", ".turbo", "vendor", ".venv"}


def default_files() -> list[str]:
    files = []
    readme = os.path.join(REPO_ROOT, "README.md")
    if os.path.exists(readme):
        files.append(readme)
    for name in sorted(os.listdir(REPO_ROOT)):
        if name.endswith(".md"):
            files.append(os.path.join(REPO_ROOT, name))
    docs = os.path.join(REPO_ROOT, "docs")
    if os.path.isdir(docs):
        for dirpath, dirnames, filenames in os.walk(docs):
            dirnames[:] = [d for d in dirnames if d not in PRUNE_DIRS]
            for fn in sorted(filenames):
                if fn.endswith(".md"):
                    files.append(os.path.join(dirpath, fn))
    # De-dup while preserving order.
    seen, out = set(), []
    for f in files:
        if f not in seen:
            seen.add(f)
            out.append(f)
    return out


def all_markdown() -> list[str]:
    out = []
    for dirpath, dirnames, filenames in os.walk(REPO_ROOT):
        dirnames[:] = [d for d in dirnames if d not in PRUNE_DIRS]
        for fn in sorted(filenames):
            if fn.endswith(".md"):
                out.append(os.path.join(dirpath, fn))
    return out


def scan_file(path: str) -> list[tuple[int, str]]:
    """Return [(line_number, broken_url)] for one file."""
    base = os.path.dirname(path)
    try:
        text = open(path, encoding="utf-8", errors="replace").read()
    except OSError as exc:  # unreadable file
        return [(0, f"<unreadable: {exc}>")]
    broken = []
    for m in LINK_RE.finditer(text):
        url = m.group(1).split()[0].strip()  # drop optional "title" after a space
        if not url or url.startswith(SKIP_PREFIXES):
            continue
        target = url.split("#", 1)[0]
        if not target:  # pure in-page anchor
            continue
        resolved = os.path.normpath(os.path.join(base, target))
        if not os.path.exists(resolved):
            line = text[: m.start()].count("\n") + 1
            broken.append((line, url))
    return broken


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("files", nargs="*", help="specific Markdown files to scan")
    ap.add_argument("--all", action="store_true", help="scan every *.md in the repo")
    ap.add_argument("--json", action="store_true", help="emit JSON instead of text")
    args = ap.parse_args(argv)

    if args.files:
        files = [os.path.abspath(f) for f in args.files]
    elif args.all:
        files = all_markdown()
    else:
        files = default_files()

    findings: dict[str, list[tuple[int, str]]] = {}
    for f in files:
        broken = scan_file(f)
        if broken:
            findings[os.path.relpath(f, REPO_ROOT)] = broken

    total = sum(len(v) for v in findings.values())

    if args.json:
        print(json.dumps({
            "scanned": len(files),
            "broken": total,
            "findings": {k: [{"line": ln, "url": u} for ln, u in v] for k, v in findings.items()},
        }, indent=2))
    else:
        for rel, broken in findings.items():
            for ln, url in broken:
                print(f"{rel}:{ln}  ->  {url}")
        print(f"\nscanned {len(files)} files, {total} broken relative link(s)")

    return 1 if total else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
