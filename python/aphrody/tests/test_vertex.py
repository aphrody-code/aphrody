# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.vertex` resolution + streaming logic."""

from __future__ import annotations

from aphrody import vertex


def test_resolve_project(monkeypatch) -> None:
    monkeypatch.delenv("APHRODY_VERTEX_PROJECT", raising=False)
    monkeypatch.delenv("GOOGLE_CLOUD_PROJECT", raising=False)
    assert vertex.resolve_project() == vertex.DEFAULT_VERTEX_PROJECT
    assert vertex.resolve_project("explicit") == "explicit"
    monkeypatch.setenv("APHRODY_VERTEX_PROJECT", "envproj")
    assert vertex.resolve_project() == "envproj"


def test_resolve_location(monkeypatch) -> None:
    monkeypatch.delenv("APHRODY_VERTEX_LOCATION", raising=False)
    assert vertex.resolve_location() == vertex.DEFAULT_VERTEX_LOCATION
    assert vertex.resolve_location("europe-west1") == "europe-west1"


def test_stream_yields_nonempty_text() -> None:
    class _Chunk:
        def __init__(self, text: str) -> None:
            self.text = text

    class _Models:
        def generate_content_stream(self, *, model, contents, config):
            return iter([_Chunk("Hel"), _Chunk(""), _Chunk("lo")])

    class _FakeClient:
        models = _Models()

    gem = vertex.GeminiVertex.__new__(vertex.GeminiVertex)
    gem.model = "m"
    gem._client = _FakeClient()
    assert list(gem.stream("hi")) == ["Hel", "lo"]
