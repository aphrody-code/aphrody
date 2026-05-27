# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Local voice-to-voice loop for the ``aphrody voice`` command.

Bridges a browser audio stream (Web Audio API) with local Whisper STT, a keyless
Gemini brain (aphrody's :class:`~aphrody.vertex.GeminiVertex` over Vertex AI —
**no API key, no external harness**) and local Kokoro TTS, then serves a small
web UI.

Pipeline::

    browser mic ─ws→ Whisper STT ─→ VoiceBrain (Vertex, keyless) ─→ Kokoro TTS ─ws→ browser

The brain keeps a per-connection message history and streams replies
sentence-by-sentence for low-latency speech, with barge-in (a new utterance
cancels the in-flight reply).
"""

from __future__ import annotations

import asyncio
import http.server
import json
import os
import sys
import urllib.request
import webbrowser
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import numpy as np
import websockets
from google.antigravity.voice import (
    LocalKokoroTextToSpeech,
    LocalWhisperSpeechToText,
)

from aphrody.vertex import DEFAULT_MODEL, GeminiVertex

# Default model directory (Kokoro weights live here, not under var/secrets —
# they are public model files, not credentials).
DEFAULT_MODELS_DIR = os.path.join(os.path.expanduser("~"), ".aphrody", "models")

KOKORO_MODEL_URL = (
    "https://huggingface.co/thewh1teagle/Kokoro/resolve/main/kokoro-v0_19.onnx"
)
KOKORO_VOICES_URL = (
    "https://github.com/thewh1teagle/kokoro-onnx/releases/download/"
    "model-files-v1.0/voices-v1.0.bin"
)

# Voice persona system instructions, keyed by Kokoro voice-name prefix.
SYSTEM_INSTRUCTIONS_EN = (
    "You are a helpful and friendly voice assistant. Respond very concisely, "
    "directly, and naturally in English. Avoid bullet points, markdown "
    "formatting, or long paragraphs. Keep sentences short and conversational, "
    "optimized for text-to-speech."
)
SYSTEM_INSTRUCTIONS_FR = (
    "Tu es un assistant vocal intelligent, chaleureux et utile. Réponds de "
    "manière extrêmement concise, directe et naturelle en français. Évite "
    "absolument les listes à puces, le markdown et les longs paragraphes. "
    "Rédige des phrases très courtes adaptées à la synthèse vocale."
)
SYSTEM_INSTRUCTIONS_JA = (
    "あなたは親切な音声アシスタントです。日本語で非常に簡潔かつ自然に答えてください。"
    "長い文章やリスト、マークダウン装飾は避け、音声合成に適した短い文で返答してください。"
)


def system_instruction_for(voice_profile: str) -> str:
    """Return the persona system instruction for a Kokoro *voice_profile*.

    Args:
        voice_profile: A Kokoro voice name (e.g. ``"ff_siwis"``); its language
            prefix (``ff``/``fm`` ▸ French, ``jf``/``jm`` ▸ Japanese, else
            English) selects the persona.

    Returns:
        The system instruction string for that language.
    """
    if voice_profile.startswith(("ff", "fm")):
        return SYSTEM_INSTRUCTIONS_FR
    if voice_profile.startswith(("jf", "jm")):
        return SYSTEM_INSTRUCTIONS_JA
    return SYSTEM_INSTRUCTIONS_EN


def whisper_language_for(voice_profile: str) -> str:
    """Return the Whisper language code matching a Kokoro *voice_profile*."""
    if voice_profile.startswith(("ff", "fm")):
        return "fr"
    if voice_profile.startswith(("jf", "jm")):
        return "ja"
    return "en"


def ensure_models_exist(models_dir: str) -> tuple[str, str]:
    """Ensure the Kokoro ONNX model and voices file exist, downloading if not.

    Args:
        models_dir: Directory to hold the model files.

    Returns:
        ``(model_path, voices_path)``.
    """
    os.makedirs(models_dir, exist_ok=True)
    model_path = os.path.join(models_dir, "kokoro-v0_19.onnx")
    voices_path = os.path.join(models_dir, "voices.bin")

    if not os.path.exists(model_path):
        print(f"Kokoro model not found at {model_path}. Auto-downloading...")
        urllib.request.urlretrieve(KOKORO_MODEL_URL, model_path)
        print("Kokoro model download complete.")

    if not os.path.exists(voices_path):
        print(f"Kokoro voices not found at {voices_path}. Auto-downloading...")
        urllib.request.urlretrieve(KOKORO_VOICES_URL, voices_path)
        print("Kokoro voices download complete.")

    return model_path, voices_path


class VoiceBrain:
    """Keyless conversational brain for the voice loop, backed by Vertex AI.

    Holds the per-session message history and a system persona, and streams
    replies through aphrody's keyless :class:`~aphrody.vertex.GeminiVertex` —
    no API key and no Antigravity harness binary required.
    """

    def __init__(
        self,
        system_instruction: str,
        *,
        model: str = DEFAULT_MODEL,
        temperature: float = 0.7,
    ) -> None:
        """Initialize the brain.

        Args:
            system_instruction: The persona/system prompt.
            model: Gemini model id.
            temperature: Sampling temperature.
        """
        self._system_instruction = system_instruction
        self._temperature = temperature
        self._history: list[dict[str, Any]] = []
        self._gemini = GeminiVertex(model=model)

    @property
    def history(self) -> list[dict[str, Any]]:
        """The running conversation history (user/model turns)."""
        return self._history

    def stream_reply(self, user_text: str) -> Iterator[str]:
        """Append *user_text*, stream the model's reply, and record it.

        Args:
            user_text: The transcribed user utterance.

        Yields:
            Reply text deltas as they stream from the model.
        """
        self._history.append({"role": "user", "parts": [{"text": user_text}]})
        accumulated = ""
        for delta in self._gemini.stream(
            list(self._history),
            system_instruction=self._system_instruction,
            temperature=self._temperature,
        ):
            accumulated += delta
            yield delta
        if accumulated:
            self._history.append(
                {"role": "model", "parts": [{"text": accumulated}]}
            )


# Punctuation that flushes the sentence buffer to TTS for low latency.
_TTS_FLUSH_CHARS = (".", "?", "!", "\n", ",", ";")


class VoiceServer:
    """Serves the voice loop over a WebSocket, proxying STT ▸ brain ▸ TTS."""

    def __init__(
        self,
        stt: LocalWhisperSpeechToText,
        tts: LocalKokoroTextToSpeech,
        voice_name: str = "ff_siwis",
    ) -> None:
        """Initialize the server.

        Args:
            stt: A loaded local Whisper speech-to-text engine.
            tts: A loaded local Kokoro text-to-speech engine.
            voice_name: Default Kokoro voice when the client sends none.
        """
        self.stt = stt
        self.tts = tts
        self.voice_name = voice_name

    async def handle_connection(self, websocket: Any) -> None:
        """Process WebSocket messages for one client connection."""
        print(f"New client connected: {websocket.remote_address}")

        audio_buffer: list[np.ndarray] = []
        is_user_speaking = False
        voice_profile = self.voice_name
        active_turn_task: asyncio.Task | None = None
        brain: VoiceBrain | None = None

        try:
            async for message in websocket:
                if isinstance(message, bytes):
                    if is_user_speaking:
                        chunk = np.frombuffer(message, dtype=np.float32)
                        audio_buffer.append(chunk)
                    continue

                try:
                    data = json.loads(message)
                except json.JSONDecodeError:
                    print("Warning: invalid JSON string.", file=sys.stderr)
                    continue

                msg_type = data.get("type")

                if msg_type == "start":
                    voice_profile = data.get("voice", self.voice_name)
                    print(f"Session start. Voice: {voice_profile}")
                    if active_turn_task and not active_turn_task.done():
                        active_turn_task.cancel()
                    brain = VoiceBrain(system_instruction_for(voice_profile))
                    await websocket.send(
                        json.dumps({"type": "status", "status": "idle"})
                    )

                elif msg_type == "speech_start":
                    print("\n[VAD] User started speaking...")
                    is_user_speaking = True
                    audio_buffer = []
                    if active_turn_task and not active_turn_task.done():
                        print("[Barge-in] Interrupting active generation.")
                        active_turn_task.cancel()
                    await websocket.send(
                        json.dumps({"type": "status", "status": "listening"})
                    )
                    await websocket.send(json.dumps({"type": "interrupt"}))

                elif msg_type == "speech_end":
                    print("[VAD] User finished speaking. Processing...")
                    is_user_speaking = False
                    await websocket.send(
                        json.dumps({"type": "status", "status": "thinking"})
                    )
                    if not audio_buffer:
                        await websocket.send(
                            json.dumps({"type": "status", "status": "idle"})
                        )
                        continue

                    audio_data = np.concatenate(audio_buffer)
                    audio_buffer = []
                    stt_lang = whisper_language_for(voice_profile)
                    loop = asyncio.get_running_loop()
                    transcription = await loop.run_in_executor(
                        None,
                        lambda: self.stt.transcribe(
                            audio_data, language=stt_lang, beam_size=1
                        ),
                    )
                    if not transcription or not transcription.strip():
                        print("[STT] No speech detected.")
                        await websocket.send(
                            json.dumps({"type": "status", "status": "idle"})
                        )
                        continue

                    print(f"[STT] User: {transcription}")
                    await websocket.send(
                        json.dumps(
                            {
                                "type": "transcript",
                                "role": "user",
                                "text": transcription,
                            }
                        )
                    )
                    if brain is None:
                        brain = VoiceBrain(
                            system_instruction_for(voice_profile)
                        )
                    active_turn_task = asyncio.create_task(
                        self.process_turn(
                            websocket, brain, transcription, voice_profile
                        )
                    )

                elif msg_type == "interrupt":
                    print("[Command] Interrupt from client.")
                    if active_turn_task and not active_turn_task.done():
                        active_turn_task.cancel()
                    await websocket.send(
                        json.dumps({"type": "status", "status": "idle"})
                    )

        except websockets.exceptions.ConnectionClosed:
            print(f"Connection closed by: {websocket.remote_address}")
        finally:
            if active_turn_task and not active_turn_task.done():
                active_turn_task.cancel()

    async def process_turn(
        self,
        websocket: Any,
        brain: VoiceBrain,
        transcription: str,
        voice_profile: str,
    ) -> None:
        """Stream the brain's reply to the client, synthesizing per sentence."""
        try:
            print("[Brain] Generating response...")
            loop = asyncio.get_running_loop()
            stream = await loop.run_in_executor(
                None, brain.stream_reply, transcription
            )

            def next_delta(iterator: Iterator[str]) -> str | None:
                try:
                    return next(iterator)
                except StopIteration:
                    return None

            sentence_buffer: list[str] = []
            while True:
                delta = await loop.run_in_executor(None, next_delta, stream)
                if delta is None:
                    break
                print(delta, end="", flush=True)
                sentence_buffer.append(delta)
                await websocket.send(
                    json.dumps(
                        {
                            "type": "transcript",
                            "role": "agent",
                            "text": delta,
                            "is_delta": True,
                        }
                    )
                )
                if any(p in delta for p in _TTS_FLUSH_CHARS):
                    stripped = "".join(sentence_buffer).strip()
                    if stripped:
                        await self.synthesize_and_stream(
                            websocket, stripped, voice_profile
                        )
                    sentence_buffer = []

            remaining = "".join(sentence_buffer).strip()
            if remaining:
                await self.synthesize_and_stream(
                    websocket, remaining, voice_profile
                )

            print("\n[Brain] Response finished.")
            await websocket.send(
                json.dumps({"type": "status", "status": "idle"})
            )
        except asyncio.CancelledError:
            print("\n[Brain] Turn cancelled (barge-in).")
        except Exception as exc:
            print(f"\nError in turn: {exc}", file=sys.stderr)
            await websocket.send(
                json.dumps({"type": "error", "message": str(exc)})
            )
            await websocket.send(
                json.dumps({"type": "status", "status": "idle"})
            )

    async def synthesize_and_stream(
        self, websocket: Any, text: str, voice_profile: str
    ) -> None:
        """Synthesize *text* with Kokoro and stream raw PCM to the client."""
        try:
            await websocket.send(
                json.dumps({"type": "status", "status": "speaking"})
            )
            loop = asyncio.get_running_loop()
            samples, sample_rate = await loop.run_in_executor(
                None, self.tts.synthesize, text, voice_profile
            )
            await websocket.send(
                json.dumps({"type": "audio_start", "sample_rate": sample_rate})
            )
            await websocket.send(samples.tobytes())
            await websocket.send(json.dumps({"type": "audio_end"}))
        except asyncio.CancelledError:
            raise
        except Exception as exc:
            print(f"TTS error for {text!r}: {exc}", file=sys.stderr)


def run_ui_http_server(
    host: str,
    ui_port: int,
    websocket_host: str,
    websocket_port: int,
    launch_browser: bool,
) -> None:
    """Serve the static voice UI and optionally open it in a browser."""
    ui_dir = Path(__file__).parent / "ui"

    class UIHandler(http.server.SimpleHTTPRequestHandler):
        """Serves the UI, injecting the live WebSocket address into index.html."""

        def __init__(self, *args: Any, **kwargs: Any) -> None:
            super().__init__(*args, directory=str(ui_dir), **kwargs)

        def do_GET(self) -> None:
            """Serve index.html with the WebSocket URL templated in."""
            if self.path in {"/", "/index.html"}:
                try:
                    content = (ui_dir / "index.html").read_text(
                        encoding="utf-8"
                    )
                    content = content.replace(
                        "ws://127.0.0.1:8789",
                        f"ws://{websocket_host}:{websocket_port}",
                    )
                    encoded = content.encode("utf-8")
                    self.send_response(200)
                    self.send_header("Content-Type", "text/html; charset=utf-8")
                    self.send_header("Content-Length", str(len(encoded)))
                    self.end_headers()
                    self.wfile.write(encoded)
                    return
                except OSError as exc:
                    print(f"Error serving index.html: {exc}", file=sys.stderr)
            super().do_GET()

        def log_message(self, format: str, *args: Any) -> None:
            """Suppress per-request HTTP logging noise."""

    try:
        server = http.server.HTTPServer((host, ui_port), UIHandler)
        print(f"Serving voice UI at http://{host}:{ui_port}")
        if launch_browser:
            webbrowser.open(f"http://{host}:{ui_port}")
        server.serve_forever()
    except OSError as exc:
        print(f"Warning: failed to start web UI server: {exc}", file=sys.stderr)


async def start_voice_server(
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
    """Start the WebSocket voice server (and optionally the web UI).

    Args:
        host: Bind address.
        port: WebSocket port.
        whisper_model: Whisper model size or path.
        kokoro_model: Kokoro ONNX path (auto-resolved/downloaded if omitted).
        voices_path: Kokoro voices path (auto-resolved/downloaded if omitted).
        voice_name: Default Kokoro voice.
        models_dir: Directory for auto-downloaded models.
        ui: Whether to serve and open the web UI.
        ui_port: Web UI port.
    """
    target_models_dir = models_dir or DEFAULT_MODELS_DIR
    if not kokoro_model or not voices_path:
        print(f"Resolving Kokoro model in {target_models_dir}...")
        resolved_model, resolved_voices = ensure_models_exist(target_models_dir)
        kokoro_model_path = kokoro_model or resolved_model
        kokoro_voices_path = voices_path or resolved_voices
    else:
        kokoro_model_path = kokoro_model
        kokoro_voices_path = voices_path

    print(f"Using Whisper model: {whisper_model}")
    print(f"Using Kokoro model: {kokoro_model_path}")
    print(f"Using voices file: {kokoro_voices_path}")

    print("Loading local Whisper model...")
    stt = LocalWhisperSpeechToText(
        model_size_or_path=whisper_model,
        device="cpu",
        compute_type="int8",
    )
    print("Loading local Kokoro TTS...")
    tts = LocalKokoroTextToSpeech(
        model_path=kokoro_model_path,
        voices_path=kokoro_voices_path,
    )

    server = VoiceServer(stt, tts, voice_name)

    if ui:
        import threading

        thread = threading.Thread(
            target=run_ui_http_server,
            args=(host, ui_port, host, port, True),
            daemon=True,
        )
        thread.start()

    print(f"Starting WebSocket server on ws://{host}:{port}")
    async with websockets.serve(server.handle_connection, host, port):
        await asyncio.Future()
