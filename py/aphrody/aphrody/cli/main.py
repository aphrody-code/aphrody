# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Main command-line interface entry point for aphrody."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

from aphrody import __version__, endpoints, vertex
from aphrody.auth import credentials, oauth
from aphrody.cli.utils import _autocomplete_dry_run, _emit
from aphrody.client import AphrodyClient
from aphrody.errors import AphrodyError, ApiError


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
        import httpx

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

    def chat(
        self,
        prompt: str,
        model: str = vertex.DEFAULT_MODEL,
        think: bool = False,
        budget: int = 1024,
    ) -> None:
        """Generate a Gemini response for ``prompt`` via Vertex AI.

        Args:
            prompt: The text prompt.
            model: The Gemini model id.
            think: Use thinking config (deep reasoning).
            budget: Thinking token budget.
        """
        gv = vertex.GeminiVertex(model=model)
        if think:
            thought, response = gv.generate_think(prompt, budget=budget)
            _emit({"thought": thought, "response": response})
        else:
            _emit(gv.generate(prompt))

    def autocomplete(
        self,
        prefix: str | None = None,
        file: str | None = None,
        suffix: str = "",
        line: int | None = None,
        col: int | None = None,
        lang: str | None = None,
        marker: str | None = None,
        n: int = 1,
        model: str = vertex.DEFAULT_MODEL,
        stream: bool = False,
        auto: bool = False,
        write: bool = False,
        dry_run: bool = False,
    ) -> None:
        """Keyless automatic code completion ("auto-recomplete").

        Predicts the code at a cursor with Gemini (keyless, via Vertex AI) and
        prints structured completions as JSON. Provide either ``--prefix`` (a
        code fragment to continue) or ``--file`` (with ``--line``/``--col`` or a
        cursor marker). With ``--suffix`` (or surrounding file context) it does
        fill-in-the-middle. ``--auto`` walks files and fills the cursor marker
        (``<|cursor|>``) in each; pair with ``--write`` to apply in place.

        Args:
            prefix: Code before the cursor (continuation mode).
            file: Source file to complete inside (mutually exclusive with
                ``--prefix``). In ``--auto`` mode, the file(s) to fill.
            suffix: Code after the cursor (enables fill-in-the-middle).
            line: 1-based cursor line within ``--file``.
            col: 1-based cursor column within the line.
            lang: Language hint (e.g. ``python``); inferred from ``--file``.
            marker: Cursor marker substring (default ``<|cursor|>`` for files).
            n: Number of candidates to return.
            model: Gemini model id.
            stream: Stream the first candidate token-by-token (low TTFT).
            auto: Auto-fill the cursor marker in the file(s) (batch loop).
            write: In ``--auto`` mode, write completions back to disk.
            dry_run: Do not call the model; echo the resolved request (offline
                smoke test, burns no live quota).
        """
        from aphrody import autocomplete as ac

        if dry_run:
            _emit(
                _autocomplete_dry_run(
                    prefix, file, suffix, line, col, lang, marker, n, model
                )
            )
            return

        if auto:
            if not file:
                raise AphrodyError("--auto requires --file")
            results = ac.recomplete_paths(
                [file],
                marker=marker or ac.DEFAULT_CURSOR_MARKER,
                model=model,
                write=write,
            )
            _emit({"auto": True, "write": write, "results": results})
            return

        if stream:
            completer = ac.CodeCompleter(model=model)
            if file is not None:
                text = Path(file).read_text(encoding="utf-8")
                eff_marker = (
                    marker if marker is not None else ac.DEFAULT_CURSOR_MARKER
                )
                pre, suf = ac.split_at_cursor(
                    text, line=line, col=col, marker=eff_marker
                )
                req = ac.CompletionRequest(
                    prefix=pre,
                    suffix=suf,
                    language=lang or ac.language_for_path(file),
                    path=file,
                )
            else:
                req = ac.CompletionRequest(
                    prefix=prefix or "", suffix=suffix, language=lang
                )
            for delta in completer.stream(req):
                sys.stdout.write(delta)
                sys.stdout.flush()
            sys.stdout.write("\n")
            return

        candidates = ac.complete(
            prefix=prefix,
            suffix=suffix,
            file=file,
            line=line,
            col=col,
            language=lang,
            marker=marker,
            n=n,
            model=model,
        )
        _emit(
            {
                "model": model,
                "count": len(candidates),
                "candidates": [c.to_dict() for c in candidates],
            }
        )

    # ``recomplete`` is an alias for ``autocomplete`` (the auto loop).
    recomplete = autocomplete

    def image(self) -> Any:
        """Nano Banana Pro image suite.

        Subcommands: ``gen`` / ``edit`` / ``compose`` / ``optimize`` /
        ``batch`` / ``prompts`` / ``template`` / ``enhance`` / ``analyze`` /
        ``anim`` / ``to3d`` / ``icon`` / ``models``.
        """
        from aphrody.cli.image import ImageCommands

        return ImageCommands()

    def blender(self) -> Any:
        """Drive a running Blender via the blender-mcp addon socket.

        Subcommands: ``scene`` / ``exec`` / ``import_glb`` / ``render`` /
        ``turntable``.
        """
        from aphrody.cli.blender import BlenderCommands

        return BlenderCommands()

    def drive(self) -> Any:
        """Authenticated Google Drive workspace.

        Subcommands: ``folder`` / ``upload`` / ``list``.
        """
        from aphrody.cli.drive import DriveCommands

        return DriveCommands()

    def evaluate(self) -> Any:
        """Local, offline, and keyless model evaluation suite.

        Subcommands: ``text`` / ``file``.
        """
        from aphrody.cli.evaluation import EvaluationCommands

        return EvaluationCommands()

    def setup(
        self,
        project: str | None = None,
        service_account: str | None = None,
        location: str | None = None,
        interactive: bool = False,
        i: bool = False,
    ) -> None:
        """Run automated local secrets, credentials, and GCP environment configuration.

        Downloads service account keys, activates the service account in gcloud,
        assigns maximum roles (owner), enables required GCP APIs, and creates the
        local secrets directory and .env configuration file.

        Args:
            project: Google Cloud Project ID override.
            service_account: Service account name/email override.
            location: Google Cloud Location/Region override.
            interactive: Prompt for Google Cloud project and resource naming interactively.
            i: Short alias for interactive.
        """
        from aphrody.cli.setup import setup_secrets

        success = setup_secrets(
            project_id=project,
            service_account=service_account,
            location=location,
            interactive=interactive or i,
        )
        if not success:
            sys.exit(1)

    def serve(
        self,
        root: str,
        host: str = "0.0.0.0",
        port: int = 8080,
        spa: bool = True,
        cache: bool = False,
        proxy: str | None = None,
        proxy_prefix: str = "/api",
    ) -> None:
        """Serve a built static / SPA site (e.g. a React ``dist``) — VPS-ready.

        Pure-stdlib threaded server with single-page-app fallback, optional
        in-RAM caching, and an optional reverse proxy (front a Rust/other
        backend from one process). Designed to run under systemd (see ``deploy/``).

        Args:
            root: Directory of the built site to serve.
            host: Bind address (``0.0.0.0`` for a VPS).
            port: Bind port.
            spa: Serve ``index.html`` for unknown routes (client-side routing).
            cache: Preload every file into RAM for zero-disk-hit serving.
            proxy: Backend base URL (e.g. ``http://127.0.0.1:3000``) to forward
                ``proxy_prefix`` requests to (e.g. a Rust API).
            proxy_prefix: URL prefix routed to the backend (default ``/api``).
        """
        from aphrody.serve import serve as _serve

        _serve(
            root,
            host,
            port,
            spa=spa,
            cache=cache,
            proxy=proxy,
            proxy_prefix=proxy_prefix,
        )

    def forensic(
        self,
        target: str,
        deep: bool = False,
        ask: str | None = None,
        out_dir: str | None = None,
        dry_run: bool = False,
        max_files: int = 200000,
    ) -> None:
        """Full forensic + classification + RAG + auto-ML pass over a target.

        Orchestrates the pipeline (inventory -> Magika classify -> LIEF PE
        inspect -> markitdown docs -> source extraction -> fastembed RAG ->
        keyless Gemini synthesis/auto-ML) and writes ``report.json`` +
        ``report.md`` under ``var/data/forensic-<target>/``.

        ``target`` may be a path or a known Antigravity name: ``install``
        (the installed program dir), ``appdata``, ``dotdir``, ``agy``,
        ``gemini``. Full mode: real values (tokens included) are read and
        classified — the owner's own machine, data and account. No analysed
        binary is executed (Magika/LIEF are static).

        Args:
            target: A path, or ``install`` / ``appdata`` / ``dotdir`` / ``agy``
                / ``gemini``.
            deep: Heavy passes — source extraction + RAG index + LLM auto-ML
                component tagging.
            ask: A question answered via RAG retrieval -> Gemini.
            out_dir: Override the output directory.
            dry_run: Skip every LLM call (offline smoke; burns no quota).
            max_files: Inventory file cap (safety bound).
        """
        from aphrody.forensic.pipeline import run_forensic

        report = run_forensic(
            target,
            deep=deep,
            dry_run=dry_run,
            ask=ask,
            out_dir=out_dir,
            max_files=max_files,
        )
        # Keep stdout compact: emit the summary, not the full per-file dump.
        summary = {
            "target": report.get("target"),
            "resolved_path": report.get("resolved_path"),
            "exists": report.get("exists"),
            "deep": report.get("deep"),
            "dry_run": report.get("dry_run"),
            "out_dir": report.get("out_dir"),
            "report_json": (
                str(Path(report["out_dir"]) / "report.json")
                if report.get("out_dir")
                else None
            ),
            "report_md": report.get("report_md"),
            "inventory": report.get("inventory", {}).get("summary"),
            "classification": report.get("classification"),
            "pe_count": len(report.get("pe_reports", [])),
            "documents": len(report.get("documents", [])),
        }
        if "extraction" in report:
            summary["extraction"] = {
                "total_files": report["extraction"].get("total_files"),
                "asar": len(report["extraction"].get("asar_archives", [])),
            }
        if "rag" in report:
            summary["rag"] = report["rag"]
        if "error" in report:
            summary["error"] = report["error"]
        _emit(summary)

    def targets(self) -> None:
        """List the known Antigravity forensic targets and whether they exist."""
        from aphrody.forensic import targets as targets_mod

        known = targets_mod.known_targets()
        _emit(
            {
                name: {"path": path, "exists": Path(path).exists()}
                for name, path in known.items()
            }
        )

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

    def think(
        self,
        prompt: str,
        model: str = "gemini-2.0-flash",
        budget: int = 2048,
    ) -> None:
        """Generate content with a deep thinking reasoning budget.

        Args:
            prompt: The user prompt.
            model: Model to use. Defaults to gemini-2.0-flash.
            budget: Thinking token budget.
        """
        from aphrody import vertex

        gv = vertex.GeminiVertex(model=model)
        thought, response = gv.generate_think(prompt, budget=budget)
        _emit({"thought": thought, "response": response})

    def video(self) -> Any:
        """Video generation suite.

        Subcommand: ``gen``.
        """
        from aphrody.cli.video import VideoCommands

        return VideoCommands()

    def music(self) -> Any:
        """Music/Audio generation suite.

        Subcommand: ``gen``.
        """
        from aphrody.cli.music import MusicCommands

        return MusicCommands()

    def research(self) -> Any:
        """Iterative agentic deep research suite."""
        from aphrody.cli.research import ResearchCommands

        return ResearchCommands()

    def rag(self) -> Any:
        """Ultimate RAG suite.

        Subcommands: ``chunk`` / ``raptor`` / ``graph`` / ``process``.
        """
        from aphrody.cli.rag import RAGCommands

        return RAGCommands()


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
