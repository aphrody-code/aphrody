# SPDX-License-Identifier: Apache-2.0
"""Tests for the keyless Google CLI commands in :mod:`aphrody.cli.google`."""

from __future__ import annotations

import unittest
from unittest import mock

from aphrody.cli import Aphrody


class TestCliGoogle(unittest.TestCase):
    @mock.patch("aphrody.google_keyless.KeylessGoogleClient")
    def test_cli_google_dns(self, mock_client_class) -> None:
        mock_client = mock.MagicMock()
        mock_client_class.return_value.__enter__.return_value = mock_client
        mock_client.resolve_dns.return_value = {"status": "ok"}

        cli = Aphrody()
        with mock.patch("aphrody.cli.google._emit") as mock_emit:
            cli.google().dns("example.com", type="MX")
            mock_client.resolve_dns.assert_called_once_with(
                "example.com", type_="MX"
            )
            mock_emit.assert_called_once_with({"status": "ok"})

    @mock.patch("aphrody.google_keyless.KeylessGoogleClient")
    def test_cli_google_translate(self, mock_client_class) -> None:
        mock_client = mock.MagicMock()
        mock_client_class.return_value.__enter__.return_value = mock_client
        mock_client.translate.return_value = "Bonjour"

        cli = Aphrody()
        with mock.patch("aphrody.cli.google._emit") as mock_emit:
            cli.google().translate("Hello", target="fr")
            mock_client.translate.assert_called_once_with(
                "Hello", target_lang="fr", source_lang="auto"
            )
            mock_emit.assert_called_once_with("Bonjour")
