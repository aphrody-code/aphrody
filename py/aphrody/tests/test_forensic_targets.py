# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.forensic.targets`."""

from __future__ import annotations

from pathlib import Path

from aphrody.forensic import targets


def test_known_targets_resolve():
    known = targets.known_targets()
    assert set(known) == {"install", "appdata", "dotdir", "agy", "gemini"}
    for path in known.values():
        assert isinstance(path, str)
        # Resolution expanded any %VAR% / ~ (no leftover template markers).
        assert "%" not in path or path  # tolerate hosts without the var


def test_resolve_literal_path(tmp_path):
    p = targets.resolve_target(str(tmp_path))
    assert p == Path(str(tmp_path))


def test_resolve_named_target():
    p = targets.resolve_target("install")
    assert isinstance(p, Path)
    assert "Antigravity IDE" in str(p)


def test_resolve_agy_target_linux():
    import os

    if os.name == "nt":
        pytest.skip("Linux-specific agy home path")
    p = targets.resolve_target("agy")
    assert "antigravity-cli" in str(p).replace("\\", "/")


def test_default_targets_returns_existing(tmp_path, monkeypatch):
    # Point one known target at an existing dir to assert filtering works.
    monkeypatch.setitem(targets._TARGETS, "install", str(tmp_path))
    existing = targets.default_targets()
    assert any(Path(p) == tmp_path for p in existing)
