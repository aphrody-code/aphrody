# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Keyless Google API commands for the aphrody CLI."""

from __future__ import annotations

from pathlib import Path

from aphrody.cli.utils import _emit


class GoogleCommands:
    """``aphrody google <action>`` — keyless and anonymous Google API suite.

    Access public Google services (DNS resolution, Book search, Translation,
    Query suggestions, public calendars, and public Google Drive downloads)
    without credentials or API keys.
    """

    def dns(self, name: str, type: str = "A") -> None:
        """Query Google Public DNS-over-HTTPS.

        Args:
            name: Domain name (e.g. "google.com").
            type: Record type (e.g. "A", "AAAA", "MX", "TXT").
        """
        from aphrody.google_keyless import KeylessGoogleClient

        with KeylessGoogleClient() as client:
            _emit(client.resolve_dns(name, type_=type))

    def books(self, query: str, max_results: int = 10, start: int = 0) -> None:
        """Search the public Google Books catalog.

        Args:
            query: Search query (e.g., "isbn:9780545010221" or "quantum physics").
            max_results: Maximum results to return (default: 10).
            start: Start index position in results (default: 0).
        """
        from aphrody.google_keyless import KeylessGoogleClient

        with KeylessGoogleClient() as client:
            _emit(
                client.search_books(
                    query, max_results=max_results, start_index=start
                )
            )

    def book(self, volume_id: str) -> None:
        """Retrieve a specific Google Books volume by its ID.

        Args:
            volume_id: Google Books volume ID.
        """
        from aphrody.google_keyless import KeylessGoogleClient

        with KeylessGoogleClient() as client:
            _emit(client.get_book(volume_id))

    def translate(
        self, text: str, target: str = "en", source: str = "auto"
    ) -> None:
        """Translate text using the keyless Google Translate API.

        Args:
            text: Text to translate.
            target: Target language code (default: "en").
            source: Source language code (default: "auto").
        """
        from aphrody.google_keyless import KeylessGoogleClient

        with KeylessGoogleClient() as client:
            res = client.translate(text, target_lang=target, source_lang=source)
            _emit(res)

    def suggest(self, query: str, client: str = "chrome") -> None:
        """Get query suggestions using Google Autocomplete.

        Args:
            query: Partial query search string.
            client: Autocomplete client ID (default: "chrome", or "firefox").
        """
        from aphrody.google_keyless import KeylessGoogleClient

        with KeylessGoogleClient() as c:
            _emit(c.autocomplete(query, client=client))

    def calendar(self, calendar_id: str) -> None:
        """Fetch and parse events from a public Google iCalendar feed.

        Args:
            calendar_id: Public calendar ID (e.g. "en.usa#holiday@group.v.calendar.google.com").
        """
        from aphrody.google_keyless import KeylessGoogleClient

        with KeylessGoogleClient() as client:
            _emit(client.get_public_calendar_events(calendar_id))

    def sheet(self, spreadsheet_id: str, gid: str | None = None) -> None:
        """Export a public Google Sheet to CSV.

        Args:
            spreadsheet_id: Google Spreadsheet ID.
            gid: Optional sheet grid ID.
        """
        from aphrody.google_keyless import KeylessGoogleClient

        with KeylessGoogleClient() as client:
            print(client.export_public_sheet_to_csv(spreadsheet_id, gid=gid))

    def doc(self, document_id: str) -> None:
        """Export a public Google Doc to plain text.

        Args:
            document_id: Google Document ID.
        """
        from aphrody.google_keyless import KeylessGoogleClient

        with KeylessGoogleClient() as client:
            print(client.export_public_doc_to_text(document_id))

    def download(self, file_id: str, out: str) -> None:
        """Download a public Google Drive file by ID.

        Args:
            file_id: Google Drive file ID.
            out: Destination output file path.
        """
        from aphrody.google_keyless import KeylessGoogleClient

        with KeylessGoogleClient() as client:
            data = client.download_public_drive_file(file_id)
            Path(out).write_bytes(data)
            _emit({"saved": out, "bytes": len(data)})
