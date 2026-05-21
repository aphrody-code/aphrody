# Copyright 2026 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""WebSocket server for local voice-to-voice web application.

Bridges the browser audio stream (Web Audio API) with the local Whisper STT,
Kokoro TTS, and Antigravity Agent connection.
"""

import argparse
import asyncio
import json
import os
import sys
import urllib.request
from typing import Any

import numpy as np
import websockets

from google.antigravity import Agent, LocalAgentConfig
from google.antigravity.types import Content
from google.antigravity.voice import (
    LocalKokoroTextToSpeech,
    LocalWhisperSpeechToText,
)

# Constants for default models
KOKORO_MODEL_URL = "https://huggingface.co/hexgrad/Kokoro-82M/resolve/main/kokoro-v0_19.onnx"
KOKORO_VOICES_URL = "https://huggingface.co/hexgrad/Kokoro-82M/resolve/main/voices.json"
DEFAULT_MODELS_DIR = os.path.join(os.path.dirname(__file__), "models")


def ensure_models_exist(models_dir: str) -> tuple[str, str]:
    """Ensure Kokoro ONNX model and voices.json exist locally. Downloads if missing."""
    os.makedirs(models_dir, exist_ok=True)
    model_path = os.path.join(models_dir, "kokoro-v0_19.onnx")
    voices_path = os.path.join(models_dir, "voices.json")

    if not os.path.exists(model_path):
        print(f"Kokoro model not found at {model_path}. Auto-downloading...")
        urllib.request.urlretrieve(KOKORO_MODEL_URL, model_path)
        print("Kokoro model download complete.")

    if not os.path.exists(voices_path):
        print(f"Kokoro voices.json not found at {voices_path}. Auto-downloading...")
        urllib.request.urlretrieve(KOKORO_VOICES_URL, voices_path)
        print("Kokoro voices.json download complete.")

    return model_path, voices_path


class VoiceServer:
    """Handles WebSocket connections and proxies audio streams between client and SDK."""

    def __init__(
        self,
        stt: LocalWhisperSpeechToText,
        tts: LocalKokoroTextToSpeech,
        voice_name: str = "af_bella",
    ) -> None:
        self.stt = stt
        self.tts = tts
        self.voice_name = voice_name
        self.agent_config = LocalAgentConfig()

    async def handle_connection(self, websocket: Any) -> None:
        """Process incoming WebSocket messages from a client connection."""
        print(f"New client connected: {websocket.remote_address}")

        # Active conversation state variables
        audio_buffer: list[np.ndarray] = []
        is_user_speaking = False
        voice_profile = self.voice_name
        active_turn_task = None

        # Start agent session
        async with Agent(self.agent_config) as voice_agent:
            conversation = voice_agent.conversation

            try:
                async for message in websocket:
                    # Handle incoming binary audio frames
                    if isinstance(message, bytes):
                        if is_user_speaking:
                            # Convert incoming bytes (PCM float32) to numpy array
                            chunk = np.frombuffer(message, dtype=np.float32)
                            audio_buffer.append(chunk)
                        continue

                    # Handle incoming JSON command messages
                    try:
                        data = json.loads(message)
                    except json.JSONDecodeError:
                        print("Warning: Received invalid JSON string.", file=sys.stderr)
                        continue

                    msg_type = data.get("type")

                    if msg_type == "start":
                        voice_profile = data.get("voice", self.voice_name)
                        print(f"Client started session. Selected voice: {voice_profile}")
                        await websocket.send(json.dumps({"type": "status", "status": "idle"}))

                    elif msg_type == "speech_start":
                        print("\n[VAD] User started speaking...")
                        is_user_speaking = True
                        audio_buffer = []

                        # Immediate barge-in interrupt: Cancel active agent outputs
                        if active_turn_task and not active_turn_task.done():
                            print("[Barge-in] Interrupting active agent generation.")
                            active_turn_task.cancel()
                            await conversation.cancel()

                        await websocket.send(json.dumps({"type": "status", "status": "listening"}))
                        await websocket.send(json.dumps({"type": "interrupt"}))

                    elif msg_type == "speech_end":
                        print("[VAD] User finished speaking. Processing...")
                        is_user_speaking = False
                        await websocket.send(json.dumps({"type": "status", "status": "thinking"}))

                        if not audio_buffer:
                            print("Warning: Audio buffer is empty.", file=sys.stderr)
                            await websocket.send(json.dumps({"type": "status", "status": "idle"}))
                            continue

                        # Transcribe the accumulated audio data
                        audio_data = np.concatenate(audio_buffer)
                        audio_buffer = []

                        # Execute transcription in a separate thread to prevent blocking the async loop
                        loop = asyncio.get_running_loop()
                        transcription = await loop.run_in_executor(
                            None, self.stt.transcribe, audio_data
                        )

                        if not transcription or not transcription.strip():
                            print("[STT] No speech detected in audio.")
                            await websocket.send(json.dumps({"type": "status", "status": "idle"}))
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

                        # Trigger agent generation task
                        active_turn_task = asyncio.create_task(
                            self.process_agent_turn(
                                websocket, conversation, transcription, voice_profile
                            )
                        )

                    elif msg_type == "interrupt":
                        print("[Command] Interrupt received from client.")
                        if active_turn_task and not active_turn_task.done():
                            active_turn_task.cancel()
                            await conversation.cancel()
                        await websocket.send(json.dumps({"type": "status", "status": "idle"}))

            except websockets.exceptions.ConnectionClosed:
                print(f"Connection closed by: {websocket.remote_address}")
            finally:
                if active_turn_task and not active_turn_task.done():
                    active_turn_task.cancel()

    async def process_agent_turn(
        self, websocket: Any, conversation: Any, transcription: str, voice_profile: str
    ) -> None:
        """Sends the user message to the agent, receives streams, and sends audio/text to client."""
        try:
            print("[Agent] Generating response...")
            await websocket.send(json.dumps({"type": "status", "status": "thinking"}))

            sentence_buffer = []

            async for chunk in conversation.chat(Content(text=transcription)):
                if not chunk.text:
                    continue

                text_delta = chunk.text
                print(text_delta, end="", flush=True)
                sentence_buffer.append(text_delta)

                await websocket.send(
                    json.dumps(
                        {
                            "type": "transcript",
                            "role": "agent",
                            "text": text_delta,
                            "is_delta": True,
                        }
                    )
                )

                # Process TTS concurrently on sentence boundaries for lower latency
                full_text = "".join(sentence_buffer)
                if any(punct in text_delta for punct in [".", "?", "!", "\n"]):
                    # Strip whitespace and check if it contains actual words
                    stripped_text = full_text.strip()
                    if stripped_text:
                        await self.synthesize_and_stream(websocket, stripped_text, voice_profile)
                    sentence_buffer = []

            # Handle any remaining text in the buffer
            if sentence_buffer:
                remaining_text = "".join(sentence_buffer).strip()
                if remaining_text:
                    await self.synthesize_and_stream(websocket, remaining_text, voice_profile)

            print("\n[Agent] Response finished.")
            await websocket.send(json.dumps({"type": "status", "status": "idle"}))

        except asyncio.CancelledError:
            print("\n[Agent] Turn processing cancelled.")
        except Exception as e:  # pylint: disable=broad-except
            print(f"\nError processing agent turn: {e}", file=sys.stderr)
            await websocket.send(json.dumps({"type": "error", "message": str(e)}))
            await websocket.send(json.dumps({"type": "status", "status": "idle"}))

    async def synthesize_and_stream(
        self, websocket: Any, text: str, voice_profile: str
    ) -> None:
        """Runs TTS on the text chunk and streams the generated PCM binary to client."""
        try:
            await websocket.send(json.dumps({"type": "status", "status": "speaking"}))
            loop = asyncio.get_running_loop()

            # Run synthesis in a thread pool executor to avoid freezing the event loop
            samples, sr = await loop.run_in_executor(
                None, self.tts.synthesize, text, voice_profile
            )

            # Send headers indicating start of audio chunk
            await websocket.send(
                json.dumps(
                    {
                        "type": "audio_start",
                        "sample_rate": sr,
                    }
                )
            )

            # Send the raw PCM float32 bytes as a binary message
            await websocket.send(samples.tobytes())

            # Send end headers
            await websocket.send(json.dumps({"type": "audio_end"}))

        except asyncio.CancelledError:
            raise
        except Exception as e:  # pylint: disable=broad-except
            print(f"TTS Synthesis error for text '{text}': {e}", file=sys.stderr)


async def main() -> None:
    parser = argparse.ArgumentParser(description="Material Design 3 Voice Server")
    parser.add_argument("--host", type=str, default="127.0.0.1", help="Host address to bind to")
    parser.add_argument("--port", type=int, default=8789, help="Port to run the WebSocket server on")
    parser.add_argument(
        "--whisper-model", type=str, default="base", help="Size or path of the Whisper model"
    )
    parser.add_argument(
        "--kokoro-model", type=str, default=None, help="Path to Kokoro ONNX file"
    )
    parser.add_argument(
        "--voices-path", type=str, default=None, help="Path to voices.json configuration"
    )
    parser.add_argument(
        "--voice-name", type=str, default="af_bella", help="Default Kokoro voice"
    )
    parser.add_argument(
        "--models-dir", type=str, default=DEFAULT_MODELS_DIR, help="Dir to store downloaded models"
    )

    args = parser.parse_args()

    # Automatically resolve or download models
    if not args.kokoro_model or not args.voices_path:
        print("Resolving Kokoro model and voices paths...")
        resolved_model, resolved_voices = ensure_models_exist(args.models_dir)
        kokoro_model_path = args.kokoro_model or resolved_model
        kokoro_voices_path = args.voices_path or resolved_voices
    else:
        kokoro_model_path = args.kokoro_model
        kokoro_voices_path = args.voices_path

    print(f"Using Whisper model: {args.whisper_model}")
    print(f"Using Kokoro model: {kokoro_model_path}")
    print(f"Using voices file: {kokoro_voices_path}")

    # Load local Speech-to-Text
    print("Loading local Whisper model...")
    stt = LocalWhisperSpeechToText(
        model_size_or_path=args.whisper_model,
        device="cpu",
        compute_type="int8",
    )

    # Load local Text-to-Speech
    print("Loading local Kokoro TTS...")
    tts = LocalKokoroTextToSpeech(
        model_path=kokoro_model_path,
        voices_path=kokoro_voices_path,
    )

    voice_server = VoiceServer(stt, tts, args.voice_name)

    print(f"Starting WebSocket server on ws://{args.host}:{args.port}")
    async with websockets.serve(voice_server.handle_connection, args.host, args.port):
        await asyncio.Future()  # run forever


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nExiting voice server...")
