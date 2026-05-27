# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.forensic.inventory` (pure, no network)."""

from __future__ import annotations

import json
import struct

from aphrody.forensic import inventory as inv


def test_sqlite_marker(tmp_path):
    f = tmp_path / "state.db"
    f.write_bytes(inv.SQLITE_MAGIC + b"\x00" * 100)
    e = inv.inventory_entry(f, tmp_path)
    assert "sqlite" in e.markers


def test_pe_cpp_marker(tmp_path):
    f = tmp_path / "native.dll"
    f.write_bytes(b"MZ" + b"\x00" * 64 + b"Rich" + b"VCRUNTIME140.dll\x00")
    e = inv.inventory_entry(f, tmp_path)
    assert "pe" in e.markers
    assert "cpp" in e.markers
    assert "go" not in e.markers


def test_pe_go_marker(tmp_path):
    f = tmp_path / "language_server.exe"
    f.write_bytes(b"MZ" + b"\x00" * 64 + inv.GO_BUILDINFO_MAGIC + b"go1.23.4")
    e = inv.inventory_entry(f, tmp_path)
    assert "pe" in e.markers
    assert "go" in e.markers
    assert e.details["go"]["go_version"] == "go1.23.4"


def test_secret_token_value_is_read(tmp_path):
    f = tmp_path / "oauth_creds.json"
    payload = {
        "access_token": "ya29.REALTOKENBYTES",
        "refresh_token": "1//refreshreal",
        "scope": "cloud-platform aicode",
        "expiry": "2026-05-22T10:00:00Z",
    }
    f.write_text(json.dumps(payload), encoding="utf-8")
    e = inv.inventory_entry(f, tmp_path)
    assert "secret" in e.markers
    sec = e.details["secret"]
    # Full real value captured (no redaction in full mode).
    assert sec["access_token"] == "ya29.REALTOKENBYTES"
    assert sec["refresh_token"] == "1//refreshreal"
    assert "cloud-platform" in sec["scope"]


def test_secret_raw_token(tmp_path):
    f = tmp_path / "access.token"
    f.write_text("ya29.RAWBEARER", encoding="utf-8")
    e = inv.inventory_entry(f, tmp_path)
    assert e.details["secret"]["token_type"] == "raw"
    assert e.details["secret"]["value"] == "ya29.RAWBEARER"


def test_google_markers(tmp_path):
    f = tmp_path / "config.js"
    f.write_text(
        "const h='https://cloudcode-pa.googleapis.com';"
        "const c='123456-abcdef.apps.googleusercontent.com';",
        encoding="utf-8",
    )
    e = inv.inventory_entry(f, tmp_path)
    assert "google" in e.markers
    g = e.details["google"]
    assert "cloudcode-pa.googleapis.com" in g["googleapis_hosts"]
    assert "123456-abcdef.apps.googleusercontent.com" in g["oauth_client_ids"]


def test_product_json_electron(tmp_path):
    f = tmp_path / "product.json"
    f.write_text(
        json.dumps(
            {"nameLong": "Antigravity IDE", "version": "2.0.2", "commit": "abc"}
        ),
        encoding="utf-8",
    )
    e = inv.inventory_entry(f, tmp_path)
    assert "electron" in e.markers
    assert e.details["product"]["version"] == "2.0.2"


def test_dpapi_dir_marker(tmp_path):
    d = tmp_path / "Protect"
    d.mkdir()
    e = inv.inventory_entry(d, tmp_path)
    assert e.is_dir
    assert "dpapi_dir" in e.markers


def test_walk_inventory_summary(tmp_path):
    (tmp_path / "a.py").write_text("print(1)", encoding="utf-8")
    (tmp_path / "db.sqlite").write_bytes(inv.SQLITE_MAGIC + b"\x00" * 10)
    sub = tmp_path / "sub"
    sub.mkdir()
    (sub / "tok.token").write_text("secretvalue", encoding="utf-8")

    result = inv.walk_inventory(tmp_path)
    summ = result.summary
    assert summ["files"] >= 3
    assert summ["markers"].get("sqlite", 0) >= 1
    assert summ["markers"].get("secret", 0) >= 1
    # secret_files list never includes the raw value.
    for s in summ["secret_files"]:
        assert "value" not in s

    # Full report is JSON-serialisable.
    json.dumps(result.to_dict())


def test_walk_single_file(tmp_path):
    f = tmp_path / "lone.py"
    f.write_text("x = 1", encoding="utf-8")
    result = inv.walk_inventory(f)
    assert result.summary["files"] == 1


def test_asar_header_roundtrip(tmp_path):
    # Build a minimal asar and confirm the inventory + reader agree on layout.
    body = b'console.log("hi")'
    tree = {"files": {"index.js": {"size": len(body), "offset": "0"}}}
    header = json.dumps(tree).encode("utf-8")
    blob = (
        struct.pack("<I", len(header) + 4)
        + struct.pack("<I", len(header))
        + header
    )
    pad = (-len(blob)) % 4
    blob += b"\x00" * pad + body
    f = tmp_path / "app.asar"
    f.write_bytes(blob)
    e = inv.inventory_entry(f, tmp_path)
    assert "asar" in e.markers
