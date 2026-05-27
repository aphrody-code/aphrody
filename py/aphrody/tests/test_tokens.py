# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.auth.tokens`."""

from __future__ import annotations

from datetime import UTC, datetime, timedelta

from aphrody.auth.tokens import OAuthToken


def test_from_envelope_wrapped() -> None:
    tok = OAuthToken.from_envelope(
        {
            "token": {
                "access_token": "a",
                "refresh_token": "r",
                "expiry": "2030-01-01T00:00:00+00:00",
            },
            "auth_method": "consumer",
        }
    )
    assert tok.access_token == "a"
    assert tok.refresh_token == "r"
    assert tok.expiry == "2030-01-01T00:00:00+00:00"


def test_from_envelope_flat() -> None:
    tok = OAuthToken.from_envelope({"access_token": "x"})
    assert tok.access_token == "x"
    assert tok.refresh_token is None


def test_blob_roundtrip() -> None:
    tok = OAuthToken("a", "r", "2030-01-01T00:00:00+00:00")
    assert OAuthToken.from_blob(tok.to_blob()) == tok


def test_to_envelope_shape() -> None:
    env = OAuthToken("a", "r", "2030-01-01T00:00:00+00:00").to_envelope()
    assert env["token"]["token_type"] == "Bearer"
    assert env["auth_method"] == "consumer"
    assert env["token"]["access_token"] == "a"


def test_is_expired_past() -> None:
    past = (datetime.now(UTC) - timedelta(hours=1)).isoformat()
    assert OAuthToken("a", expiry=past).is_expired() is True


def test_is_expired_future() -> None:
    future = (datetime.now(UTC) + timedelta(hours=1)).isoformat()
    assert OAuthToken("a", expiry=future).is_expired() is False


def test_is_expired_unknown() -> None:
    assert OAuthToken("a").is_expired() is False


def test_is_expired_leeway() -> None:
    soon = (datetime.now(UTC) + timedelta(seconds=30)).isoformat()
    assert OAuthToken("a", expiry=soon).is_expired(leeway_seconds=60) is True


def test_access_token_prefix() -> None:
    assert OAuthToken("abcdefghijk").access_token_prefix.startswith("abcdefgh")
    assert OAuthToken("").access_token_prefix == ""


def test_oauth_token_repr_does_not_leak() -> None:
    tok = OAuthToken(
        access_token="secret_access_token_12345",
        refresh_token="secret_refresh_token_67890",
        expiry="2030-01-01",
    )
    rep = repr(tok)
    assert "secret_access_token_12345" not in rep
    assert "secret_refresh_token_67890" not in rep
    assert "secret_a..." in rep
    assert "2030-01-01" in rep
