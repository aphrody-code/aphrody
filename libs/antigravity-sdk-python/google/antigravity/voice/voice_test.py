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

"""Tests for Local Voice agent, Speech-to-Text, and Text-to-Speech submodules."""

import asyncio
import unittest
from unittest import mock

import numpy as np

# We mock sounddevice during import so the tests run smoothly
# regardless of whether the actual native audio libraries/devices are present.
with (
    mock.patch("sounddevice.InputStream"),
    mock.patch("sounddevice.OutputStream"),
):
    from google.antigravity.voice import (
        LocalKokoroTextToSpeech,
        LocalVoiceAgentLoop,
        LocalWhisperSpeechToText,
    )


class VoiceTest(unittest.IsolatedAsyncioTestCase):
    @mock.patch("google.antigravity.voice.stt.WhisperModel")
    def test_whisper_stt(self, mock_whisper_model_class):
        mock_instance = mock.MagicMock()
        mock_whisper_model_class.return_value = mock_instance

        mock_segment = mock.MagicMock()
        mock_segment.text = "Hello world"
        mock_instance.transcribe.return_value = ([mock_segment], None)

        stt = LocalWhisperSpeechToText(model_size_or_path="base")
        audio_data = np.zeros(16000, dtype=np.float32)
        transcription = stt.transcribe(audio_data)

        self.assertEqual(transcription, "Hello world")
        mock_instance.transcribe.assert_called_once_with(
            audio_data, beam_size=5, language=None
        )

    @mock.patch("google.antigravity.voice.tts.Kokoro")
    def test_kokoro_tts(self, mock_kokoro_class):
        mock_instance = mock.MagicMock()
        mock_kokoro_class.return_value = mock_instance

        mock_instance.create.return_value = ([0.1, 0.2, 0.3], 24000)

        tts = LocalKokoroTextToSpeech(
            model_path="dummy.onnx", voices_path="dummy.json"
        )
        samples, sr = tts.synthesize("Hello", voice="af_bella")

        np.testing.assert_array_almost_equal(
            samples, np.array([0.1, 0.2, 0.3], dtype=np.float32)
        )
        self.assertEqual(sr, 24000)
        mock_instance.create.assert_called_once_with(
            "Hello", voice="af_bella", speed=1.0, lang="en-us"
        )

    @mock.patch("sounddevice.OutputStream")
    async def test_voice_agent_loop_playback(self, mock_output_stream_class):
        stt = mock.MagicMock()
        tts = mock.MagicMock()

        loop = LocalVoiceAgentLoop(stt=stt, tts=tts)

        audio_samples = np.zeros(100, dtype=np.float32)
        await loop.playback_queue.put((audio_samples, 24000))

        playback_task = asyncio.create_task(loop.play_audio_worker())

        await asyncio.sleep(0.05)
        playback_task.cancel()

        mock_output_stream_class.assert_called_once_with(
            samplerate=24000, channels=1
        )

    @mock.patch("sounddevice.InputStream")
    async def test_voice_agent_loop_run(self, mock_input_stream_class):
        stt = mock.MagicMock()
        tts = mock.MagicMock()

        loop = LocalVoiceAgentLoop(stt=stt, tts=tts)

        mock_agent = mock.MagicMock()
        mock_conv = mock.MagicMock()
        mock_agent.conversation = mock_conv

        silent_chunk = np.zeros(1000, dtype=np.float32)
        speech_chunk = np.ones(1000, dtype=np.float32) * 0.1

        get_calls = [silent_chunk, speech_chunk, asyncio.CancelledError()]

        async def mock_get():
            if not get_calls:
                await asyncio.sleep(1)
                raise asyncio.CancelledError()
            val = get_calls.pop(0)
            if isinstance(val, BaseException):
                raise val
            return val

        loop.audio_queue.get = mock_get

        with self.assertRaises(asyncio.CancelledError):
            await loop.run(mock_agent)
