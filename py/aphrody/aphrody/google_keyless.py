# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Keyless, anonymous, and public Google API integration module.

This module provides direct, credential-free access to public Google APIs and
endpoints, including Books search, Public DNS, Translate (gtx client), Search
Suggestions (Autocomplete), public iCalendar feeds, and public Drive/Docs/Sheets
file extraction.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    import httpx

from aphrody.errors import ApiError


class KeylessGoogleClient:
    """Client wrapper for keyless Google API features.

    Exposes public endpoints that do not require API keys or developer
    credentials.
    """

    def __init__(self, http: httpx.Client | None = None) -> None:
        """Initialize the keyless client.

        Args:
            http: An optional custom httpx Client. If None, a default one is
                created.
        """
        if http is None:
            import httpx

            self._http = httpx.Client(timeout=60.0)
        else:
            self._http = http
        self._owns_http = http is None

    def close(self) -> None:
        """Close the underlying HTTP client if owned by this instance."""
        if self._owns_http:
            self._http.close()

    def __enter__(self) -> KeylessGoogleClient:
        return self

    def __exit__(self, *exc: object) -> None:
        self.close()

    def _request(
        self,
        method: str,
        url: str,
        *,
        params: dict[str, Any] | None = None,
        headers: dict[str, str] | None = None,
    ) -> httpx.Response:
        """Send an HTTP request and raise ApiError on non-2xx response."""
        response = self._http.request(
            method, url, params=params, headers=headers
        )
        if not response.is_success:
            raise ApiError(response.status_code, response.text, url=url)
        return response

    def resolve_dns(self, name: str, type_: str = "A") -> dict[str, Any]:
        """Query Google Public DNS-over-HTTPS.

        Args:
            name: The domain name to resolve (e.g., "example.com").
            type_: The query type (e.g., "A", "AAAA", "MX", "TXT").

        Returns:
            The decoded JSON response from the DNS endpoint.
        """
        url = "https://dns.google/resolve"
        response = self._request(
            "GET", url, params={"name": name, "type": type_}
        )
        return response.json()

    def search_books(
        self, query: str, max_results: int = 10, start_index: int = 0
    ) -> dict[str, Any]:
        """Search the public Google Books catalog.

        Args:
            query: The search query (e.g., "isbn:9780545010221" or "quantum physics").
            max_results: Maximum number of results to return (default: 10).
            start_index: The position in the collection at which to start the
                list of results.

        Returns:
            The decoded JSON response from the Books volume search endpoint.
        """
        url = "https://www.googleapis.com/books/v1/volumes"
        params = {
            "q": query,
            "maxResults": max_results,
            "startIndex": start_index,
        }
        response = self._request("GET", url, params=params)
        return response.json()

    def get_book(self, volume_id: str) -> dict[str, Any]:
        """Retrieve a specific Google Books volume by its ID.

        Args:
            volume_id: The Google Books volume ID.

        Returns:
            The decoded JSON response containing the volume metadata.
        """
        url = f"https://www.googleapis.com/books/v1/volumes/{volume_id}"
        response = self._request("GET", url)
        return response.json()

    def translate(
        self, text: str, target_lang: str = "en", source_lang: str = "auto"
    ) -> str:
        """Translate text using the keyless Google Translate API.

        Args:
            text: The text to translate.
            target_lang: Target language code (default: "en").
            source_lang: Source language code (default: "auto").

        Returns:
            The translated text.
        """
        url = "https://translate.googleapis.com/translate_a/single"
        params = {
            "client": "gtx",
            "sl": source_lang,
            "tl": target_lang,
            "dt": "t",
            "q": text,
        }
        response = self._request("GET", url, params=params)
        data = response.json()
        translations = []
        if data and isinstance(data, list) and data[0]:
            for item in data[0]:
                if item and isinstance(item, list) and len(item) > 0:
                    translations.append(item[0])
        return "".join(translations)

    def autocomplete(self, query: str, client: str = "chrome") -> list[str]:
        """Get query suggestions using Google Autocomplete / Suggest Queries.

        Args:
            query: The partial query search string.
            client: Autocomplete client ID (default: "chrome", or "firefox").

        Returns:
            A list of search query suggestions.
        """
        url = "https://suggestqueries.google.com/complete/search"
        params = {"client": client, "q": query}
        response = self._request("GET", url, params=params)
        data = response.json()
        if len(data) > 1 and isinstance(data[1], list):
            return [str(item) for item in data[1]]
        return []

    def get_public_calendar_events(
        self, calendar_id: str
    ) -> list[dict[str, Any]]:
        """Fetch and parse events from a public Google iCalendar feed.

        Args:
            calendar_id: The ID of the public calendar (e.g. standard email
                address or public calendar ID).

        Returns:
            A list of event dictionaries containing basic details (e.g.,
            summary, description, dtstart, dtend, location, uid).
        """
        url = f"https://calendar.google.com/calendar/ical/{calendar_id}/public/basic.ics"
        response = self._request("GET", url)
        raw_text = response.text

        # Unfold lines according to RFC 5545 (line folded by starting with space/tab)
        lines = []
        for line in raw_text.splitlines():
            if line.startswith((" ", "\t")):
                if lines:
                    lines[-1] += line[1:]
            else:
                lines.append(line)

        events: list[dict[str, Any]] = []
        current_event: dict[str, Any] | None = None

        for line in lines:
            if line.startswith("BEGIN:VEVENT"):
                current_event = {}
            elif line.startswith("END:VEVENT"):
                if current_event is not None:
                    events.append(current_event)
                    current_event = None
            elif current_event is not None:
                if ":" in line:
                    key_part, value = line.split(":", 1)
                    key = key_part.split(";")[0].upper()
                    # Basic unescaping of common ics characters
                    value = (
                        value.replace("\\,", ",")
                        .replace("\\;", ";")
                        .replace("\\n", "\n")
                        .replace("\\N", "\n")
                    )
                    current_event[key.lower()] = value

        return events

    def export_public_sheet_to_csv(
        self, spreadsheet_id: str, gid: str | None = None
    ) -> str:
        """Export a public Google Sheet (viewable by anyone with link) to CSV.

        Args:
            spreadsheet_id: The Google Spreadsheet ID.
            gid: Optional sheet grid ID.

        Returns:
            The CSV content of the spreadsheet as a string.
        """
        url = f"https://docs.google.com/spreadsheets/d/{spreadsheet_id}/export"
        params = {"format": "csv"}
        if gid is not None:
            params["gid"] = gid
        response = self._request("GET", url, params=params)
        return response.text

    def export_public_doc_to_text(self, document_id: str) -> str:
        """Export a public Google Doc (viewable by anyone with link) to plain text.

        Args:
            document_id: The Google Document ID.

        Returns:
            The plain text content of the document.
        """
        url = f"https://docs.google.com/document/d/{document_id}/export"
        params = {"format": "txt"}
        response = self._request("GET", url, params=params)
        return response.text

    def download_public_drive_file(self, file_id: str) -> bytes:
        """Download a public Google Drive file by ID.

        Handles the Google Drive virus scan warning redirect for large files.

        Args:
            file_id: The Google Drive file ID.

        Returns:
            The raw bytes of the downloaded file.
        """
        url = "https://docs.google.com/uc"
        params = {"export": "download", "id": file_id}
        # First request to get the file or the warning page
        response = self._http.get(url, params=params)

        confirm_token = None
        for cookie_name, cookie_value in response.cookies.items():
            if cookie_name.startswith("download_warning"):
                confirm_token = cookie_value
                break

        if not confirm_token:
            import re

            match = re.search(r"confirm=([0-9A-Za-z_]+)", response.text)
            if match:
                confirm_token = match.group(1)

        if confirm_token:
            params["confirm"] = confirm_token
            response = self._request("GET", url, params=params)
        elif not response.is_success:
            raise ApiError(response.status_code, response.text, url=url)

        return response.content


class AsyncKeylessGoogleClient:
    """Async client wrapper for keyless Google API features.

    Exposes public endpoints that do not require API keys or developer
    credentials.
    """

    def __init__(self, http: httpx.AsyncClient | None = None) -> None:
        """Initialize the async keyless client.

        Args:
            http: An optional custom httpx AsyncClient. If None, a default one is
                created.
        """
        if http is None:
            import httpx

            self._http = httpx.AsyncClient(timeout=60.0)
        else:
            self._http = http
        self._owns_http = http is None

    async def close(self) -> None:
        """Close the underlying HTTP client if owned by this instance."""
        if self._owns_http:
            await self._http.aclose()

    async def __aenter__(self) -> AsyncKeylessGoogleClient:
        return self

    async def __aexit__(self, *exc: object) -> None:
        await self.close()

    async def _request(
        self,
        method: str,
        url: str,
        *,
        params: dict[str, Any] | None = None,
        headers: dict[str, str] | None = None,
    ) -> httpx.Response:
        """Send an HTTP request asynchronously and raise ApiError on non-2xx response."""
        response = await self._http.request(
            method, url, params=params, headers=headers
        )
        if not response.is_success:
            raise ApiError(response.status_code, response.text, url=url)
        return response

    async def resolve_dns(self, name: str, type_: str = "A") -> dict[str, Any]:
        """Query Google Public DNS-over-HTTPS.

        Args:
            name: The domain name to resolve (e.g., "example.com").
            type_: The query type (e.g., "A", "AAAA", "MX", "TXT").

        Returns:
            The decoded JSON response from the DNS endpoint.
        """
        url = "https://dns.google/resolve"
        response = await self._request(
            "GET", url, params={"name": name, "type": type_}
        )
        return response.json()

    async def search_books(
        self, query: str, max_results: int = 10, start_index: int = 0
    ) -> dict[str, Any]:
        """Search the public Google Books catalog.

        Args:
            query: The search query (e.g., "isbn:9780545010221" or "quantum physics").
            max_results: Maximum number of results to return (default: 10).
            start_index: The position in the collection at which to start the
                list of results.

        Returns:
            The decoded JSON response from the Books volume search endpoint.
        """
        url = "https://www.googleapis.com/books/v1/volumes"
        params = {
            "q": query,
            "maxResults": max_results,
            "startIndex": start_index,
        }
        response = await self._request("GET", url, params=params)
        return response.json()

    async def get_book(self, volume_id: str) -> dict[str, Any]:
        """Retrieve a specific Google Books volume by its ID.

        Args:
            volume_id: The Google Books volume ID.

        Returns:
            The decoded JSON response containing the volume metadata.
        """
        url = f"https://www.googleapis.com/books/v1/volumes/{volume_id}"
        response = await self._request("GET", url)
        return response.json()

    async def translate(
        self, text: str, target_lang: str = "en", source_lang: str = "auto"
    ) -> str:
        """Translate text using the keyless Google Translate API.

        Args:
            text: The text to translate.
            target_lang: Target language code (default: "en").
            source_lang: Source language code (default: "auto").

        Returns:
            The translated text.
        """
        url = "https://translate.googleapis.com/translate_a/single"
        params = {
            "client": "gtx",
            "sl": source_lang,
            "tl": target_lang,
            "dt": "t",
            "q": text,
        }
        response = await self._request("GET", url, params=params)
        data = response.json()
        translations = []
        if data and isinstance(data, list) and data[0]:
            for item in data[0]:
                if item and isinstance(item, list) and len(item) > 0:
                    translations.append(item[0])
        return "".join(translations)

    async def autocomplete(
        self, query: str, client: str = "chrome"
    ) -> list[str]:
        """Get query suggestions using Google Autocomplete / Suggest Queries.

        Args:
            query: The partial query search string.
            client: Autocomplete client ID (default: "chrome", or "firefox").

        Returns:
            A list of search query suggestions.
        """
        url = "https://suggestqueries.google.com/complete/search"
        params = {"client": client, "q": query}
        response = await self._request("GET", url, params=params)
        data = response.json()
        if len(data) > 1 and isinstance(data[1], list):
            return [str(item) for item in data[1]]
        return []

    async def get_public_calendar_events(
        self, calendar_id: str
    ) -> list[dict[str, Any]]:
        """Fetch and parse events from a public Google iCalendar feed.

        Args:
            calendar_id: The ID of the public calendar (e.g. standard email
                address or public calendar ID).

        Returns:
            A list of event dictionaries containing basic details (e.g.,
            summary, description, dtstart, dtend, location, uid).
        """
        url = f"https://calendar.google.com/calendar/ical/{calendar_id}/public/basic.ics"
        response = await self._request("GET", url)
        raw_text = response.text

        # Unfold lines according to RFC 5545 (line folded by starting with space/tab)
        lines = []
        for line in raw_text.splitlines():
            if line.startswith((" ", "\t")):
                if lines:
                    lines[-1] += line[1:]
            else:
                lines.append(line)

        events: list[dict[str, Any]] = []
        current_event: dict[str, Any] | None = None

        for line in lines:
            if line.startswith("BEGIN:VEVENT"):
                current_event = {}
            elif line.startswith("END:VEVENT"):
                if current_event is not None:
                    events.append(current_event)
                    current_event = None
            elif current_event is not None:
                if ":" in line:
                    key_part, value = line.split(":", 1)
                    key = key_part.split(";")[0].upper()
                    # Basic unescaping of common ics characters
                    value = (
                        value.replace("\\,", ",")
                        .replace("\\;", ";")
                        .replace("\\n", "\n")
                        .replace("\\N", "\n")
                    )
                    current_event[key.lower()] = value

        return events

    async def export_public_sheet_to_csv(
        self, spreadsheet_id: str, gid: str | None = None
    ) -> str:
        """Export a public Google Sheet (viewable by anyone with link) to CSV.

        Args:
            spreadsheet_id: The Google Spreadsheet ID.
            gid: Optional sheet grid ID.

        Returns:
            The CSV content of the spreadsheet as a string.
        """
        url = f"https://docs.google.com/spreadsheets/d/{spreadsheet_id}/export"
        params = {"format": "csv"}
        if gid is not None:
            params["gid"] = gid
        response = await self._request("GET", url, params=params)
        return response.text

    async def export_public_doc_to_text(self, document_id: str) -> str:
        """Export a public Google Doc (viewable by anyone with link) to plain text.

        Args:
            document_id: The Google Document ID.

        Returns:
            The plain text content of the document.
        """
        url = f"https://docs.google.com/document/d/{document_id}/export"
        params = {"format": "txt"}
        response = await self._request("GET", url, params=params)
        return response.text

    async def download_public_drive_file(self, file_id: str) -> bytes:
        """Download a public Google Drive file by ID.

        Handles the Google Drive virus scan warning redirect for large files.

        Args:
            file_id: The Google Drive file ID.

        Returns:
            The raw bytes of the downloaded file.
        """
        url = "https://docs.google.com/uc"
        params = {"export": "download", "id": file_id}
        # First request to get the file or the warning page
        response = await self._http.get(url, params=params)

        confirm_token = None
        for cookie_name, cookie_value in response.cookies.items():
            if cookie_name.startswith("download_warning"):
                confirm_token = cookie_value
                break

        if not confirm_token:
            import re

            match = re.search(r"confirm=([0-9A-Za-z_]+)", response.text)
            if match:
                confirm_token = match.group(1)

        if confirm_token:
            params["confirm"] = confirm_token
            response = await self._request("GET", url, params=params)
        elif not response.is_success:
            raise ApiError(response.status_code, response.text, url=url)

        return response.content
