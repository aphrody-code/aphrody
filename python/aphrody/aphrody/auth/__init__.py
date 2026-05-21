# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Keyless authentication for Google's AI Ultra stack.

This package reads the OAuth credentials already present on the machine — the
Antigravity desktop client's ``gemini:antigravity`` token in the Windows
Credential Manager (or a local cache on other platforms) — and refreshes them
transparently through the proven OAuth path. No API key is ever required.
"""

from aphrody.auth.credentials import (
    access_token,
    load_google_credentials,
    load_token,
)
from aphrody.auth.tokens import OAuthToken

__all__ = [
    "OAuthToken",
    "access_token",
    "load_google_credentials",
    "load_token",
]
