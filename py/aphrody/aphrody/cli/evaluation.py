# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Offline model evaluation commands for the aphrody CLI."""

from __future__ import annotations

import json
from pathlib import Path

from aphrody.cli.utils import _emit


class EvaluationCommands:
    """``aphrody evaluate <action>`` — local, offline, and keyless model evaluation.

    Score model outputs against ground truth targets using exact match, BLEU-1,
    and ROUGE-L metrics calculated entirely client-side.
    """

    def text(self, prediction: str, target: str) -> None:
        """Evaluate a prediction string against a target ground truth.

        Args:
            prediction: Generated text output.
            target: Ground truth reference text.
        """
        from aphrody.evaluation import LocalEvaluator

        evaluator = LocalEvaluator()
        _emit(evaluator.evaluate(prediction, target))

    def file(self, path: str) -> None:
        """Batch-evaluate a JSON Lines (.jsonl) dataset file.

        Each line in the file must be a JSON object containing "prediction" and
        "target" string fields.

        Args:
            path: Path to the .jsonl dataset file.
        """
        from aphrody.evaluation import LocalEvaluator

        evaluator = LocalEvaluator()
        lines = Path(path).read_text(encoding="utf-8").splitlines()

        total_exact_match = 0.0
        total_bleu = 0.0
        total_rouge_l = 0.0
        total_semantic_similarity = 0.0
        count = 0

        for line in lines:
            if not line.strip():
                continue
            data = json.loads(line)
            pred = data.get("prediction", "")
            target = data.get("target", "")
            scores = evaluator.evaluate(pred, target)
            total_exact_match += scores["exact_match"]
            total_bleu += scores["bleu"]
            total_rouge_l += scores["rouge_l"]
            total_semantic_similarity += scores["semantic_similarity"]
            count += 1

        if count == 0:
            _emit({"error": "No valid data records found in dataset."})
            return

        _emit(
            {
                "records": count,
                "average_exact_match": total_exact_match / count,
                "average_bleu": total_bleu / count,
                "average_rouge_l": total_rouge_l / count,
                "average_semantic_similarity": total_semantic_similarity
                / count,
            }
        )
