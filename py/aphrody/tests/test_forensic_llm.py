# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.forensic.llm` (mocked GeminiVertex, no network)."""

from __future__ import annotations

from typing import ClassVar

from aphrody.forensic import rag
from aphrody.forensic.llm import ForensicLLM, _parse_json_object


class _FakeGemini:
    """A GeminiVertex stand-in: streams a canned response, captures prompts."""

    def __init__(self, response):
        self._response = response
        self.last_prompt = None
        self.last_system = None

    def stream(
        self, contents, *, model=None, system_instruction=None, temperature=None
    ):
        self.last_prompt = contents
        self.last_system = system_instruction
        for word in self._response.split(" "):
            yield word + " "


class _FakeEmbedder:
    VOCAB: ClassVar = ["auth", "agent"]

    def embed(self, texts):
        import numpy as np

        for t in texts:
            low = t.lower()
            v = [float(low.count(w)) for w in self.VOCAB]
            if not any(v):
                v[0] = 0.001
            yield np.asarray(v, dtype="float32")


_INV = {
    "summary": {"files": 3, "dirs": 1, "markers": {"go": 1}, "secret_files": []}
}
_CLS = {"total": 3, "by_category": {"code": 2, "go-binary": 1}}


def test_synthesize_streams_and_uses_system():
    g = _FakeGemini("Antigravity is a Windsurf fork.")
    llm = ForensicLLM(gemini=g)
    out = llm.synthesize(inventory=_INV, classification=_CLS, pe_reports=[])
    assert "Antigravity" in out
    assert "reverse-engineering" in g.last_system.lower()


def test_ask_with_rag():
    f_g = _FakeGemini("It authenticates via OAuth.")
    index = rag.RagIndex(embedder=_FakeEmbedder())
    import pathlib
    import tempfile

    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "auth.js"
        p.write_text("auth auth oauth", encoding="utf-8")
        index.add_files([p])
        llm = ForensicLLM(gemini=f_g)
        res = llm.ask("How does auth work?", rag=index, k=1)
    assert res["question"] == "How does auth work?"
    assert "OAuth" in res["answer"]
    assert len(res["passages"]) == 1
    assert "Retrieved passages" in f_g.last_prompt


def test_ask_without_rag():
    g = _FakeGemini("No source available.")
    llm = ForensicLLM(gemini=g)
    res = llm.ask("anything?", rag=None)
    assert res["passages"] == []
    assert "No source" in res["answer"]


def test_auto_ml_parses_json():
    payload = (
        '{"architecture": "Electron shell + Go LS", '
        '"components": [{"name": "language_server", "language": "go", '
        '"runtime": "native", "role": "agent", "tags": ["cortex"]}]}'
    )
    g = _FakeGemini(payload)
    llm = ForensicLLM(gemini=g)
    out = llm.auto_ml(inventory=_INV, classification=_CLS, pe_reports=[])
    assert out["architecture"].startswith("Electron")
    assert out["components"][0]["language"] == "go"


def test_parse_json_object_with_fence():
    text = '```json\n{"a": 1}\n```'
    assert _parse_json_object(text) == {"a": 1}


def test_parse_json_object_garbage():
    assert _parse_json_object("not json at all") == {}
