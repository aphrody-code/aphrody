# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.voice_server` persona + brain logic.

Skipped when the optional voice dependencies (``aphrody[voice]``) are absent.
"""

from __future__ import annotations

import pytest

pytest.importorskip("google.antigravity.voice")

from aphrody import voice_server


def test_system_instruction_for() -> None:
    assert voice_server.system_instruction_for("ff_siwis") == (
        voice_server.SYSTEM_INSTRUCTIONS_FR
    )
    assert voice_server.system_instruction_for("jf_x") == (
        voice_server.SYSTEM_INSTRUCTIONS_JA
    )
    assert voice_server.system_instruction_for("af_heart") == (
        voice_server.SYSTEM_INSTRUCTIONS_EN
    )


def test_whisper_language_for() -> None:
    assert voice_server.whisper_language_for("ff_siwis") == "fr"
    assert voice_server.whisper_language_for("jm_x") == "ja"
    assert voice_server.whisper_language_for("am_x") == "en"


def test_voice_brain_streams_and_records(monkeypatch) -> None:
    class _FakeVertex:
        def __init__(self, *args, **kwargs) -> None:
            pass

        def stream(
            self, contents, *, system_instruction=None, temperature=None
        ):
            return iter(["Bon", "jour"])

    monkeypatch.setattr(voice_server, "GeminiVertex", _FakeVertex)
    brain = voice_server.VoiceBrain("sys")
    assert list(brain.stream_reply("salut")) == ["Bon", "jour"]
    assert brain.history[0] == {"role": "user", "parts": [{"text": "salut"}]}
    assert brain.history[1] == {
        "role": "model",
        "parts": [{"text": "Bonjour"}],
    }
