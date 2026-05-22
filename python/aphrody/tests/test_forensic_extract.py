# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.forensic.extract` (pure asar reader, no Electron)."""

from __future__ import annotations

import json
import struct
import types as _t

from aphrody.forensic import extract


def _build_asar(members: dict[str, bytes]) -> bytes:
    """Build a minimal asar archive from ``{relpath: bytes}``."""
    bodies = b""
    files: dict = {}
    offset = 0
    for rel, data in members.items():
        node = files
        parts = rel.split("/")
        for part in parts[:-1]:
            node = node.setdefault(part, {"files": {}})["files"]
        node[parts[-1]] = {"size": len(data), "offset": str(offset)}
        bodies += data
        offset += len(data)
    tree = {"files": files}
    header = json.dumps(tree).encode("utf-8")
    blob = (
        struct.pack("<I", len(header) + 4)
        + struct.pack("<I", len(header))
        + header
    )
    pad = (-len(blob)) % 4
    return blob + b"\x00" * pad + bodies


def test_read_asar_header():
    blob = _build_asar({"index.js": b"console.log(1)"})
    tree, off = extract.read_asar_header(blob)
    assert "index.js" in tree["files"]
    assert off > 0


def test_extract_asar_writes_members(tmp_path):
    blob = _build_asar(
        {
            "extension.js": b'console.log("hello")',
            "sub/config.json": b'{"a":1}',
            "icon.png": b"\x89PNG\r\n",  # binary, skipped in source_only
        }
    )
    arc = tmp_path / "app.asar"
    arc.write_bytes(blob)
    out = tmp_path / "out"
    summary = extract.extract_asar(arc, out)
    assert summary["members_written"] == 2  # png skipped
    # The actual bytes round-trip.
    written = {p: open(p, "rb").read() for p in summary["files"]}
    assert any(b'console.log("hello")' == v for v in written.values())
    assert any(b'{"a":1}' == v for v in written.values())


def test_extract_all_loose_and_asar(tmp_path):
    # An inventory mixing a loose JS file and an asar archive.
    blob = _build_asar({"main.js": b"var x=1"})
    arc = tmp_path / "node_modules.asar"
    arc.write_bytes(blob)
    loose = tmp_path / "extension.js"
    loose.write_text("export const a = 1;", encoding="utf-8")

    entries = [
        _t.SimpleNamespace(
            path=str(arc),
            ext="asar",
            is_dir=False,
            markers=["asar"],
            size=len(blob),
            rel="node_modules.asar",
        ),
        _t.SimpleNamespace(
            path=str(loose),
            ext="js",
            is_dir=False,
            markers=[],
            size=loose.stat().st_size,
            rel="extension.js",
        ),
    ]
    res = extract.extract_all(entries, tmp_path / "work", include_go=False)
    d = res.to_dict()
    assert d["loose_files"] == 1
    assert len(d["asar_archives"]) == 1
    assert d["total_files"] >= 2


def test_extract_all_handles_bad_asar(tmp_path):
    bad = tmp_path / "broken.asar"
    bad.write_bytes(b"not an asar")
    entries = [
        _t.SimpleNamespace(
            path=str(bad),
            ext="asar",
            is_dir=False,
            markers=["asar"],
            size=11,
            rel="broken.asar",
        )
    ]
    res = extract.extract_all(entries, tmp_path / "work", include_go=False)
    assert "error" in res.asar_archives[0]
