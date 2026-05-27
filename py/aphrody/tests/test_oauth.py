# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.auth.oauth` using a mocked HTTP transport."""

from __future__ import annotations

import httpx
import pytest
from aphrody.auth import oauth
from aphrody.auth.tokens import OAuthToken
from aphrody.errors import NoRefreshTokenError, OAuthServerError

from aphrody import endpoints


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


def test_refresh_retry_on_429_then_success(httpx_mock, monkeypatch) -> None:
    httpx_mock.add_response(
        url=endpoints.OAUTH_TOKEN_ENDPOINT,
        status_code=429,
        text="Rate limit exceeded",
    )
    httpx_mock.add_response(
        url=endpoints.OAUTH_TOKEN_ENDPOINT,
        status_code=200,
        json={"access_token": "new", "expires_in": 3600},
    )

    sleep_calls = []
    monkeypatch.setattr("time.sleep", sleep_calls.append)

    with httpx.Client() as client:
        tok = oauth.refresh(
            OAuthToken("old", "ref"),
            http=client,
            max_retries=2,
            retry_delay=1.0,
        )

    assert tok.access_token == "new"
    assert len(sleep_calls) == 1
    assert 0.5 <= sleep_calls[0] <= 1.5


def test_refresh_retry_on_503_then_success(httpx_mock, monkeypatch) -> None:
    httpx_mock.add_response(
        url=endpoints.OAUTH_TOKEN_ENDPOINT,
        status_code=503,
        text="Service Unavailable",
    )
    httpx_mock.add_response(
        url=endpoints.OAUTH_TOKEN_ENDPOINT,
        status_code=503,
        text="Service Unavailable",
    )
    httpx_mock.add_response(
        url=endpoints.OAUTH_TOKEN_ENDPOINT,
        status_code=200,
        json={"access_token": "new", "expires_in": 3600},
    )

    sleep_calls = []
    monkeypatch.setattr("time.sleep", sleep_calls.append)

    with httpx.Client() as client:
        tok = oauth.refresh(
            OAuthToken("old", "ref"),
            http=client,
            max_retries=3,
            retry_delay=1.0,
            max_retry_delay=8.0,
        )

    assert tok.access_token == "new"
    assert len(sleep_calls) == 2
    assert 0.5 <= sleep_calls[0] <= 1.5
    assert 1.0 <= sleep_calls[1] <= 3.0


def test_refresh_retry_exhausted(httpx_mock, monkeypatch) -> None:
    for _ in range(4):
        httpx_mock.add_response(
            url=endpoints.OAUTH_TOKEN_ENDPOINT,
            status_code=503,
            text="Service Unavailable",
        )

    sleep_calls = []
    monkeypatch.setattr("time.sleep", sleep_calls.append)

    with httpx.Client() as client:
        with pytest.raises(OAuthServerError) as excinfo:
            oauth.refresh(
                OAuthToken("old", "ref"),
                http=client,
                max_retries=3,
                retry_delay=1.0,
            )

    assert excinfo.value.status == 503
    assert len(sleep_calls) == 3


def test_refresh_other_server_error_not_retried(
    httpx_mock, monkeypatch
) -> None:
    httpx_mock.add_response(
        url=endpoints.OAUTH_TOKEN_ENDPOINT,
        status_code=500,
        text="Internal Server Error",
    )

    sleep_calls = []
    monkeypatch.setattr("time.sleep", sleep_calls.append)

    with httpx.Client() as client:
        with pytest.raises(OAuthServerError) as excinfo:
            oauth.refresh(
                OAuthToken("old", "ref"),
                http=client,
                max_retries=3,
                retry_delay=1.0,
            )

    assert excinfo.value.status == 500
    assert len(sleep_calls) == 0
