# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Resolve a valid, refreshed credential from the local credential store.

This is the single entry point the rest of aphrody uses to obtain
authorization. It reads the token written by the Antigravity client and hands
back either a raw access token (for the Cloud Code HTTP client) or a
``google.oauth2.credentials.Credentials`` (for the ``google-genai`` Vertex AI
client).

Keyless refresh policy:
    The Antigravity desktop OAuth client is a *confidential* client — Google's
    token endpoint rejects a ``refresh_token`` grant that omits the client
    secret, and aphrody embeds no secret (and no API key). Instead, aphrody
    relies on the Antigravity app keeping the token current in the OS
    credential store and simply **re-reads** it. The google-auth credentials
    returned here override ``refresh`` to do exactly that, so even
    ``google-genai`` — which forces a refresh on every Vertex request — stays
    keyless.
"""

from __future__ import annotations

from datetime import UTC, datetime
from typing import TYPE_CHECKING

import httpx

from aphrody import endpoints
from aphrody.auth import credential_store, oauth
from aphrody.auth.tokens import OAuthToken
from aphrody.errors import NoRefreshTokenError, OAuthServerError

if TYPE_CHECKING:
    from google.oauth2.credentials import Credentials


def _naive_utc_expiry(expiry_iso: str | None) -> datetime | None:
    """Parse an RFC 3339 expiry into the naive-UTC datetime google-auth wants.

    Args:
        expiry_iso: An ISO 8601 timestamp, or ``None``.

    Returns:
        A timezone-naive UTC ``datetime``, or ``None`` when unparseable.
    """
    if not expiry_iso:
        return None
    try:
        parsed = datetime.fromisoformat(expiry_iso)
    except ValueError:
        return None
    if parsed.tzinfo is not None:
        parsed = parsed.astimezone(UTC).replace(tzinfo=None)
    return parsed


def load_token(
    *,
    refresh_if_expired: bool = True,
    persist: bool = True,
    http: httpx.Client | None = None,
) -> OAuthToken:
    """Load a usable token, refreshing it if it has expired.

    When the token has expired, an OAuth ``refresh_token`` grant is attempted
    first; if Google rejects it (the desktop client is confidential and aphrody
    embeds no secret), aphrody falls back to re-reading the credential store,
    where the Antigravity app keeps a current token. Both paths are keyless.

    Args:
        refresh_if_expired: When ``True``, refresh tokens whose expiry has
            passed (or is within the default leeway).
        persist: When ``True``, write a refreshed token to aphrody's private
            cache so later invocations reuse it.
        http: An optional ``httpx.Client`` to reuse; a short-lived one is
            created when omitted.

    Returns:
        A valid :class:`OAuthToken`.
    """
    token = credential_store.read_token()
    if not (refresh_if_expired and token.is_expired()):
        return token

    owns_client = http is None
    client = http or httpx.Client(timeout=30.0)
    refreshed = False
    try:
        try:
            token = oauth.refresh(token, http=client)
            refreshed = True
        except (OAuthServerError, NoRefreshTokenError):
            # Confidential client / no embedded secret: fall back to the token
            # the Antigravity app keeps fresh in the OS credential store.
            token = credential_store.read_token()
    finally:
        if owns_client:
            client.close()

    if persist and refreshed:
        try:
            credential_store.write_cache(token)
        except OSError:
            pass
    return token


def access_token(**kwargs) -> str:
    """Return a valid Bearer access token string.

    Args:
        **kwargs: Forwarded to :func:`load_token`.

    Returns:
        The access token.
    """
    return load_token(**kwargs).access_token


def to_google_credentials(token: OAuthToken) -> Credentials:
    """Adapt an :class:`OAuthToken` into keyless google-auth ``Credentials``.

    The returned object is suitable for ``google-genai`` in Vertex AI mode. Its
    ``refresh`` is overridden to re-read the OS credential store rather than
    call Google's token endpoint, so the confidential desktop client never
    needs a secret. This is what makes Vertex generation work end to end with
    no API key.

    Args:
        token: The token to adapt.

    Returns:
        A google-auth ``Credentials`` whose refresh is keyless.
    """
    from google.oauth2.credentials import Credentials

    class _AntigravityCredentials(Credentials):
        """``Credentials`` that refresh by re-reading the OS credential store."""

        def refresh(self, request: object) -> None:
            """Re-read the Antigravity-maintained token (no OAuth call)."""
            try:
                fresh = credential_store.read_token()
            except Exception:
                return
            self.token = fresh.access_token
            self.expiry = _naive_utc_expiry(fresh.expiry)

    return _AntigravityCredentials(
        token=token.access_token,
        refresh_token=token.refresh_token,
        token_uri=endpoints.OAUTH_TOKEN_ENDPOINT,
        client_id=endpoints.ANTIGRAVITY_CLIENT_ID,
        client_secret=None,
        scopes=list(endpoints.ANTIGRAVITY_SCOPES),
        expiry=_naive_utc_expiry(token.expiry),
    )


def load_google_credentials(**kwargs) -> Credentials:
    """Load a valid token and adapt it into keyless google-auth ``Credentials``.

    Args:
        **kwargs: Forwarded to :func:`load_token`.

    Returns:
        A ready-to-use google-auth ``Credentials``.
    """
    return to_google_credentials(load_token(**kwargs))
