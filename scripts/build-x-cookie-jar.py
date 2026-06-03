#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""
Import X.com cookies + build ~/.aphrody/x-session.json.

Usage (never commit the export file):
  python3 build-x-cookie-jar.py ~/Downloads/x-cookies.json [--handle yoyo__goat]

Writes (mode 600):
  ~/.bxc/cookies/xcom.json
  ~/.aphrody/cookies/xcom.json
  ~/.aphrody/x-session.json  (auth_token + ct0)
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("export", help="Playwright/CDP JSON cookie array export")
    p.add_argument("--handle", default="", help="Optional @handle for session metadata")
    args = p.parse_args()

    raw = json.loads(Path(args.export).expanduser().read_text(encoding="utf-8"))
    if not isinstance(raw, list):
        sys.exit("expected JSON array")

    auth = ct0 = ""
    for c in raw:
        if c.get("name") == "auth_token":
            auth = c.get("value", "")
        if c.get("name") == "ct0":
            ct0 = c.get("value", "")
    if not auth or not ct0:
        sys.exit("export must include auth_token and ct0 for .x.com")

    session: dict[str, str] = {"auth_token": auth, "ct0": ct0}
    if args.handle:
        session["handle"] = args.handle.lstrip("@")

    bxc_path = Path.home() / ".bxc" / "cookies" / "xcom.json"
    aph_cookies = Path.home() / ".aphrody" / "cookies" / "xcom.json"
    aph_session = Path.home() / ".aphrody" / "x-session.json"
    for path in (bxc_path, aph_cookies, aph_session):
        path.parent.mkdir(parents=True, exist_ok=True)

    payload = json.dumps(raw, indent=2) + "\n"
    bxc_path.write_text(payload, encoding="utf-8")
    aph_cookies.write_text(payload, encoding="utf-8")
    aph_session.write_text(json.dumps(session, indent=2) + "\n", encoding="utf-8")
    for path in (bxc_path, aph_cookies, aph_session):
        os.chmod(path, 0o600)

    print(json.dumps({"cookies": str(bxc_path), "session": str(aph_session), "count": len(raw)}))


if __name__ == "__main__":
    main()