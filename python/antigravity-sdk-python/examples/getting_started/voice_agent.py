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

"""Example demonstrating local speech-to-speech agent experience.

This example runs a local loop that captures microphone audio, transcribes it,
sends it to the agent, and synthesizes the agent's response back to audio.

To run:
  python voice_agent.py --whisper-model base --kokoro-model path/to/kokoro.onnx --voices-path path/to/voices.json
"""

import argparse
import asyncio
import sys

from google.antigravity import Agent, LocalAgentConfig
from google.antigravity.voice import (
    LocalKokoroTextToSpeech,
    LocalVoiceAgentLoop,
    LocalWhisperSpeechToText,
)


async def main() -> None:
    parser = argparse.ArgumentParser(
        description="Local speech-to-speech voice agent example."
    )
    parser.add_argument(
        "--whisper-model",
        type=str,
        default="base",
        help="Whisper model size or path (e.g. 'base', 'small', 'tiny')",
    )
    parser.add_argument(
        "--kokoro-model",
        type=str,
        required=True,
        help="Path to the Kokoro ONNX model file (e.g. 'kokoro-v0_19.onnx')",
    )
    parser.add_argument(
        "--voices-path",
        type=str,
        required=True,
        help="Path to the voices.json config file",
    )
    parser.add_argument(
        "--voice-name",
        type=str,
        default="af_bella",
        help="Kokoro voice name to use (e.g. 'af_bella', 'af_sarah')",
    )
    parser.add_argument(
        "--energy-threshold",
        type=float,
        default=0.02,
        help="RMS energy threshold for VAD (speech activity detection)",
    )
    parser.add_argument(
        "--silence-timeout",
        type=float,
        default=0.6,
        help="Silence timeout in seconds to detect end of speech",
    )

    args = parser.parse_args()

    print("Initializing Speech-to-Text (Whisper)...")
    try:
        stt = LocalWhisperSpeechToText(
            model_size_or_path=args.whisper_model,
            device="cpu",
            compute_type="int8",
        )
    except Exception as e:
        print(f"Error initializing Whisper model: {e}", file=sys.stderr)
        sys.exit(1)

    print("Initializing Text-to-Speech (Kokoro)...")
    try:
        tts = LocalKokoroTextToSpeech(
            model_path=args.kokoro_model,
            voices_path=args.voices_path,
        )
    except Exception as e:
        print(f"Error initializing Kokoro TTS: {e}", file=sys.stderr)
        print(
            "Please download kokoro-v0_19.onnx and voices.json from HF hexgrad/Kokoro-82M.",
            file=sys.stderr,
        )
        sys.exit(1)

    print("Initializing Agent...")
    config = LocalAgentConfig()

    async with Agent(config) as voice_agent:
        loop = LocalVoiceAgentLoop(
            stt=stt,
            tts=tts,
            voice_name=args.voice_name,
            vad_energy_threshold=args.energy_threshold,
            silence_timeout_seconds=args.silence_timeout,
        )

        # Start the playback worker in the background
        playback_task = asyncio.create_task(loop.play_audio_worker())

        try:
            await loop.run(voice_agent)
        except KeyboardInterrupt:
            print("\nExiting voice agent...")
        finally:
            playback_task.cancel()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass
