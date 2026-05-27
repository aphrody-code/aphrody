# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Tests for the autonomous upgrade CLI subcommand."""

from __future__ import annotations

import subprocess
from pathlib import Path
from unittest import mock

import pytest
from aphrody.cli.web import WebCommands

import aphrody


@pytest.fixture
def restore_gemini_web():
    """Fixture to ensure gemini_web.py is restored after tests."""
    client_file = Path(aphrody.__file__).parent / "gemini_web.py"
    original_content = client_file.read_text(encoding="utf-8")
    yield client_file
    client_file.write_text(original_content, encoding="utf-8")


def test_auto_upgrade_no_changes(httpx_mock, restore_gemini_web) -> None:
    """Test when no changes are scraped from the Gemini app."""
    # Mock scraper to return current hashes
    mock_scraped = {
        "script_urls": ["/js/1.js"],
        "css_classes": [],
        "css_variables": [],
        "rpc_services": [],
        "rpc_methods": [],
        "rpc_mappings": {
            "MaZiqc": "BardFrontendService.ListConversations",
            "GzXR5e": "BardFrontendService.DeleteConversation",
        },
        "boq_hashes": [],
        "interactive_roles": [],
        "aria_attributes": [],
        "models": [],
        "feature_flags": [],
        "buttons": [],
    }

    mock_gv = mock.MagicMock()
    mock_gv.generate.return_value = '{"replacements": []}'

    with (
        mock.patch(
            "aphrody.gemini_scraper.GeminiScraper.scrape",
            return_value=mock_scraped,
        ),
        mock.patch("aphrody.vertex.GeminiVertex", return_value=mock_gv),
        mock.patch("aphrody.cli.web._emit") as mock_emit,
    ):
        WebCommands().auto_upgrade()

        mock_emit.assert_called_once()
        res = mock_emit.call_args[0][0]
        assert res["success"] is True
        assert res["upgraded"] is False
        assert len(res["replacements_applied"]) == 0


def test_auto_upgrade_llm_success(httpx_mock, restore_gemini_web) -> None:
    """Test when the LLM identifies and generates the replacements block successfully."""
    mock_scraped = {
        "script_urls": ["/js/1.js"],
        "rpc_mappings": {
            "MaZiqcLLM": "BardFrontendService.ListConversations",
        },
    }

    # Mock the LLM returning a valid replacement JSON response
    mock_gv = mock.MagicMock()
    mock_gv.generate.return_value = """
    ```json
    {
      "replacements": [
        {
          "file": "aphrody/aphrody/gemini_web.py",
          "target": "MaZiqc",
          "replacement": "MaZiqcLLM"
        }
      ]
    }
    ```
    """

    with (
        mock.patch(
            "aphrody.gemini_scraper.GeminiScraper.scrape",
            return_value=mock_scraped,
        ),
        mock.patch("aphrody.vertex.GeminiVertex", return_value=mock_gv),
        mock.patch("subprocess.run") as mock_run,
        mock.patch("aphrody.cli.web._emit") as mock_emit,
    ):
        mock_run.return_value = subprocess.CompletedProcess(
            args=[], returncode=0, stdout=b"", stderr=b""
        )

        WebCommands().auto_upgrade()

        # Check modifications applied
        updated_code = restore_gemini_web.read_text(encoding="utf-8")
        assert "MaZiqcLLM" in updated_code

        mock_emit.assert_called_once()
        res = mock_emit.call_args[0][0]
        assert res["success"] is True
        assert res["upgraded"] is True
        assert len(res["replacements_applied"]) == 1


def test_auto_upgrade_fallback_success(httpx_mock, restore_gemini_web) -> None:
    """Test fallback upgrade path when LLM fails or is bypassed."""
    mock_scraped = {
        "script_urls": ["/js/1.js"],
        "css_classes": [],
        "css_variables": [],
        "rpc_services": [],
        "rpc_methods": [],
        "rpc_mappings": {
            "MaZiqcFallback": "BardFrontendService.ListConversations",
            "GzXR5eFallback": "BardFrontendService.DeleteConversation",
        },
        "boq_hashes": [],
        "interactive_roles": [],
        "aria_attributes": [],
        "models": [],
        "feature_flags": [],
        "buttons": [],
    }

    # Verify original file contains old hashes
    original_code = restore_gemini_web.read_text(encoding="utf-8")
    assert "MaZiqc" in original_code
    assert "GzXR5e" in original_code

    # Mock the LLM call to fail to trigger fallback path
    mock_gv = mock.MagicMock()
    mock_gv.generate.side_effect = Exception("Vertex region error")

    with (
        mock.patch(
            "aphrody.gemini_scraper.GeminiScraper.scrape",
            return_value=mock_scraped,
        ),
        mock.patch("aphrody.vertex.GeminiVertex", return_value=mock_gv),
        mock.patch("subprocess.run") as mock_run,
        mock.patch("aphrody.cli.web._emit") as mock_emit,
    ):
        mock_run.return_value = subprocess.CompletedProcess(
            args=[], returncode=0, stdout=b"", stderr=b""
        )

        WebCommands().auto_upgrade()

        # Check modifications applied
        updated_code = restore_gemini_web.read_text(encoding="utf-8")
        assert "MaZiqcFallback" in updated_code
        assert "GzXR5eFallback" in updated_code

        mock_emit.assert_called_once()
        res = mock_emit.call_args[0][0]
        assert res["success"] is True
        assert res["upgraded"] is True
        assert len(res["replacements_applied"]) == 2


def test_auto_upgrade_rollback_on_failure(
    httpx_mock, restore_gemini_web
) -> None:
    """Test rollback occurs when validations fail."""
    mock_scraped = {
        "script_urls": ["/js/1.js"],
        "css_classes": [],
        "css_variables": [],
        "rpc_services": [],
        "rpc_methods": [],
        "rpc_mappings": {
            "MaZiqcNew": "BardFrontendService.ListConversations",
        },
        "boq_hashes": [],
        "interactive_roles": [],
        "aria_attributes": [],
        "models": [],
        "feature_flags": [],
        "buttons": [],
    }

    original_code = restore_gemini_web.read_text(encoding="utf-8")

    # Mock LLM to fail to trigger fallback
    mock_gv = mock.MagicMock()
    mock_gv.generate.side_effect = Exception("Vertex Region Error")

    with (
        mock.patch(
            "aphrody.gemini_scraper.GeminiScraper.scrape",
            return_value=mock_scraped,
        ),
        mock.patch("aphrody.vertex.GeminiVertex", return_value=mock_gv),
        mock.patch("subprocess.run") as mock_run,
        mock.patch("aphrody.cli.web._emit") as mock_emit,
    ):
        # Mock validations to fail
        mock_run.return_value = subprocess.CompletedProcess(
            args=[], returncode=1, stdout=b"Test Failure", stderr=b""
        )

        with pytest.raises(
            RuntimeError, match="Autonomous upgrade failed validation"
        ):
            WebCommands().auto_upgrade()

        # Check code was rolled back
        current_code = restore_gemini_web.read_text(encoding="utf-8")
        assert current_code == original_code
        mock_emit.assert_not_called()
