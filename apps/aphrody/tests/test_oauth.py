# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.auth.oauth` using a mocked HTTP transport."""

from __future__ import annotations

import httpx
import pytest
from aphrody import endpoints
from aphrody.auth import oauth
from aphrody.auth.tokens import OAuthToken
from aphrody.errors import NoRefreshTokenError, OAuthServerError


def test_refresh_success(httpx_mock) -> None:
    httpx_mock.add_response(
        url=endpoints.OAUTH_TOKEN_ENDPOINT,
        json={"access_token": "new", "expires_in": 3600},
    )
    with httpx.Client() as client:
        tok = oauth.refresh(OAuthToken("old", "ref"), http=client)
    assert tok.access_token == "new"
    assert tok.refresh_token == "ref"  # preserved when not rotated
    assert tok.expiry is not None


def test_refresh_without_refresh_token() -> None:
    with httpx.Client() as client, pytest.raises(NoRefreshTokenError):
        oauth.refresh(OAuthToken("old", None), http=client)


def test_refresh_server_error(httpx_mock) -> None:
    httpx_mock.add_response(
        url=endpoints.OAUTH_TOKEN_ENDPOINT,
        status_code=400,
        text='{"error": "invalid_request"}',
    )
    with httpx.Client() as client, pytest.raises(OAuthServerError):
        oauth.refresh(OAuthToken("old", "ref"), http=client)


def test_tokeninfo(httpx_mock) -> None:
    httpx_mock.add_response(json={"aud": "client", "scope": "a b"})
    with httpx.Client() as client:
        info = oauth.tokeninfo("acc", http=client)
    assert info["aud"] == "client"
