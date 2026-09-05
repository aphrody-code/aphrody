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

"""Text-to-Speech module using local Kokoro model."""

from __future__ import annotations

from typing import Protocol

import numpy as np

try:
    from kokoro_onnx import Kokoro

    HAS_KOKORO = True
except ImportError:
    HAS_KOKORO = False


class TextToSpeech(Protocol):
    """Protocol defining the Text-to-Speech interface."""

    def synthesize(self, text: str, voice: str) -> tuple[np.ndarray, int]:
        """Synthesize text to audio.

        Args:
            text: Text to synthesize.
            voice: Name of the voice to use.

        Returns:
            A tuple of (audio_samples, sample_rate).
        """
        ...


class LocalKokoroTextToSpeech:
    """Local Text-to-Speech implementation using Kokoro ONNX model."""

    def __init__(self, model_path: str, voices_path: str) -> None:
        """Initialize the Kokoro TTS model.

        Args:
            model_path: Path to the kokoro ONNX model file.
            voices_path: Path to the voices JSON configuration file.

        Raises:
            ImportError: If kokoro-onnx package is not installed.
        """
        if not HAS_KOKORO:
            raise ImportError(
                "kokoro-onnx is not installed. Install the 'voice' extra."
            )

        self.kokoro = Kokoro(model_path, voices_path)

    def synthesize(
        self, text: str, voice: str = "af_bella", lang: str | None = None
    ) -> tuple[np.ndarray, int]:
        """Synthesize text to audio.

        Args:
            text: Text to synthesize.
            voice: Name of the voice to use.
            lang: Optional language code (e.g. 'fr-fr', 'ja', 'en-us'). If None,
              it is auto-detected from the voice name prefix.

        Returns:
            A tuple of (audio_samples, sample_rate) where audio_samples is a
            numpy array of mono float32 samples at 24000Hz.
        """
        if lang is None:
            # Auto-detect language from voice prefix
            if voice.startswith("jf") or voice.startswith("jm"):
                lang = "ja"
            elif voice.startswith("ff") or voice.startswith("fm"):
                lang = "fr-fr"
            elif voice.startswith("bf") or voice.startswith("bm"):
                lang = "en-gb"
            else:
                lang = "en-us"

        samples, sample_rate = self.kokoro.create(
            text,
            voice=voice,
            speed=1.0,
            lang=lang,
        )
        return np.array(samples, dtype=np.float32), sample_rate
