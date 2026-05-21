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

"""Speech-to-Text module using local Whisper model."""

from __future__ import annotations

from typing import Protocol

import numpy as np

try:
    from faster_whisper import WhisperModel

    HAS_FASTER_WHISPER = True
except ImportError:
    HAS_FASTER_WHISPER = False


class SpeechToText(Protocol):
    """Protocol defining the Speech-to-Text interface."""

    def transcribe(self, audio_data: np.ndarray) -> str:
        """Transcribe PCM float32 mono audio at 16kHz to text.

        Args:
            audio_data: Numpy array of mono float32 audio samples.

        Returns:
            The transcribed text.
        """
        ...


class LocalWhisperSpeechToText:
    """Local Speech-to-Text implementation using faster-whisper (CTranslate2)."""

    def __init__(
        self,
        model_size_or_path: str = "base",
        device: str = "cpu",
        compute_type: str = "default",
    ) -> None:
        """Initialize the local Whisper model.

        Args:
            model_size_or_path: Size of the model (e.g. 'base', 'small') or absolute path.
            device: Run on 'cpu' or 'cuda'.
            compute_type: Computation precision (e.g. 'int8', 'float16').

        Raises:
            ImportError: If faster-whisper package is not installed.
        """
        if not HAS_FASTER_WHISPER:
            raise ImportError(
                "faster-whisper is not installed. Install the 'voice' extra."
            )

        self.model = WhisperModel(
            model_size_or_path,
            device=device,
            compute_type=compute_type,
        )

    def transcribe(self, audio_data: np.ndarray) -> str:
        """Transcribe PCM float32 mono audio at 16kHz to text.

        Args:
            audio_data: Numpy array of mono float32 audio samples.

        Returns:
            The transcribed text.
        """
        segments, _ = self.model.transcribe(audio_data, beam_size=5)
        text_segments = [segment.text for segment in segments]
        return " ".join(text_segments).strip()
