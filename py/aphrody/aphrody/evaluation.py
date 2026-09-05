# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Local, keyless, and offline model evaluation metrics."""

from __future__ import annotations

import collections
import re
from typing import Any

_MODEL_CACHE: dict[str, Any] = {}


def _get_embedding_model() -> Any | None:
    """Get the text embedding model, caching it at the module level."""
    if "model" not in _MODEL_CACHE:
        try:
            from fastembed import TextEmbedding

            _MODEL_CACHE["model"] = TextEmbedding()
        except ImportError:
            _MODEL_CACHE["model"] = None
    return _MODEL_CACHE["model"]


def _tokenize(text: str) -> list[str]:
    """Tokenise text into lowercase words/numbers."""
    return re.findall(r"\w+", text.lower())


def exact_match(prediction: str, target: str) -> float:
    """Calculate exact match score (1.0 if identical else 0.0)."""
    return 1.0 if prediction.strip() == target.strip() else 0.0


def bleu_score(prediction: str, target: str) -> float:
    """Calculate a simple token-level BLEU-1 score (precision)."""
    pred_tokens = _tokenize(prediction)
    target_tokens = _tokenize(target)
    if not pred_tokens or not target_tokens:
        return 0.0
    pred_counts = collections.Counter(pred_tokens)
    target_counts = collections.Counter(target_tokens)
    overlap = sum(
        min(count, target_counts[tok]) for tok, count in pred_counts.items()
    )
    return overlap / len(pred_tokens)


def rouge_l_score(prediction: str, target: str) -> float:
    """Calculate ROUGE-L score using Longest Common Subsequence (LCS)."""
    pred_tokens = _tokenize(prediction)
    target_tokens = _tokenize(target)
    m, n = len(pred_tokens), len(target_tokens)
    if m == 0 or n == 0:
        return 0.0

    # Standard dynamic programming for LCS length
    dp = [[0] * (n + 1) for _ in range(m + 1)]
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if pred_tokens[i - 1] == target_tokens[j - 1]:
                dp[i][j] = dp[i - 1][j - 1] + 1
            else:
                dp[i][j] = max(dp[i - 1][j], dp[i][j - 1])
    lcs_len = dp[m][n]

    # Calculate precision, recall, and F1
    precision = lcs_len / m
    recall = lcs_len / n
    if precision + recall == 0:
        return 0.0
    return (2 * precision * recall) / (precision + recall)


def semantic_similarity(prediction: str, target: str) -> float:
    """Calculate semantic similarity using fastembed or fallback to Jaccard similarity.

    If fastembed is importable:
      Instantiate a text embedding model (or cache it at the module level).
      Generate embeddings for prediction and target and calculate their cosine similarity.
    If fastembed is NOT importable:
      Fall back gracefully to a token-level Jaccard similarity of the unique tokens of both strings.
      Ensure it returns a float between 0.0 and 1.0.
    """
    model = _get_embedding_model()
    if model is not None:
        import numpy as np

        embeddings = list(model.embed([prediction, target]))
        vec1 = np.asarray(embeddings[0])
        vec2 = np.asarray(embeddings[1])
        norm1 = np.linalg.norm(vec1)
        norm2 = np.linalg.norm(vec2)
        if norm1 == 0.0 or norm2 == 0.0:
            if norm1 == 0.0 and norm2 == 0.0:
                return 1.0
            return 0.0
        similarity = np.dot(vec1, vec2) / (norm1 * norm2)
        return float(similarity)

    # Fallback to token-level Jaccard similarity
    pred_tokens = set(_tokenize(prediction))
    target_tokens = set(_tokenize(target))
    if not pred_tokens and not target_tokens:
        return 1.0
    if not pred_tokens or not target_tokens:
        return 0.0
    intersect = pred_tokens.intersection(target_tokens)
    union = pred_tokens.union(target_tokens)
    return float(len(intersect) / len(union))


class LocalEvaluator:
    """Offline evaluator for text models and generated outputs."""

    def evaluate(self, prediction: str, target: str) -> dict[str, float]:
        """Run all local evaluation metrics.

        Args:
            prediction: The generated string to evaluate.
            target: The ground truth target string.

        Returns:
            A dictionary containing exact_match, bleu, rouge_l, and semantic_similarity scores.
        """
        return {
            "exact_match": exact_match(prediction, target),
            "bleu": bleu_score(prediction, target),
            "rouge_l": rouge_l_score(prediction, target),
            "semantic_similarity": semantic_similarity(prediction, target),
        }
