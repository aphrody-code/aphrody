# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.auth.cookies`."""

from __future__ import annotations

import json

import pytest
from aphrody.auth import cookies
from aphrody.auth.cookies import Cookie, CookieJar, CookieStoreError


def test_domain_match() -> None:
    assert cookies._domain_match(".google.com", "gemini.google.com")
    assert cookies._domain_match("gemini.google.com", "gemini.google.com")
    assert not cookies._domain_match("play.google.com", "gemini.google.com")


def test_cookie_from_cookie_editor_keys() -> None:
    c = Cookie.from_dict(
        {
            "name": "x",
            "value": "v",
            "domain": ".google.com",
            "httpOnly": True,
            "expirationDate": 123.0,
        }
    )
    assert c.http_only is True
    assert c.expiry == 123.0


def test_header_filters_by_host() -> None:
    jar = CookieJar(
        [Cookie("A", "1", ".google.com"), Cookie("B", "2", "play.google.com")]
    )
    header = jar.header("gemini.google.com")
    assert "A=1" in header
    assert "B=2" not in header


def test_require_raises_when_missing() -> None:
    jar = CookieJar([Cookie("X", "1")])
    with pytest.raises(CookieStoreError):
        jar.require("__Secure-1PSID")


def test_header_no_match_raises() -> None:
    jar = CookieJar([Cookie("A", "1", "example.com")])
    with pytest.raises(CookieStoreError):
        jar.header("gemini.google.com")


def test_save_load_roundtrip(tmp_path) -> None:
    jar = CookieJar([Cookie("__Secure-1PSID", "secret", ".google.com")])
    path = tmp_path / "c.json"
    cookies.save(jar, path)
    loaded = cookies.load(path)
    cookie = loaded.get("__Secure-1PSID")
    assert cookie is not None
    assert cookie.value == "secret"


def test_load_missing_raises(tmp_path) -> None:
    with pytest.raises(CookieStoreError):
        cookies.load(tmp_path / "nope.json")


def test_status_never_leaks_values() -> None:
    jar = CookieJar([Cookie("__Secure-1PSID", "supersecret", ".google.com")])
    status = cookies.status(jar)
    assert status["count"] == 1
    assert status["has_required"] is True
    assert "supersecret" not in json.dumps(status)


def test_import_cookie_editor(tmp_path, monkeypatch) -> None:
    monkeypatch.setenv("APHRODY_COOKIES_PATH", str(tmp_path / "c.json"))
    jar = cookies.import_cookie_editor(
        '[{"name":"__Secure-1PSID","value":"v","domain":".google.com"}]'
    )
    cookie = jar.get("__Secure-1PSID")
    assert cookie is not None
    assert cookie.value == "v"
