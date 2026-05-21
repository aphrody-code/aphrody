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

"""Unified client loop for local speech-to-speech agent experience."""

from __future__ import annotations

import asyncio
import sys
from typing import Any

import numpy as np

try:
    import sounddevice as sd

    HAS_SOUNDDEVICE = True
except ImportError:
    HAS_SOUNDDEVICE = False

from google.antigravity.agent import Agent
from google.antigravity.types import Content
from google.antigravity.voice.stt import SpeechToText
from google.antigravity.voice.tts import TextToSpeech


class LocalVoiceAgentLoop:
    """Coordinates local STT, Agent, and TTS loops for a voice session."""

    def __init__(
        self,
        stt: SpeechToText,
        tts: TextToSpeech,
        voice_name: str = "af_bella",
        sample_rate: int = 16000,
        vad_energy_threshold: float = 0.02,
        silence_timeout_seconds: float = 0.6,
    ) -> None:
        """Initialize the local voice agent loop.

        Args:
            stt: Speech-to-Text transcriber.
            tts: Text-to-Speech synthesizer.
            voice_name: Voice profile to use for synthesis.
            sample_rate: Microphone sample rate (default 16000).
            vad_energy_threshold: Simple RMS energy threshold for speech activity.
            silence_timeout_seconds: Timeout for detecting speech boundary.

        Raises:
            ImportError: If sounddevice package is not installed.
        """
        if not HAS_SOUNDDEVICE:
            raise ImportError(
                "sounddevice is not installed. Install the 'voice' extra."
            )

        self.stt = stt
        self.tts = tts
        self.voice_name = voice_name
        self.sample_rate = sample_rate
        self.vad_energy_threshold = vad_energy_threshold
        self.silence_timeout_seconds = silence_timeout_seconds

        self.audio_queue: asyncio.Queue[np.ndarray] = asyncio.Queue()
        self.playback_queue: asyncio.Queue[tuple[np.ndarray, int]] = (
            asyncio.Queue()
        )
        self.interrupt_event = asyncio.Event()
        self.is_agent_speaking = False

    async def _mic_callback(
        self,
        indata: np.ndarray,
        frames: int,
        time: Any,
        status: Any,
    ) -> None:
        """Callback from sounddevice to receive microphone frames."""
        if status:
            print(f"Audio buffer status: {status}", file=sys.stderr)
        # Put audio frames into processing queue
        await self.audio_queue.put(indata.copy().flatten())

    async def play_audio_worker(self) -> None:
        """Asynchronous worker that plays generated voice responses."""
        while True:
            audio_data, sr = await self.playback_queue.get()
            self.is_agent_speaking = True
            self.interrupt_event.clear()

            # Start playback stream
            try:
                stream = sd.OutputStream(samplerate=sr, channels=1)
                stream.start()

                # Write in chunks to allow checking for interrupts
                chunk_size = 4000
                for i in range(0, len(audio_data), chunk_size):
                    if self.interrupt_event.is_set():
                        break
                    chunk = audio_data[i : i + chunk_size]
                    stream.write(chunk)

                stream.stop()
                stream.close()
            except Exception as e:  # pylint: disable=broad-except
                print(f"Error during audio playback: {e}", file=sys.stderr)
            finally:
                self.playback_queue.task_done()
                self.is_agent_speaking = False

    async def run(self, agent: Agent) -> None:
        """Main speech-to-speech loop.

        Args:
            agent: The standard Antigravity Agent.
        """
        conversation = agent.conversation
        speech_buffer = []
        silence_samples = 0
        speech_detected = False

        loop = asyncio.get_running_loop()

        def sync_callback(indata, frames, time, status):
            asyncio.run_coroutine_threadsafe(
                self._mic_callback(indata, frames, time, status), loop
            )

        # Start microphone input stream
        mic_stream = sd.InputStream(
            samplerate=self.sample_rate,
            channels=1,
            dtype="float32",
            callback=sync_callback,
        )
        mic_stream.start()

        print(
            "\n>>> Local speech-to-speech loop active. Speak into microphone."
        )

        try:
            while True:
                chunk = await self.audio_queue.get()

                # VAD calculation using RMS energy
                energy = float(np.sqrt(np.mean(chunk**2)))
                is_speech = energy > self.vad_energy_threshold

                if is_speech:
                    if self.is_agent_speaking:
                        print(
                            "\n[Barge-in] User speech detected. Interrupting output."
                        )
                        self.interrupt_event.set()

                        # Flush the playback queue
                        while not self.playback_queue.empty():
                            try:
                                self.playback_queue.get_nowait()
                                self.playback_queue.task_done()
                            except asyncio.QueueEmpty:
                                break
                        # Request turn cancellation from Agent connection
                        await conversation.cancel()

                    speech_detected = True
                    speech_buffer.append(chunk)
                    silence_samples = 0
                elif speech_detected:
                    silence_samples += len(chunk)
                    speech_buffer.append(chunk)

                    # Check if silence limit is reached
                    if (
                        silence_samples / self.sample_rate
                    ) >= self.silence_timeout_seconds:
                        print("\nUser finished speaking. Transcribing...")
                        speech_data = np.concatenate(speech_buffer)

                        # Reset buffers
                        speech_buffer = []
                        speech_detected = False

                        # 1. Run local STT
                        transcription = self.stt.transcribe(speech_data)

                        if transcription:
                            print(f"User: {transcription}")
                            print("Agent: ", end="", flush=True)

                            # 2. Feed text into Agent and capture streaming chunks
                            sentence_buffer = []
                            async for chunk_item in conversation.chat(
                                Content(text=transcription)
                            ):
                                if (
                                    hasattr(chunk_item, "text")
                                    and chunk_item.text
                                ):
                                    text_delta = chunk_item.text
                                    print(text_delta, end="", flush=True)
                                    sentence_buffer.append(text_delta)

                                    # Split on punctuation boundary for concurrent synthesis
                                    full_buffer = "".join(sentence_buffer)
                                    if any(
                                        punct in text_delta
                                        for punct in [".", "?", "!"]
                                    ):
                                        try:
                                            voice_samples, sr = (
                                                self.tts.synthesize(
                                                    full_buffer,
                                                    voice=self.voice_name,
                                                )
                                            )
                                            await self.playback_queue.put(
                                                (voice_samples, sr)
                                            )
                                        except Exception as e:  # pylint: disable=broad-except
                                            print(
                                                f"\nTTS Error: {e}",
                                                file=sys.stderr,
                                            )
                                        sentence_buffer = []

                            # Synthesize any trailing content
                            if sentence_buffer:
                                trailing_text = "".join(sentence_buffer)
                                try:
                                    voice_samples, sr = self.tts.synthesize(
                                        trailing_text,
                                        voice=self.voice_name,
                                    )
                                    await self.playback_queue.put(
                                        (voice_samples, sr)
                                    )
                                except Exception as e:  # pylint: disable=broad-except
                                    print(
                                        f"\nTTS Error: {e}",
                                        file=sys.stderr,
                                    )
                            print()
        finally:
            mic_stream.stop()
            mic_stream.close()
