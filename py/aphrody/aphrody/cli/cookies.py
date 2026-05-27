# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Cookie management command group for the aphrody CLI."""

from __future__ import annotations

from pathlib import Path

from aphrody.auth import cookies as cookies_store
from aphrody.cli.utils import _emit


class CookieCommands:
    """``aphrody cookies <action>`` — manage the keyless Google cookie jar.

    Values are never printed: :meth:`status` reports metadata only.
    """

    def status(self) -> None:
        """Show the stored cookie jar metadata (names/domains, never values)."""
        _emit(cookies_store.status())

    def load(self, file: str) -> None:
        """Import cookies from a Cookie-Editor JSON export *file*.

        Args:
            file: Path to a Cookie-Editor (or compatible) JSON export.
        """
        text = Path(file).read_text(encoding="utf-8")
        jar = cookies_store.import_cookie_editor(text)
        _emit({"imported": len(jar), **cookies_store.status(jar)})

    def extract(self, domain: str = "google.com") -> None:
        """Extract cookies straight from local Chrome (best effort).

        Args:
            domain: Cookie domain filter (default ``"google.com"``).
        """
        jar = cookies_store.extract_from_chrome(domain)
        _emit({"extracted": len(jar), **cookies_store.status(jar)})

    def export(self, format: str = "csv") -> None:
        """Export cookies in CSV format for legacy compatibility.

        Args:
            format: Export format (default ``"csv"``).

        Raises:
            AphrodyError: If the format is unsupported or loading fails.
        """
        if format.lower() != "csv":
            from aphrody.errors import AphrodyError

            raise AphrodyError(
                f"Unsupported format {format!r}. Only 'csv' is supported."
            )

        import csv
        import io

        jar = cookies_store.load()
        output = io.StringIO()
        writer = csv.writer(output, lineterminator="\n")
        writer.writerow(
            ["name", "value", "domain", "path", "expiry", "secure", "http_only"]
        )
        for cookie in jar.cookies:
            writer.writerow(
                [
                    cookie.name,
                    cookie.value,
                    cookie.domain,
                    cookie.path,
                    cookie.expiry if cookie.expiry is not None else "",
                    cookie.secure,
                    cookie.http_only,
                ]
            )
        _emit(output.getvalue().rstrip("\r\n"))
