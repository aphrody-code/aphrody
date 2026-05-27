# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Exception hierarchy for aphrody.

All exceptions derive from :class:`AphrodyError`, so callers can catch the
whole family with a single ``except AphrodyError``.
"""

from __future__ import annotations


class AphrodyError(Exception):
    """Base class for every error raised by aphrody."""


class UnsupportedPlatformError(AphrodyError):
    """Raised when an operation is not available on the current platform."""


class CredentialManagerError(AphrodyError):
    """Raised when a Windows Credential Manager call fails.

    Attributes:
        code: The Win32 ``GetLastError`` code (``1168`` = not found).
    """

    def __init__(self, code: int, message: str | None = None) -> None:
        self.code = code
        super().__init__(
            message or f"Windows Credential Manager call failed (code={code})"
        )


class EmptyCredentialError(AphrodyError):
    """Raised when a credential exists but its blob is empty."""


class TokenParseError(AphrodyError):
    """Raised when a credential blob or token response cannot be parsed."""


class NoRefreshTokenError(AphrodyError):
    """Raised when a refresh is requested but no refresh token is available."""


class OAuthServerError(AphrodyError):
    """Raised on a non-2xx response from a Google OAuth endpoint.

    Attributes:
        status: The HTTP status code.
        body: The (possibly truncated) response body, for diagnostics.
    """

    def __init__(self, status: int, body: str) -> None:
        self.status = status
        self.body = body
        super().__init__(f"OAuth server returned HTTP {status}: {body[:300]}")


class ApiError(AphrodyError):
    """Raised on a non-2xx response from a Gemini / Cloud Code API endpoint.

    Attributes:
        status: The HTTP status code.
        body: The (possibly truncated) response body, for diagnostics.
        url: The request URL that failed.
    """

    def __init__(self, status: int, body: str, url: str = "") -> None:
        self.status = status
        self.body = body
        self.url = url
        super().__init__(f"API returned HTTP {status} for {url}: {body[:300]}")
