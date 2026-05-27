# SPDX-License-Identifier: Apache-2.0
"""Tests for the local evaluation metrics in :mod:`aphrody.evaluation`."""

from __future__ import annotations

from unittest.mock import patch

import numpy as np
import pytest
from aphrody.evaluation import (
    LocalEvaluator,
    bleu_score,
    exact_match,
    rouge_l_score,
    semantic_similarity,
)


def test_exact_match() -> None:
    assert exact_match("hello world", "hello world") == 1.0
    assert exact_match("  hello  ", "hello") == 1.0
    assert exact_match("hello", "world") == 0.0


def test_bleu_score() -> None:
    assert bleu_score("hello world", "hello world") == 1.0
    assert bleu_score("hello world", "hello") == 0.5
    assert bleu_score("hello", "") == 0.0


def test_rouge_l_score() -> None:
    assert rouge_l_score(
        "the quick brown fox", "the brown fox"
    ) == pytest.approx(0.857, abs=1e-3)
    assert rouge_l_score("hello", "world") == 0.0


def test_local_evaluator() -> None:
    evaluator = LocalEvaluator()
    with patch("aphrody.evaluation._get_embedding_model", return_value=None):
        res = evaluator.evaluate("hello world", "hello world")
        assert res == {
            "exact_match": 1.0,
            "bleu": 1.0,
            "rouge_l": 1.0,
            "semantic_similarity": 1.0,
        }


def test_semantic_similarity_fallback() -> None:
    # Force fallback by mocking _get_embedding_model to return None
    with patch("aphrody.evaluation._get_embedding_model", return_value=None):
        # exact match
        assert semantic_similarity("hello world", "hello world") == 1.0
        # empty strings
        assert semantic_similarity("", "") == 1.0
        # one empty
        assert semantic_similarity("hello", "") == 0.0
        # overlap
        # tokens for "hello world": {"hello", "world"}
        # tokens for "hello": {"hello"}
        # intersection: {"hello"} (size 1)
        # union: {"hello", "world"} (size 2)
        # score = 1 / 2 = 0.5
        assert semantic_similarity("hello world", "hello") == 0.5
        # completely different
        assert semantic_similarity("hello", "world") == 0.0


def test_semantic_similarity_fastembed() -> None:
    class FakeEmbedder:
        def embed(self, texts: list[str]) -> list[np.ndarray]:
            res = []
            for t in texts:
                if "world" in t:
                    res.append(np.array([1.0, 0.0], dtype="float32"))
                else:
                    res.append(np.array([0.6, 0.8], dtype="float32"))
            return res

    with patch(
        "aphrody.evaluation._get_embedding_model", return_value=FakeEmbedder()
    ):
        score = semantic_similarity("hello world", "hello")
        assert score == pytest.approx(0.6, abs=1e-5)

    # zero vector case
    class ZeroEmbedder:
        def embed(self, texts: list[str]) -> list[np.ndarray]:
            return [np.array([0.0, 0.0], dtype="float32") for _ in texts]

    with patch(
        "aphrody.evaluation._get_embedding_model", return_value=ZeroEmbedder()
    ):
        assert semantic_similarity("hello", "world") == 1.0  # both are zero

    # semi-zero vector case
    class SemiZeroEmbedder:
        def embed(self, texts: list[str]) -> list[np.ndarray]:
            return [
                np.array([0.0, 0.0], dtype="float32"),
                np.array([1.0, 0.0], dtype="float32"),
            ]

    with patch(
        "aphrody.evaluation._get_embedding_model",
        return_value=SemiZeroEmbedder(),
    ):
        assert semantic_similarity("hello", "world") == 0.0
