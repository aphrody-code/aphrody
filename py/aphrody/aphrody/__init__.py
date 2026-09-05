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
from aphrody.background_review import BackgroundReview, spawn_background_review
from aphrody.client import AphrodyClient
from aphrody.command_guard import CommandGuard, SecurityError
from aphrody.evaluation import LocalEvaluator
from aphrody.google_drive import AuthenticatedDriveClient
from aphrody.session_db import SessionDB
from aphrody.soul_creator import SoulCreator
from aphrody.timeout_monitor import (
    RunawayDetector,
    RunawayLoopError,
    TimeoutMonitor,
)
from aphrody.vertex import GeminiVertex

__all__ = [
    "AphrodyClient",
    "AuthenticatedDriveClient",
    "BackgroundReview",
    "CodeCompleter",
    "CommandGuard",
    "Completion",
    "CompletionRequest",
    "GeminiVertex",
    "LocalEvaluator",
    "OAuthToken",
    "RunawayDetector",
    "RunawayLoopError",
    "SecurityError",
    "SessionDB",
    "SoulCreator",
    "TimeoutMonitor",
    "__version__",
    "spawn_background_review",
]
