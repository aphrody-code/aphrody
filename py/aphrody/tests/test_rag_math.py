# SPDX-License-Identifier: Apache-2.0
"""Tests for hybrid Python/Rust math and ranking operations."""

from __future__ import annotations

import pytest
from aphrody.rag.math import (
    HAS_RUST,
    cosine_similarity,
    py_cosine_similarity,
    py_reciprocal_rank_fusion,
    py_top_k_cosine_similarity,
    reciprocal_rank_fusion,
)


def test_py_math_cosine_similarity():
    v1 = [1.0, 0.0, 0.0]
    v2 = [1.0, 0.0, 0.0]
    assert py_cosine_similarity(v1, v2) == pytest.approx(1.0)

    v3 = [0.0, 1.0, 0.0]
    assert py_cosine_similarity(v1, v3) == pytest.approx(0.0)


def test_py_math_top_k():
    query = [1.0, 0.0]
    embeddings = [
        [0.0, 1.0],  # similarity 0
        [1.0, 0.0],  # similarity 1
        [0.707, 0.707],  # similarity 0.707
    ]
    results = py_top_k_cosine_similarity(query, embeddings, k=2)
    assert len(results) == 2
    assert results[0][0] == 1  # exact match first
    assert results[0][1] == pytest.approx(1.0)
    assert results[1][0] == 2  # next closest


def test_py_math_rrf():
    rankings = [
        [0, 1, 2],
        [1, 0, 2],
    ]
    results = py_reciprocal_rank_fusion(rankings, k=60.0)
    assert len(results) == 3
    assert results[2][0] == 2


def test_hybrid_facade():
    v1 = [1.0, 2.0, 3.0]
    v2 = [1.0, 2.0, 3.0]
    assert cosine_similarity(v1, v2) == pytest.approx(1.0)

    rankings = [[0, 1], [1, 0]]
    res = reciprocal_rank_fusion(rankings, k=60.0)
    assert len(res) == 2


def test_rust_bindings():
    if HAS_RUST:
        from aphrody.aphrody_rust import (
            cosine_similarity as rust_cos,
        )
        from aphrody.aphrody_rust import (
            reciprocal_rank_fusion as rust_rrf,
        )
        from aphrody.aphrody_rust import (
            top_k_cosine_similarity as rust_top,
        )

        v1 = [1.0, 0.0]
        v2 = [1.0, 0.0]
        assert rust_cos(v1, v2) == pytest.approx(1.0)

        query = [1.0, 0.0]
        embs = [[0.0, 1.0], [1.0, 0.0]]
        res = rust_top(query, embs, 1)
        assert len(res) == 1
        assert res[0][0] == 1
        assert res[0][1] == pytest.approx(1.0)

        rankings = [[0, 1], [1, 0]]
        res_rrf = rust_rrf(rankings, 60.0)
        assert len(res_rrf) == 2
