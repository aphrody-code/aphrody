# SPDX-License-Identifier: Apache-2.0
"""Tests for the web CLI command in :mod:`aphrody.cli`."""

from __future__ import annotations

import unittest
from unittest import mock

from aphrody.cli import Aphrody


class TestCliWeb(unittest.TestCase):
    @mock.patch("aphrody.gemini_web.GeminiWebClient")
    def test_cli_web_single_shot(self, mock_client_class) -> None:
        mock_client = mock.MagicMock()
        mock_client_class.return_value.__enter__.return_value = mock_client
        mock_client.generate.return_value = "Hello"

        cli = Aphrody()
        with mock.patch("aphrody.cli.web._emit") as mock_emit:
            cli.web()(prompt="hi")
            mock_client.generate.assert_called_once_with(
                "hi", keep_context=False
            )
            mock_emit.assert_called_once_with("Hello")

    @mock.patch("aphrody.gemini_web.GeminiWebClient")
    def test_cli_web_thread(self, mock_client_class) -> None:
        mock_client = mock.MagicMock()
        mock_client_class.return_value.__enter__.return_value = mock_client
        mock_client.generate.return_value = "Hello context"
        mock_client.conversation = ("c1", "r1", "rc1")
        mock_client.last_title = "Conversation Title"

        cli = Aphrody()
        with mock.patch("aphrody.cli.web._emit") as mock_emit:
            cli.web()(prompt="hi", thread=True)
            mock_client.generate.assert_called_once_with(
                "hi", keep_context=True
            )
            mock_emit.assert_called_once_with(
                {
                    "reply": "Hello context",
                    "title": "Conversation Title",
                    "conversation": {"cid": "c1", "rid": "r1", "rcid": "rc1"},
                }
            )

    @mock.patch("aphrody.gemini_web.GeminiWebClient")
    def test_cli_web_resume_call(self, mock_client_class) -> None:
        mock_client = mock.MagicMock()
        mock_client_class.return_value.__enter__.return_value = mock_client
        mock_client.generate.return_value = "Hello"

        cli = Aphrody()
        with mock.patch("aphrody.cli.web._emit"):
            cli.web()(prompt="hi", cid="c1", rid="r1", rcid="rc1")
            mock_client.resume.assert_called_once_with("c1", "r1", "rc1")
            mock_client.generate.assert_called_once_with(
                "hi", keep_context=False
            )

    @mock.patch("aphrody.gemini_web.GeminiWebClient")
    def test_cli_web_conversations(self, mock_client_class) -> None:
        mock_client = mock.MagicMock()
        mock_client_class.return_value.__enter__.return_value = mock_client
        mock_client.list_conversations.return_value = [
            {"cid": "c1", "title": "Chat 1", "updated_at": 12345}
        ]

        cli = Aphrody()
        with mock.patch("aphrody.cli.web._emit") as mock_emit:
            cli.web().conversations()
            mock_client.list_conversations.assert_called_once()
            mock_emit.assert_called_once_with(
                [{"cid": "c1", "title": "Chat 1", "updated_at": 12345}]
            )

    @mock.patch("aphrody.gemini_web.GeminiWebClient")
    def test_cli_web_resume_cmd(self, mock_client_class) -> None:
        mock_client = mock.MagicMock()
        mock_client_class.return_value.__enter__.return_value = mock_client
        mock_client.generate.return_value = "Hello Resumed"

        cli = Aphrody()
        with mock.patch("aphrody.cli.web._emit") as mock_emit:
            cli.web().resume("c_1", prompt="hello")
            mock_client.resume.assert_called_once_with("c_1", None, None)
            mock_client.generate.assert_called_once_with(
                "hello", keep_context=False
            )
            mock_emit.assert_called_once_with("Hello Resumed")

    @mock.patch("aphrody.gemini_web.GeminiWebClient")
    def test_cli_web_delete(self, mock_client_class) -> None:
        mock_client = mock.MagicMock()
        mock_client_class.return_value.__enter__.return_value = mock_client

        cli = Aphrody()
        with mock.patch("aphrody.cli.web._emit") as mock_emit:
            cli.web().delete("c_1")
            mock_client.delete_conversation.assert_called_once_with("c_1")
            mock_emit.assert_called_once_with({"deleted": "c_1"})

    @mock.patch("aphrody.gemini_scraper.GeminiScraper")
    def test_cli_web_scrape(self, mock_scraper_class) -> None:
        mock_scraper = mock.MagicMock()
        mock_scraper_class.return_value = mock_scraper
        mock_scraper.scrape.return_value = {
            "script_urls": ["url1"],
            "css_classes": ["class1"],
            "css_variables": ["--var1"],
            "rpc_services": ["service1"],
            "rpc_methods": ["method1"],
            "rpc_mappings": {"hash1": "method1"},
            "boq_hashes": ["hash1"],
            "interactive_roles": ["role1"],
            "aria_attributes": ["attr1"],
            "models": ["model1"],
            "feature_flags": ["flag1"],
            "buttons": [{"tag": "button", "text": "text1"}],
        }
        mock_scraper.format_markdown_report.return_value = "# Report"

        cli = Aphrody()
        with mock.patch("aphrody.cli.web._emit") as mock_emit:
            cli.web().scrape()
            mock_scraper.scrape.assert_called_once()
            mock_emit.assert_called_once()
