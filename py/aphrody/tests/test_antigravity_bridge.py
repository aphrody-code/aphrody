# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.antigravity_bridge` (keyless SDK transport).

These run only when the ``google-antigravity`` SDK is importable (workspace
member). The Gemini Vertex client is faked, so no network/credentials are
touched.
"""

from __future__ import annotations

import types as _pytypes

import pytest

ag = pytest.importorskip("google.antigravity")
from aphrody import antigravity_bridge as bridge  # noqa: E402

# ---------------------------------------------------------------------------
# Fake google-genai streaming client.
# ---------------------------------------------------------------------------


class _FakeModels:
    def __init__(self, deltas):
        self._deltas = deltas

    def generate_content_stream(self, *, model, contents, config):
        for d in self._deltas:
            yield _pytypes.SimpleNamespace(
                text=d,
                function_calls=None,
                candidates=[],
                usage_metadata=None,
            )


class _FakeGemini:
    def __init__(self, deltas):
        self.model = "gemini-2.5-flash"
        self._models = _FakeModels(deltas)

    @property
    def client(self):
        return _pytypes.SimpleNamespace(models=self._models)


def test_connection_streams_text():
    """A bare AphrodyConnection turn yields deltas and a terminal step."""
    import asyncio

    async def run():
        conn = bridge.AphrodyConnection(
            gemini=_FakeGemini(["Hel", "lo"]),
            conversation_id="t1",
        )
        await conn.send("hi")
        steps = [s async for s in conn.receive_steps()]
        return steps

    steps = asyncio.run(run())
    deltas = [s.content_delta for s in steps if s.content_delta]
    assert "".join(deltas) == "Hello"
    finals = [s for s in steps if s.is_complete_response]
    assert finals and finals[-1].content == "Hello"


def test_conversation_chat_via_bridge(monkeypatch):
    """Drive the SDK Conversation layer through the keyless strategy."""
    import asyncio

    from google.antigravity.conversation.conversation import Conversation

    # Make the strategy build our fake Gemini instead of a live one.
    def _fake_vertex(*args, **kwargs):
        return _FakeGemini(["world"])

    monkeypatch.setattr(bridge, "GeminiVertex", _fake_vertex)

    async def run():
        strat = bridge.AphrodyConnectionStrategy(model="gemini-2.5-flash")
        async with Conversation.create(strat) as conv:
            resp = await conv.chat("hello")
            text = await resp.text()
            return text, conv.last_response

    text, last = asyncio.run(run())
    assert text == "world"
    assert last == "world"


def test_agent_config_create_strategy():
    """AphrodyAgentConfig dispatches to the keyless strategy."""
    cfg = bridge.AphrodyAgentConfig(model="gemini-2.5-flash")
    strat = cfg.create_strategy(tool_runner=None, hook_runner=None)
    assert isinstance(strat, bridge.AphrodyConnectionStrategy)
    assert strat._model == "gemini-2.5-flash"


def test_content_to_genai_passthrough():
    assert bridge._content_to_genai("hi") == "hi"
    assert bridge._content_to_genai(None) == ""


def test_coerce_system_instruction_str():
    assert bridge._coerce_system_instruction("do x") == "do x"
    assert bridge._coerce_system_instruction(None) is None
