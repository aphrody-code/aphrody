# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.auth.credential_store` (agy OAuth paths)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from aphrody.auth import credential_store
from aphrody.auth.tokens import OAuthToken
from aphrody.errors import UnsupportedPlatformError


def _agy_envelope(access: str = "ya29.test", refresh: str = "1//ref") -> bytes:
    return json.dumps(
        {
            "token": {
                "access_token": access,
                "refresh_token": refresh,
                "expiry": "2030-01-01T00:00:00+00:00",
                "token_type": "Bearer",
            },
            "auth_method": "consumer",
        }
    ).encode()


def test_agy_oauth_path_default(monkeypatch) -> None:
    monkeypatch.delenv("APHRODY_AGY_OAUTH_FILE", raising=False)
    p = credential_store.agy_oauth_path()
    assert p == Path.home() / ".gemini" / "antigravity-cli" / "antigravity-oauth-token"


def test_agy_oauth_path_env_override(monkeypatch, tmp_path: Path) -> None:
    custom = tmp_path / "custom-oauth.json"
    monkeypatch.setenv("APHRODY_AGY_OAUTH_FILE", str(custom))
    assert credential_store.agy_oauth_path() == custom


def test_aphrody_login_token_path_xdg(monkeypatch, tmp_path: Path) -> None:
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path))
    assert credential_store.aphrody_login_token_path() == (
        tmp_path / "aphrody" / "antigravity-token.json"
    )


def test_token_search_paths_order(monkeypatch, tmp_path: Path) -> None:
    monkeypatch.setenv("APHRODY_SECRETS_DIR", str(tmp_path / "secrets"))
    paths = credential_store.token_search_paths()
    assert paths[0] == credential_store.agy_oauth_path()
    assert paths[1] == credential_store.aphrody_login_token_path()
    assert paths[2] == credential_store.cache_path()
    assert len(paths) == len(set(paths))


def test_read_file_token_parses_agy_envelope(tmp_path: Path) -> None:
    f = tmp_path / "antigravity-oauth-token"
    f.write_bytes(_agy_envelope("acc1"))
    tok = credential_store.read_file_token(f)
    assert tok is not None
    assert tok.access_token == "acc1"
    assert tok.refresh_token == "1//ref"


def test_read_token_from_paths_prefers_first_existing(
    monkeypatch, tmp_path: Path
) -> None:
    agy = tmp_path / "agy.json"
    cache = tmp_path / "cache.json"
    agy.write_bytes(_agy_envelope("from-agy"))
    cache.write_bytes(_agy_envelope("from-cache"))
    monkeypatch.setattr(
        credential_store,
        "token_search_paths",
        lambda: [agy, cache],
    )
    tok = credential_store.read_token_from_paths()
    assert tok is not None
    assert tok.access_token == "from-agy"


@pytest.mark.skipif(
    credential_store._IS_WINDOWS,
    reason="non-Windows read_token uses file search paths",
)
def test_read_token_non_windows_uses_agy_file(monkeypatch, tmp_path: Path) -> None:
    agy = tmp_path / "antigravity-oauth-token"
    agy.write_bytes(_agy_envelope("linux-acc"))
    monkeypatch.setattr(credential_store, "agy_oauth_path", lambda: agy)
    monkeypatch.setattr(
        credential_store,
        "token_search_paths",
        lambda: [agy],
    )
    tok = credential_store.read_token()
    assert tok.access_token == "linux-acc"


@pytest.mark.skipif(
    credential_store._IS_WINDOWS,
    reason="non-Windows read_token uses file search paths",
)
def test_read_token_non_windows_missing_raises(monkeypatch, tmp_path: Path) -> None:
    missing = tmp_path / "missing.json"
    monkeypatch.setattr(
        credential_store,
        "token_search_paths",
        lambda: [missing],
    )
    with pytest.raises(UnsupportedPlatformError) as exc:
        credential_store.read_token()
    assert "agy" in str(exc.value).lower() or "antigravity" in str(exc.value).lower()