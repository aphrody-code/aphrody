# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""The :class:`OAuthToken` value type and its (de)serialization helpers.

The Antigravity CLI (``agy``) stores the Google OAuth token as UTF-8 JSON with
the shape::

    {
      "token": {
        "access_token": "ya29.…",
        "refresh_token": "1//…",
        "expiry": "2026-05-21T16:34:15.517684+02:00",
        "token_type": "Bearer"
      },
      "auth_method": "consumer"
    }

This module parses that envelope and normalizes the data into a small, typed
dataclass that knows how to tell whether it has expired.
"""

from __future__ import annotations

import dataclasses
import json
from datetime import UTC, datetime


@dataclasses.dataclass(slots=True)
class OAuthToken:
    """A Google OAuth 2.0 token pair.

    Attributes:
        access_token: Short-lived Bearer token for API requests.
        refresh_token: Long-lived token used to mint new access tokens. May be
            ``None`` when the source did not provide one.
        expiry: RFC 3339 timestamp at which ``access_token`` stops being valid.
            May be ``None`` when unknown.
    """

    access_token: str
    refresh_token: str | None = None
    expiry: str | None = None

    @classmethod
    def from_envelope(cls, data: dict) -> OAuthToken:
        """Build a token from a ``{"token": {...}}`` envelope or a flat dict.

        Args:
            data: A dict that either wraps the token under a ``"token"`` key
                (the credential-manager shape) or is itself the token object.

        Returns:
            The parsed :class:`OAuthToken`.

        Raises:
            KeyError: If no ``access_token`` field is present.
        """
        token = data.get("token", data) if isinstance(data, dict) else {}
        return cls(
            access_token=token["access_token"],
            refresh_token=token.get("refresh_token"),
            expiry=token.get("expiry"),
        )

    @classmethod
    def from_blob(cls, blob: bytes | str) -> OAuthToken:
        """Parse a raw JSON credential blob into a token.

        Args:
            blob: The UTF-8 JSON blob, as bytes or str.

        Returns:
            The parsed :class:`OAuthToken`.
        """
        text = blob.decode("utf-8") if isinstance(blob, bytes) else blob
        return cls.from_envelope(json.loads(text))

    def to_envelope(self) -> dict:
        """Serialize back into the credential-manager envelope shape.

        Returns:
            A dict ready to be JSON-encoded and written to the credential
            store, preserving the ``token_type``/``auth_method`` fields the
            Antigravity client expects.
        """
        inner: dict[str, object] = {
            "access_token": self.access_token,
            "token_type": "Bearer",
        }
        if self.refresh_token is not None:
            inner["refresh_token"] = self.refresh_token
        if self.expiry is not None:
            inner["expiry"] = self.expiry
        return {"token": inner, "auth_method": "consumer"}

    def to_blob(self) -> bytes:
        """Serialize the token to a UTF-8 JSON credential blob."""
        return json.dumps(self.to_envelope()).encode("utf-8")

    def is_expired(self, leeway_seconds: int = 60) -> bool:
        """Report whether the access token has expired (or is about to).

        A ``leeway`` is applied so callers refresh slightly early rather than
        racing the exact expiry instant.

        Args:
            leeway_seconds: Treat the token as expired this many seconds before
                its real expiry.

        Returns:
            ``True`` if expired/near-expiry; ``False`` if still valid or if the
            expiry is unknown (in which case callers should rely on a 401 to
            trigger a refresh).
        """
        if not self.expiry:
            return False
        try:
            exp = datetime.fromisoformat(self.expiry)
        except ValueError:
            return False
        if exp.tzinfo is None:
            exp = exp.replace(tzinfo=UTC)
        now = datetime.now(UTC)
        return (now.timestamp() + leeway_seconds) >= exp.timestamp()

    @property
    def access_token_prefix(self) -> str:
        """A short, non-sensitive prefix of the access token for logging."""
        return self.access_token[:8] + "…" if self.access_token else ""

    def __repr__(self) -> str:
        """A safe representation that does not leak sensitive tokens."""
        masked_access = (
            f"{self.access_token[:8]}..." if self.access_token else ""
        )
        masked_refresh = "..." if self.refresh_token else "None"
        return (
            f"OAuthToken(access_token={masked_access!r}, "
            f"refresh_token={masked_refresh!r}, expiry={self.expiry!r})"
        )
