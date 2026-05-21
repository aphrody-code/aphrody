# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""The ``aphrody`` command-line interface.

A keyless CLI for Google's AI Ultra stack, built with python-fire (Google's
own CLI generator). Every command authenticates with the local Antigravity
OAuth token — there is no API key anywhere.

Examples:
    $ aphrody whoami
    $ aphrody token
    $ aphrody chat "Explain OAuth refresh in one sentence."
    $ aphrody image "a banana-shaped spaceship, studio render" --out ship.png
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

import httpx

from aphrody import __version__, endpoints, gemini_web, vertex
from aphrody.auth import cookies as cookies_store
from aphrody.auth import credentials, oauth
from aphrody.client import AphrodyClient
from aphrody.errors import AphrodyError, ApiError


class _CookieCommands:
    """``aphrody cookies <action>`` — manage the keyless Google cookie jar.

    Values are never printed: :meth:`status` reports metadata only.
    """

    def status(self) -> None:
        """Show the stored cookie jar metadata (names/domains, never values)."""
        _emit(cookies_store.status())

    def load(self, file: str) -> None:
        """Import cookies from a Cookie-Editor JSON export *file*.

        Args:
            file: Path to a Cookie-Editor (or compatible) JSON export.
        """
        text = Path(file).read_text(encoding="utf-8")
        jar = cookies_store.import_cookie_editor(text)
        _emit({"imported": len(jar), **cookies_store.status(jar)})

    def extract(self, domain: str = "google.com") -> None:
        """Extract cookies straight from local Chrome (best effort).

        Args:
            domain: Cookie domain filter (default ``"google.com"``).
        """
        jar = cookies_store.extract_from_chrome(domain)
        _emit({"extracted": len(jar), **cookies_store.status(jar)})


def _emit(value: Any) -> None:
    """Print a result as indented JSON (dict/list) or plain text."""
    if isinstance(value, (dict, list)):
        print(json.dumps(value, indent=2, ensure_ascii=False))
    else:
        print(value)


class Aphrody:
    """Keyless access to Gemini, Cloud Code and Vertex AI (no API key).

    Credentials are read from the Antigravity OAuth token already present on
    the machine and refreshed transparently.
    """

    def version(self) -> None:
        """Print the aphrody version."""
        _emit(__version__)

    def whoami(self) -> None:
        """Show the signed-in Google account (email + name)."""
        with AphrodyClient.from_credential_manager() as client:
            info = client.userinfo()
        _emit({"email": info.get("email"), "name": info.get("name")})

    def token(self) -> None:
        """Show token status (scopes, expiry) WITHOUT revealing the token."""
        with httpx.Client(timeout=30.0) as http:
            token = credentials.load_token(http=http)
            info = oauth.tokeninfo(token.access_token, http=http)
        _emit(
            {
                "email": info.get("email"),
                "client_id": info.get("aud"),
                "expires_in_seconds": info.get("expires_in"),
                "expiry": token.expiry,
                "scopes": info.get("scope", "").split(),
                "has_refresh_token": token.refresh_token is not None,
            }
        )

    def models(self) -> None:
        """List Cloud Code models for the account (with a tier fallback).

        ``loadCodeAssist`` resolves the entitlement tier and the user's
        Code Assist project; ``fetchAvailableModels`` is then queried for that
        project. Consumer tiers gate ``fetchAvailableModels`` (HTTP 403); in
        that case the working tier summary is returned instead of failing.
        """
        endpoint = endpoints.CloudCodeEndpoint.PROD
        with AphrodyClient.from_credential_manager(
            cloud_code_endpoint=endpoint
        ) as client:
            info = client.load_code_assist(
                {"metadata": {"pluginType": "GEMINI"}}
            )
            project = info.get("cloudaicompanionProject")
            try:
                body = {"project": project} if project else {}
                _emit(client.fetch_available_models(body))
            except ApiError as exc:
                tier = info.get("currentTier", {})
                paid = info.get("paidTier", {})
                _emit(
                    {
                        "tier": tier.get("name"),
                        "tier_id": tier.get("id"),
                        "paid_tier": paid.get("name"),
                        "cloudaicompanion_project": project,
                        "note": (
                            f"fetchAvailableModels denied (HTTP {exc.status}); "
                            "this tier gates the model list. Showing the tier "
                            "from loadCodeAssist instead."
                        ),
                    }
                )

    def chat(self, prompt: str, model: str = vertex.DEFAULT_MODEL) -> None:
        """Generate a Gemini response for ``prompt`` via Vertex AI.

        Args:
            prompt: The text prompt.
            model: The Gemini model id.
        """
        _emit(vertex.GeminiVertex(model=model).generate(prompt))

    def web(self, prompt: str) -> None:
        """Ask the Gemini *web app* (gemini.google.com) via session cookies.

        The keyless cookie path: the same Boq backend the browser talks to,
        with no API key and no OAuth token — only the stored Google cookies.

        Args:
            prompt: The message to send.
        """
        with gemini_web.GeminiWebClient() as client:
            _emit(client.generate(prompt, keep_context=False))

    def cookies(self) -> _CookieCommands:
        """Manage the Google cookie jar: ``status`` / ``load`` / ``extract``."""
        return _CookieCommands()

    def image(
        self,
        prompt: str,
        out: str = "nanobanana.png",
        model: str | None = None,
    ) -> None:
        """Generate an image from ``prompt`` with Gemini Image (nano-banana).

        Args:
            prompt: The image description.
            out: Output PNG path.
            model: Optional image model id override.
        """
        from aphrody import images  # lazy: provided by the images module

        paths = images.generate_image(prompt, out=out, model=model)
        _emit({"saved": [str(p) for p in _as_list(paths)]})

    def voice(
        self,
        host: str = "127.0.0.1",
        port: int = 8789,
        whisper_model: str = "base",
        kokoro_model: str | None = None,
        voices_path: str | None = None,
        voice_name: str = "ff_siwis",
        models_dir: str | None = None,
        ui: bool = True,
        ui_port: int = 8790,
    ) -> None:
        """Start the local voice-to-voice WebSocket server (offline Whisper + Kokoro) and serve the UI.

        Args:
            host: Host address to bind to (default "127.0.0.1").
            port: Port to run the WebSocket server on (default 8789).
            whisper_model: Size or path of the Whisper model (default "base").
            kokoro_model: Path to Kokoro ONNX file.
            voices_path: Path to voices.json configuration.
            voice_name: Default Kokoro voice name (default "ff_siwis").
            models_dir: Directory to store downloaded models (default ~/.aphrody/models).
            ui: Launch the web UI automatically (default True).
            ui_port: Port to run the web UI server on if --ui is True (default 8790).
        """
        import asyncio

        from aphrody.voice_server import start_voice_server

        try:
            asyncio.run(
                start_voice_server(
                    host=host,
                    port=port,
                    whisper_model=whisper_model,
                    kokoro_model=kokoro_model,
                    voices_path=voices_path,
                    voice_name=voice_name,
                    models_dir=models_dir,
                    ui=ui,
                    ui_port=ui_port,
                )
            )
        except KeyboardInterrupt:
            print("\nExiting voice server...")


def _as_list(value: Any) -> list:
    """Wrap a scalar in a list; pass lists through unchanged."""
    return value if isinstance(value, list) else [value]


def main() -> None:
    """Entry point for the ``aphrody`` console script."""
    import fire

    # LLM-first: always emit UTF-8, regardless of the host console code page
    # (Windows consoles default to cp1252 and would mangle accented replies).
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8")  # type: ignore[attr-defined]
        except (AttributeError, ValueError):
            pass

    try:
        fire.Fire(Aphrody, name="aphrody")
    except AphrodyError as exc:
        print(f"error: {exc}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
