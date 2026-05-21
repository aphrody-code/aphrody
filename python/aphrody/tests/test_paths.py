# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody._paths`."""

from __future__ import annotations

from pathlib import Path

from aphrody import _paths


def test_env_override(monkeypatch, tmp_path) -> None:
    monkeypatch.setenv("APHRODY_SECRETS_DIR", str(tmp_path))
    assert _paths.secrets_dir() == tmp_path
    assert _paths.secret_file("x.json") == tmp_path / "x.json"


def test_repo_var_secrets(monkeypatch, tmp_path) -> None:
    monkeypatch.delenv("APHRODY_SECRETS_DIR", raising=False)
    (tmp_path / ".git").mkdir()
    monkeypatch.chdir(tmp_path)
    assert _paths.secrets_dir() == tmp_path / "var" / "secrets"


def test_fallback_to_home(monkeypatch, tmp_path) -> None:
    monkeypatch.delenv("APHRODY_SECRETS_DIR", raising=False)
    sub = tmp_path / "no-repo-here"
    sub.mkdir()
    monkeypatch.chdir(sub)
    # tmp_path has no .git ancestor, so resolution falls back to ~/.aphrody.
    assert _paths.secrets_dir() == Path.home() / ".aphrody"
