# SPDX-License-Identifier: Apache-2.0
"""Tests for the local evaluation CLI commands in :mod:`aphrody.cli.evaluation`."""

from __future__ import annotations

from unittest import mock

from aphrody.cli import Aphrody


def test_cli_evaluate_text() -> None:
    cli = Aphrody()
    with mock.patch("aphrody.cli.evaluation._emit") as mock_emit:
        with mock.patch(
            "aphrody.evaluation._get_embedding_model", return_value=None
        ):
            cli.evaluate().text("hello world", "hello world")
            mock_emit.assert_called_once_with(
                {
                    "exact_match": 1.0,
                    "bleu": 1.0,
                    "rouge_l": 1.0,
                    "semantic_similarity": 1.0,
                }
            )


def test_cli_evaluate_file(tmp_path) -> None:
    dataset = tmp_path / "dataset.jsonl"
    dataset.write_text(
        '{"prediction": "hello world", "target": "hello"}\n'
        '{"prediction": "foo", "target": "bar"}\n',
        encoding="utf-8",
    )

    cli = Aphrody()
    with mock.patch("aphrody.cli.evaluation._emit") as mock_emit:
        with mock.patch(
            "aphrody.evaluation._get_embedding_model", return_value=None
        ):
            cli.evaluate().file(str(dataset))
            mock_emit.assert_called_once()
            args, _ = mock_emit.call_args
            res = args[0]
            assert res["records"] == 2
            assert res["average_exact_match"] == 0.0
            assert res["average_bleu"] == 0.25
            assert res["average_rouge_l"] > 0.0
            assert res["average_semantic_similarity"] == 0.25
