# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Tests for the cookie CLI commands in :mod:`aphrody.cli.cookies`."""

from __future__ import annotations

import unittest
from unittest import mock

from aphrody.auth.cookies import Cookie, CookieJar, CookieStoreError
from aphrody.cli import Aphrody
from aphrody.errors import AphrodyError


class TestCliCookies(unittest.TestCase):
    @mock.patch("aphrody.cli.cookies.cookies_store.load")
    def test_cli_cookies_export_csv(self, mock_load) -> None:
        mock_load.return_value = CookieJar(
            [
                Cookie(
                    name="session_id",
                    value="xyz123",
                    domain=".google.com",
                    path="/",
                    expiry=1700000000.0,
                    secure=True,
                    http_only=True,
                ),
                Cookie(
                    name="pref",
                    value="dark",
                    domain=".google.com",
                    path="/pref",
                    expiry=None,
                    secure=False,
                    http_only=False,
                ),
            ]
        )

        cli = Aphrody()
        with mock.patch("aphrody.cli.cookies._emit") as mock_emit:
            cli.cookies().export(format="csv")
            mock_load.assert_called_once()

            # The output should be a single string with CSV contents.
            mock_emit.assert_called_once()
            emitted_value = mock_emit.call_args[0][0]

            expected_csv = (
                "name,value,domain,path,expiry,secure,http_only\n"
                "session_id,xyz123,.google.com,/,1700000000.0,True,True\n"
                "pref,dark,.google.com,/pref,,False,False"
            )
            self.assertEqual(emitted_value, expected_csv)

    def test_cli_cookies_export_unsupported_format(self) -> None:
        cli = Aphrody()
        with self.assertRaises(AphrodyError) as ctx:
            cli.cookies().export(format="json")
        self.assertIn("Unsupported format 'json'", str(ctx.exception))

    @mock.patch("aphrody.cli.cookies.cookies_store.load")
    def test_cli_cookies_export_load_failure(self, mock_load) -> None:
        mock_load.side_effect = CookieStoreError("cookie store not found")
        cli = Aphrody()
        with self.assertRaises(CookieStoreError) as ctx:
            cli.cookies().export(format="csv")
        self.assertIn("cookie store not found", str(ctx.exception))
