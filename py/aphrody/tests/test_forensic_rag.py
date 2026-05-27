# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.forensic.rag` (mocked fastembed, real numpy)."""

from __future__ import annotations

from typing import ClassVar

import numpy as np
from aphrody.forensic import rag


class _FakeEmbedder:
    """Deterministic bag-of-keywords embedder over a tiny fixed vocabulary."""

    VOCAB: ClassVar = ["auth", "token", "render", "network", "agent", "model"]

    def embed(self, texts):
        for t in texts:
            low = t.lower()
            vec = [float(low.count(w)) for w in self.VOCAB]
            if not any(vec):
                vec[0] = 0.001  # avoid all-zero
            yield np.asarray(vec, dtype="float32")


def test_chunk_text():
    chunks = rag.chunk_text("x" * 3000, size=1200, overlap=200)
    assert len(chunks) >= 3
    assert all(len(c) <= 1200 for c in chunks)


def test_chunk_text_short():
    assert rag.chunk_text("small") == ["small"]
    assert rag.chunk_text("") == []


def test_add_files_and_query(tmp_path):
    auth = tmp_path / "auth.js"
    auth.write_text("auth token auth token oauth flow", encoding="utf-8")
    render = tmp_path / "render.js"
    render.write_text("render render network draw frame", encoding="utf-8")

    index = rag.RagIndex(embedder=_FakeEmbedder())
    added = index.add_files([auth, render])
    assert added >= 2
    assert index.size >= 2

    hits = index.query("token auth login", k=1)
    assert len(hits) == 1
    assert hits[0]["doc"].endswith("auth.js")


def test_save_and_load(tmp_path):
    f = tmp_path / "a.js"
    f.write_text("agent model agent network", encoding="utf-8")
    index = rag.build_index([f], embedder=_FakeEmbedder())
    saved = index.save(tmp_path / "idx")
    assert saved["chunks"] == index.size

    loaded = rag.RagIndex.load(tmp_path / "idx", embedder=_FakeEmbedder())
    assert loaded.size == index.size
    hits = loaded.query("agent network", k=1)
    assert hits and hits[0]["doc"].endswith("a.js")


def test_query_empty_index():
    index = rag.RagIndex(embedder=_FakeEmbedder())
    assert index.query("anything", k=3) == []
