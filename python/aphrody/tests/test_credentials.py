# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.auth.credentials` (keyless refresh)."""

from __future__ import annotations

from datetime import datetime

from aphrody.auth import credentials
from aphrody.auth.tokens import OAuthToken
from aphrody.errors import OAuthServerError


def test_naive_utc_expiry() -> None:
    # +02:00 → naive UTC is two hours earlier.
    assert credentials._naive_utc_expiry("2030-01-01T00:00:00+02:00") == (
        datetime(2029, 12, 31, 22, 0, 0)
    )
    assert credentials._naive_utc_expiry(None) is None
    assert credentials._naive_utc_expiry("not-a-date") is None


def test_to_google_credentials_refreshes_from_store(monkeypatch) -> None:
    creds = credentials.to_google_credentials(
        OAuthToken("acc", "ref", "2030-01-01T00:00:00+00:00")
    )
    assert creds.token == "acc"
    monkeypatch.setattr(
        credentials.credential_store,
        "read_token",
        lambda: OAuthToken("fresh", "ref", "2030-01-01T00:00:00+00:00"),
    )
    creds.refresh(object())
    assert creds.token == "fresh"


def test_load_token_falls_back_on_refresh_error(monkeypatch) -> None:
    expired = OAuthToken("old", "ref", "2000-01-01T00:00:00+00:00")
    monkeypatch.setattr(
        credentials.credential_store, "read_token", lambda: expired
    )

    def boom(token, *, http):
        raise OAuthServerError(400, "client_secret is missing")

    monkeypatch.setattr(credentials.oauth, "refresh", boom)
    tok = credentials.load_token(persist=False)
    assert tok.access_token == "old"
