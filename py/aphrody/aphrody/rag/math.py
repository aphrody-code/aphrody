# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""High-performance math and ranking functions for Python RAG.

Provides Rust-accelerated versions of cosine similarity, top-K search,
and Reciprocal Rank Fusion (RRF), with pure-Python fallbacks.
"""

from __future__ import annotations

import math

try:
    from aphrody.aphrody_rust import (
        cosine_similarity as rust_cosine_similarity,
    )
    from aphrody.aphrody_rust import (
        reciprocal_rank_fusion as rust_reciprocal_rank_fusion,
    )
    from aphrody.aphrody_rust import (
        top_k_cosine_similarity as rust_top_k_cosine_similarity,
    )

    HAS_RUST = True
except ImportError:
    HAS_RUST = False


def py_cosine_similarity(v1: list[float], v2: list[float]) -> float:
    """Pure-Python implementation of cosine similarity."""
    if len(v1) != len(v2) or not v1:
        raise ValueError("Vectors must be non-empty and of same length")
    dot = 0.0
    norm_a = 0.0
    norm_b = 0.0
    for a, b in zip(v1, v2):
        dot += a * b
        norm_a += a * a
        norm_b += b * b
    if norm_a == 0.0 or norm_b == 0.0:
        return 0.0
    return dot / (math.sqrt(norm_a) * math.sqrt(norm_b))


def py_top_k_cosine_similarity(
    query: list[float],
    embeddings: list[list[float]],
    k: int,
) -> list[tuple[int, float]]:
    """Pure-Python implementation of top-K cosine similarity search."""
    if not query:
        raise ValueError("Query vector must be non-empty")
    results = []
    for idx, emb in enumerate(embeddings):
        if len(emb) != len(query):
            continue
        score = py_cosine_similarity(query, emb)
        results.append((idx, score))
    results.sort(key=lambda x: x[1], reverse=True)
    return results[:k]


def py_reciprocal_rank_fusion(
    rankings: list[list[int]],
    k: float = 60.0,
) -> list[tuple[int, float]]:
    """Pure-Python implementation of Reciprocal Rank Fusion (RRF)."""
    scores: dict[int, float] = {}
    for ranking in rankings:
        for rank, item_id in enumerate(ranking):
            score = 1.0 / (k + rank)
            scores[item_id] = scores.get(item_id, 0.0) + score
    results = list(scores.items())
    results.sort(key=lambda x: x[1], reverse=True)
    return results


def cosine_similarity(v1: list[float], v2: list[float]) -> float:
    """Calculate cosine similarity using Rust if available, else Python."""
    if HAS_RUST:
        try:
            return rust_cosine_similarity(v1, v2)
        except Exception:
            pass
    return py_cosine_similarity(v1, v2)


def top_k_cosine_similarity(
    query: list[float],
    embeddings: list[list[float]],
    k: int,
) -> list[tuple[int, float]]:
    """Get top-K elements by cosine similarity using Rust if available."""
    if HAS_RUST:
        try:
            return rust_top_k_cosine_similarity(query, embeddings, k)
        except Exception:
            pass
    return py_top_k_cosine_similarity(query, embeddings, k)


def reciprocal_rank_fusion(
    rankings: list[list[int]],
    k: float = 60.0,
) -> list[tuple[int, float]]:
    """Fuse rank lists using RRF with Rust acceleration if available."""
    if HAS_RUST:
        try:
            return rust_reciprocal_rank_fusion(rankings, k)
        except Exception:
            pass
    return py_reciprocal_rank_fusion(rankings, k)
