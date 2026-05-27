# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.forensic.classify` (mocked Magika)."""

from __future__ import annotations

import types as _t

from aphrody.forensic import classify, inventory


class _FakeOutput:
    def __init__(self, label, mime, group, is_text):
        self.label = label
        self.mime_type = mime
        self.group = group
        self.is_text = is_text


class _FakeResult:
    def __init__(self, label, mime, group, is_text, score, ok=True):
        self.ok = ok
        self.output = _FakeOutput(label, mime, group, is_text)
        self.score = score
        self.prediction = _t.SimpleNamespace(score=score)


class _FakeMagika:
    def __init__(self, mapping):
        # mapping: filename substring -> result tuple
        self._mapping = mapping

    def identify_path(self, path):
        for key, res in self._mapping.items():
            if key in str(path):
                return res
        return _FakeResult("unknown", "", "", False, 0.0)


def test_classify_path_python():
    m = _FakeMagika(
        {"x.py": _FakeResult("python", "text/x-python", "code", True, 0.99)}
    )
    cls = classify.classify_path("dir/x.py", magika=m)
    assert cls.label == "python"
    assert cls.group == "code"
    assert cls.score == 0.99
    assert cls.category == "code"


def test_classify_entries_fuses_markers(tmp_path):
    f = tmp_path / "language_server.exe"
    f.write_bytes(
        b"MZ" + b"\x00" * 64 + inventory.GO_BUILDINFO_MAGIC + b"go1.23"
    )
    entries = inventory.walk_inventory(tmp_path).entries
    m = _FakeMagika(
        {
            "language_server.exe": _FakeResult(
                "pebin", "application/x-dosexec", "executable", False, 0.95
            )
        }
    )
    out = classify.classify_entries(entries, magika=m)
    # The go marker wins over the magika 'executable' group.
    go = next(c for c in out if c.path.endswith("language_server.exe"))
    assert go.category == "go-binary"


def test_classify_secret_category(tmp_path):
    f = tmp_path / "oauth_creds.json"
    f.write_text('{"access_token":"t"}', encoding="utf-8")
    entries = inventory.walk_inventory(tmp_path).entries
    m = _FakeMagika(
        {
            "oauth_creds.json": _FakeResult(
                "json", "application/json", "code", True, 0.97
            )
        }
    )
    out = classify.classify_entries(entries, magika=m)
    sec = next(c for c in out if c.path.endswith("oauth_creds.json"))
    assert sec.category == "secret"


def test_aggregate():
    cls = [
        classify.Classification(
            "a", "python", "", "code", 0.9, True, [], "code"
        ),
        classify.Classification("b", "json", "", "code", 0.9, True, [], "code"),
        classify.Classification(
            "c", "pebin", "", "executable", 0.9, False, ["go"], "go-binary"
        ),
    ]
    agg = classify.aggregate(cls)
    assert agg["total"] == 3
    assert agg["by_category"]["code"] == 2
    assert agg["by_category"]["go-binary"] == 1


def test_unreadable_result():
    m = _FakeMagika({"bad": _FakeResult("", "", "", False, 0.0, ok=False)})
    cls = classify.classify_path("bad", magika=m)
    assert cls.category == "unreadable"
