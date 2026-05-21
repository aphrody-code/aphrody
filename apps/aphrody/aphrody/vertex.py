# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Gemini access through Vertex AI using google-genai and OAuth credentials.

This is the proven, recommended path for text generation: the Antigravity
OAuth token (scope ``cloud-platform``) authorizes Vertex AI directly, so
``google-genai`` talks to Gemini with **no API key**. It mirrors the working
``webview2_magika_langextract`` example, generalized and refresh-aware.

Project/location resolution order:

1. explicit ``project`` / ``location`` arguments,
2. ``APHRODY_VERTEX_PROJECT`` / ``APHRODY_VERTEX_LOCATION`` env vars,
3. ``GOOGLE_CLOUD_PROJECT`` env var,
4. the project bound to the Antigravity account (:data:`DEFAULT_VERTEX_PROJECT`).
"""

from __future__ import annotations

import os
from typing import TYPE_CHECKING, Any

from aphrody.auth import credentials as _credentials

if TYPE_CHECKING:
    from google.oauth2.credentials import Credentials

#: Vertex project bound to the Antigravity account (overridable via env).
DEFAULT_VERTEX_PROJECT = "rgfr-8927d"

#: Default Vertex region.
DEFAULT_VERTEX_LOCATION = "us-central1"

#: Default Gemini model id.
DEFAULT_MODEL = "gemini-2.5-flash"


def resolve_project(explicit: str | None = None) -> str:
    """Resolve the Vertex project id to use.

    Args:
        explicit: A caller-provided project id, taking precedence over env.

    Returns:
        The resolved project id.
    """
    return (
        explicit
        or os.environ.get("APHRODY_VERTEX_PROJECT")
        or os.environ.get("GOOGLE_CLOUD_PROJECT")
        or DEFAULT_VERTEX_PROJECT
    )


def resolve_location(explicit: str | None = None) -> str:
    """Resolve the Vertex location/region to use."""
    return (
        explicit
        or os.environ.get("APHRODY_VERTEX_LOCATION")
        or DEFAULT_VERTEX_LOCATION
    )


class GeminiVertex:
    """A thin, keyless Gemini client backed by Vertex AI.

    Example:
        >>> gem = GeminiVertex()
        >>> gem.generate("Say hello in one word.")
        'Hello'
    """

    def __init__(
        self,
        *,
        project: str | None = None,
        location: str | None = None,
        model: str = DEFAULT_MODEL,
        creds: Credentials | None = None,
    ) -> None:
        """Initialize the Vertex-backed Gemini client.

        Args:
            project: Vertex project id (see module resolution order).
            location: Vertex region.
            model: Default model id used by :meth:`generate`.
            creds: Pre-built google-auth credentials; loaded from the local
                credential store when omitted.
        """
        from google import genai

        self.project = resolve_project(project)
        self.location = resolve_location(location)
        self.model = model
        credentials = creds or _credentials.load_google_credentials()
        self._client = genai.Client(
            vertexai=True,
            project=self.project,
            location=self.location,
            credentials=credentials,
        )

    @property
    def client(self) -> Any:
        """The underlying ``google.genai.Client`` for advanced use."""
        return self._client

    def generate(self, prompt: str, *, model: str | None = None) -> str:
        """Generate text for a single prompt and return the response text.

        Args:
            prompt: The user prompt.
            model: Override the default model for this call.

        Returns:
            The model's text response (empty string if none was produced).
        """
        response = self._client.models.generate_content(
            model=model or self.model,
            contents=prompt,
        )
        return response.text or ""

    def generate_content(
        self,
        contents: Any,
        *,
        model: str | None = None,
        config: Any | None = None,
    ) -> Any:
        """Call ``generate_content`` with full control over the request.

        Args:
            contents: Any value accepted by google-genai ``contents`` (string,
                parts, multi-turn list, multimodal content).
            model: Override the default model.
            config: Optional ``GenerateContentConfig``.

        Returns:
            The raw google-genai ``GenerateContentResponse``.
        """
        return self._client.models.generate_content(
            model=model or self.model,
            contents=contents,
            config=config,
        )


def generate(prompt: str, *, model: str = DEFAULT_MODEL) -> str:
    """Convenience one-shot generation using a fresh :class:`GeminiVertex`.

    Args:
        prompt: The user prompt.
        model: The model id to use.

    Returns:
        The model's text response.
    """
    return GeminiVertex(model=model).generate(prompt)
