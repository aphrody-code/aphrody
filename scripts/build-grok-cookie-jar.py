#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""
Convert a Playwright/CDP JSON cookie export to bxc + aphrody jars.

Usage (never commit the input export):
  python3 build-grok-cookie-jar.py ~/Downloads/grok-cookies.json

Writes (mode 600):
  ~/.bxc/cookies/grok.json
  ~/.aphrody/cookies/grok.json
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path


def main() -> None:
    if len(sys.argv) != 2:
        print("usage: build-grok-cookie-jar.py <cookies-export.json>", file=sys.stderr)
        sys.exit(2)
    src = Path(sys.argv[1]).expanduser()
    raw = json.loads(src.read_text(encoding="utf-8"))
    if not isinstance(raw, list):
        raise SystemExit("expected JSON array of cookie objects")

    bxc_path = Path.home() / ".bxc" / "cookies" / "grok.json"
    aph_path = Path.home() / ".aphrody" / "cookies" / "grok.json"
    bxc_path.parent.mkdir(parents=True, exist_ok=True)
    aph_path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(raw, indent=2) + "\n"
    bxc_path.write_text(payload, encoding="utf-8")
    aph_path.write_text(payload, encoding="utf-8")
    os.chmod(bxc_path, 0o600)
    os.chmod(aph_path, 0o600)
    print(json.dumps({"bxc": str(bxc_path), "aphrody": str(aph_path), "count": len(raw)}))


if __name__ == "__main__":
    main()