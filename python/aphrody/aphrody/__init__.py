# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""aphrody — a keyless Python client for Google's AI Ultra stack.

aphrody talks to Gemini, Cloud Code and Vertex AI using **only** the OAuth
credentials already present on the machine (the Antigravity desktop client's
``gemini:antigravity`` token, gcloud Application Default Credentials, and
browser cookies). It never asks for, stores, or transmits an API key.

The public surface mirrors the native Rust ``antigravity-sdk`` crate:

    >>> from aphrody import AphrodyClient
    >>> client = AphrodyClient.from_credential_manager()
    >>> client.userinfo()["email"]
    'user@example.com'
"""

from aphrody._version import __version__
from aphrody.auth.tokens import OAuthToken
from aphrody.autocomplete import CodeCompleter, Completion, CompletionRequest
from aphrody.client import AphrodyClient
from aphrody.gemini_web import GeminiWebClient
from aphrody.vertex import GeminiVertex

__all__ = [
    "AphrodyClient",
    "CodeCompleter",
    "Completion",
    "CompletionRequest",
    "GeminiVertex",
    "GeminiWebClient",
    "OAuthToken",
    "__version__",
]
