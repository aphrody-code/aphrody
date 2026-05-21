# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Google OAuth 2.0 operations: refresh, tokeninfo and userinfo.

The refresh exchange mirrors the proven native path: a form POST to the Google
token endpoint with the Antigravity ``client_id`` and the ``refresh_token``,
**with no client secret** (the desktop client is an installed/PKCE app). This
matches the behaviour validated against the live 2.0.1 client.
"""

from __future__ import annotations

from datetime import UTC, datetime, timedelta

import httpx

from aphrody import endpoints
from aphrody.auth.tokens import OAuthToken
from aphrody.errors import NoRefreshTokenError, OAuthServerError


def refresh(
    token: OAuthToken,
    *,
    http: httpx.Client,
    client_id: str = endpoints.ANTIGRAVITY_CLIENT_ID,
) -> OAuthToken:
    """Exchange a refresh token for a fresh access token.

    Args:
        token: The token whose ``refresh_token`` should be exchanged.
        http: An ``httpx.Client`` used for the request.
        client_id: The OAuth client id to present. Defaults to the Antigravity
            CLI client id.

    Returns:
        A new :class:`OAuthToken` carrying the fresh access token and an
        absolute RFC 3339 ``expiry`` computed from ``expires_in``. The original
        refresh token is preserved when Google does not rotate it.

    Raises:
        NoRefreshTokenError: The input token has no refresh token.
        OAuthServerError: The token endpoint returned a non-2xx response.
    """
    if not token.refresh_token:
        raise NoRefreshTokenError("token has no refresh_token; cannot refresh")

    response = http.post(
        endpoints.OAUTH_TOKEN_ENDPOINT,
        data={
            "client_id": client_id,
            "grant_type": "refresh_token",
            "refresh_token": token.refresh_token,
        },
    )
    if response.status_code // 100 != 2:
        raise OAuthServerError(response.status_code, response.text)

    payload = response.json()
    expiry: str | None = None
    if "expires_in" in payload:
        delta = timedelta(seconds=int(payload["expires_in"]))
        expiry = (datetime.now(UTC) + delta).isoformat()

    return OAuthToken(
        access_token=payload["access_token"],
        refresh_token=payload.get("refresh_token") or token.refresh_token,
        expiry=expiry,
    )


def tokeninfo(access_token: str, *, http: httpx.Client) -> dict:
    """Validate an access token and return its claims.

    Args:
        access_token: The Bearer access token to inspect.
        http: An ``httpx.Client`` used for the request.

    Returns:
        The decoded ``tokeninfo`` claims (``aud``, ``scope``, ``exp``, ...).

    Raises:
        OAuthServerError: The endpoint returned a non-2xx response.
    """
    response = http.get(
        endpoints.OAUTH_TOKENINFO_ENDPOINT,
        params={"access_token": access_token},
    )
    if response.status_code // 100 != 2:
        raise OAuthServerError(response.status_code, response.text)
    return response.json()


def userinfo(access_token: str, *, http: httpx.Client) -> dict:
    """Fetch the signed-in user's OpenID profile (email + name).

    Args:
        access_token: The Bearer access token.
        http: An ``httpx.Client`` used for the request.

    Returns:
        The ``userinfo`` payload (``email``, ``name``, ``picture``, ...).

    Raises:
        OAuthServerError: The endpoint returned a non-2xx response.
    """
    response = http.get(
        endpoints.OAUTH_USERINFO_ENDPOINT,
        headers={"Authorization": f"Bearer {access_token}"},
    )
    if response.status_code // 100 != 2:
        raise OAuthServerError(response.status_code, response.text)
    return response.json()
