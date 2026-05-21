# SPDX-License-Identifier: Apache-2.0
"""Tests for :class:`aphrody.client.AphrodyClient` retry/refresh behavior."""

from __future__ import annotations

import httpx
import pytest
from aphrody.auth import credential_store
from aphrody.auth.tokens import OAuthToken
from aphrody.client import AphrodyClient
from aphrody.errors import ApiError


def test_get_json_success(httpx_mock) -> None:
    httpx_mock.add_response(json={"ok": True})
    with httpx.Client() as http:
        client = AphrodyClient(token=OAuthToken("acc", "ref"), http=http)
        assert client.get_json("https://example.com/x") == {"ok": True}


def test_retry_on_401_refreshes_from_store(httpx_mock, monkeypatch) -> None:
    monkeypatch.setattr(
        credential_store, "read_token", lambda: OAuthToken("fresh", "ref")
    )
    httpx_mock.add_response(status_code=401, json={"error": "unauth"})
    httpx_mock.add_response(json={"ok": True})
    with httpx.Client() as http:
        client = AphrodyClient(
            token=OAuthToken("acc", "ref"), http=http, persist_refresh=False
        )
        assert client.get_json("https://example.com/x") == {"ok": True}
    assert client.token.access_token == "fresh"


def test_non_2xx_raises_api_error(httpx_mock) -> None:
    httpx_mock.add_response(status_code=500, text="boom")
    with httpx.Client() as http:
        client = AphrodyClient(token=OAuthToken("acc"), http=http)
        with pytest.raises(ApiError):
            client.get_json("https://example.com/x")
