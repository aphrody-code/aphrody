# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Tests for KeylessGoogleClient features."""

from __future__ import annotations

import unittest

import pytest
from aphrody.errors import ApiError
from aphrody.google_keyless import AsyncKeylessGoogleClient, KeylessGoogleClient


def test_context_manager() -> None:
    """Test that client supports context manager protocol."""
    with KeylessGoogleClient() as client:
        assert client is not None


def test_resolve_dns(httpx_mock) -> None:
    """Test Google Public DNS resolution."""
    httpx_mock.add_response(
        url="https://dns.google/resolve?name=example.com&type=A",
        json={
            "Status": 0,
            "Answer": [
                {"name": "example.com.", "type": 1, "data": "93.184.215.14"}
            ],
        },
    )
    with KeylessGoogleClient() as client:
        res = client.resolve_dns("example.com", "A")
        assert res["Status"] == 0
        assert res["Answer"][0]["data"] == "93.184.215.14"


def test_search_books(httpx_mock) -> None:
    """Test Google Books catalog searching."""
    httpx_mock.add_response(
        url="https://www.googleapis.com/books/v1/volumes?q=test&maxResults=5&startIndex=0",
        json={
            "kind": "books#volumes",
            "items": [{"id": "vol1", "volumeInfo": {"title": "Test Book"}}],
        },
    )
    with KeylessGoogleClient() as client:
        res = client.search_books("test", max_results=5)
        assert res["kind"] == "books#volumes"
        assert res["items"][0]["volumeInfo"]["title"] == "Test Book"


def test_get_book(httpx_mock) -> None:
    """Test retrieving a specific Google Books volume."""
    httpx_mock.add_response(
        url="https://www.googleapis.com/books/v1/volumes/vol1",
        json={"id": "vol1", "volumeInfo": {"title": "Specific Test Book"}},
    )
    with KeylessGoogleClient() as client:
        res = client.get_book("vol1")
        assert res["id"] == "vol1"
        assert res["volumeInfo"]["title"] == "Specific Test Book"


def test_translate(httpx_mock) -> None:
    """Test translating text using the keyless Translate endpoint."""
    # The translate response format is a nested list where the translation is in the first element.
    mock_response = [
        [
            ["Bonjour", "Hello", None, None, 1],
            [" le monde", " world", None, None, 1],
        ],
        None,
        "en",
    ]
    httpx_mock.add_response(
        url="https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=fr&dt=t&q=Hello+world",
        json=mock_response,
    )
    with KeylessGoogleClient() as client:
        translated = client.translate("Hello world", target_lang="fr")
        assert translated == "Bonjour le monde"


def test_autocomplete(httpx_mock) -> None:
    """Test autocomplete query suggestions."""
    httpx_mock.add_response(
        url="https://suggestqueries.google.com/complete/search?client=chrome&q=python",
        json=[
            "python",
            ["python tutorial", "python download", "python documentation"],
        ],
    )
    with KeylessGoogleClient() as client:
        suggestions = client.autocomplete("python")
        assert suggestions == [
            "python tutorial",
            "python download",
            "python documentation",
        ]


def test_get_public_calendar_events(httpx_mock) -> None:
    """Test fetching and parsing public Google iCalendar feeds."""
    mock_ics = (
        "BEGIN:VCALENDAR\r\n"
        "VERSION:2.0\r\n"
        "PRODID:-//Google Inc//Google Calendar 70.9054//EN\r\n"
        "BEGIN:VEVENT\r\n"
        "UID:event123@google.com\r\n"
        "DTSTART:20260523T080000Z\r\n"
        "DTEND:20260523T090000Z\r\n"
        "SUMMARY:Meeting with\r\n"
        "  Team\r\n"
        "DESCRIPTION:Project status update\\, discussing\\nkey deliverables.\r\n"
        "LOCATION:Conference Room A\r\n"
        "END:VEVENT\r\n"
        "END:VCALENDAR\r\n"
    )
    httpx_mock.add_response(
        url="https://calendar.google.com/calendar/ical/test_cal/public/basic.ics",
        text=mock_ics,
    )
    with KeylessGoogleClient() as client:
        events = client.get_public_calendar_events("test_cal")
        assert len(events) == 1
        event = events[0]
        assert event["uid"] == "event123@google.com"
        assert event["dtstart"] == "20260523T080000Z"
        assert event["dtend"] == "20260523T090000Z"
        # Test line unfolding
        assert event["summary"] == "Meeting with Team"
        # Test unescaping
        assert (
            event["description"]
            == "Project status update, discussing\nkey deliverables."
        )
        assert event["location"] == "Conference Room A"


def test_export_public_sheet_to_csv(httpx_mock) -> None:
    """Test exporting a public Google Sheet to CSV."""
    mock_csv = "Col1,Col2\nVal1,Val2"
    httpx_mock.add_response(
        url="https://docs.google.com/spreadsheets/d/sheet123/export?format=csv&gid=456",
        text=mock_csv,
    )
    with KeylessGoogleClient() as client:
        csv_data = client.export_public_sheet_to_csv("sheet123", gid="456")
        assert csv_data == mock_csv


def test_export_public_doc_to_text(httpx_mock) -> None:
    """Test exporting a public Google Doc to text."""
    mock_text = "This is a public Google Doc content."
    httpx_mock.add_response(
        url="https://docs.google.com/document/d/doc123/export?format=txt",
        text=mock_text,
    )
    with KeylessGoogleClient() as client:
        doc_text = client.export_public_doc_to_text("doc123")
        assert doc_text == mock_text


def test_download_public_drive_file(httpx_mock) -> None:
    """Test downloading a public Google Drive file."""
    mock_bytes = b"sample file bytes"
    httpx_mock.add_response(
        url="https://docs.google.com/uc?export=download&id=file123",
        content=mock_bytes,
    )
    with KeylessGoogleClient() as client:
        file_data = client.download_public_drive_file("file123")
        assert file_data == mock_bytes


def test_download_public_drive_file_large(httpx_mock) -> None:
    """Test downloading a large public Google Drive file with confirm token warning page."""
    # First response returns warning HTML with confirmation token
    httpx_mock.add_response(
        url="https://docs.google.com/uc?export=download&id=file123",
        text="Download warning page... <a href='/uc?export=download&confirm=xyz123&id=file123'>Confirm</a>",
    )
    # Second response returns file bytes after confirming
    httpx_mock.add_response(
        url="https://docs.google.com/uc?export=download&id=file123&confirm=xyz123",
        content=b"large file bytes",
    )
    with KeylessGoogleClient() as client:
        file_data = client.download_public_drive_file("file123")
        assert file_data == b"large file bytes"


def test_api_error_propagation(httpx_mock) -> None:
    """Test that ApiError is raised on HTTP errors."""
    httpx_mock.add_response(
        url="https://dns.google/resolve?name=fail.com&type=A",
        status_code=503,
        text="Service Unavailable",
    )
    with KeylessGoogleClient() as client:
        with pytest.raises(ApiError) as exc_info:
            client.resolve_dns("fail.com", "A")
        assert exc_info.value.status == 503
        assert "Service Unavailable" in exc_info.value.body


class TestAsyncKeylessGoogleClient(unittest.IsolatedAsyncioTestCase):
    """Async tests for AsyncKeylessGoogleClient features using IsolatedAsyncioTestCase."""

    async def test_context_manager(self) -> None:
        """Test that client supports async context manager protocol."""
        async with AsyncKeylessGoogleClient() as client:
            self.assertIsNotNone(client)

    async def test_resolve_dns(self) -> None:
        """Test Google Public DNS resolution asynchronously."""
        import httpx

        def handler(request: httpx.Request) -> httpx.Response:
            self.assertEqual(
                str(request.url),
                "https://dns.google/resolve?name=example.com&type=A",
            )
            return httpx.Response(
                200,
                json={
                    "Status": 0,
                    "Answer": [
                        {
                            "name": "example.com.",
                            "type": 1,
                            "data": "93.184.215.14",
                        }
                    ],
                },
            )

        transport = httpx.MockTransport(handler)
        async with httpx.AsyncClient(transport=transport) as http:
            async with AsyncKeylessGoogleClient(http=http) as client:
                res = await client.resolve_dns("example.com", "A")
                self.assertEqual(res["Status"], 0)
                self.assertEqual(res["Answer"][0]["data"], "93.184.215.14")

    async def test_search_books(self) -> None:
        """Test Google Books catalog searching asynchronously."""
        import httpx

        def handler(request: httpx.Request) -> httpx.Response:
            self.assertEqual(
                str(request.url),
                "https://www.googleapis.com/books/v1/volumes?q=test&maxResults=5&startIndex=0",
            )
            return httpx.Response(
                200,
                json={
                    "kind": "books#volumes",
                    "items": [
                        {"id": "vol1", "volumeInfo": {"title": "Test Book"}}
                    ],
                },
            )

        transport = httpx.MockTransport(handler)
        async with httpx.AsyncClient(transport=transport) as http:
            async with AsyncKeylessGoogleClient(http=http) as client:
                res = await client.search_books("test", max_results=5)
                self.assertEqual(res["kind"], "books#volumes")
                self.assertEqual(
                    res["items"][0]["volumeInfo"]["title"], "Test Book"
                )

    async def test_get_book(self) -> None:
        """Test retrieving a specific Google Books volume asynchronously."""
        import httpx

        def handler(request: httpx.Request) -> httpx.Response:
            self.assertEqual(
                str(request.url),
                "https://www.googleapis.com/books/v1/volumes/vol1",
            )
            return httpx.Response(
                200,
                json={
                    "id": "vol1",
                    "volumeInfo": {"title": "Specific Test Book"},
                },
            )

        transport = httpx.MockTransport(handler)
        async with httpx.AsyncClient(transport=transport) as http:
            async with AsyncKeylessGoogleClient(http=http) as client:
                res = await client.get_book("vol1")
                self.assertEqual(res["id"], "vol1")
                self.assertEqual(
                    res["volumeInfo"]["title"], "Specific Test Book"
                )

    async def test_translate(self) -> None:
        """Test translating text asynchronously."""
        import httpx

        mock_response = [
            [
                ["Bonjour", "Hello", None, None, 1],
                [" le monde", " world", None, None, 1],
            ],
            None,
            "en",
        ]

        def handler(request: httpx.Request) -> httpx.Response:
            self.assertEqual(
                str(request.url),
                "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=fr&dt=t&q=Hello+world",
            )
            return httpx.Response(200, json=mock_response)

        transport = httpx.MockTransport(handler)
        async with httpx.AsyncClient(transport=transport) as http:
            async with AsyncKeylessGoogleClient(http=http) as client:
                translated = await client.translate(
                    "Hello world", target_lang="fr"
                )
                self.assertEqual(translated, "Bonjour le monde")

    async def test_autocomplete(self) -> None:
        """Test autocomplete query suggestions asynchronously."""
        import httpx

        def handler(request: httpx.Request) -> httpx.Response:
            self.assertEqual(
                str(request.url),
                "https://suggestqueries.google.com/complete/search?client=chrome&q=python",
            )
            return httpx.Response(
                200,
                json=[
                    "python",
                    [
                        "python tutorial",
                        "python download",
                        "python documentation",
                    ],
                ],
            )

        transport = httpx.MockTransport(handler)
        async with httpx.AsyncClient(transport=transport) as http:
            async with AsyncKeylessGoogleClient(http=http) as client:
                suggestions = await client.autocomplete("python")
                self.assertEqual(
                    suggestions,
                    [
                        "python tutorial",
                        "python download",
                        "python documentation",
                    ],
                )

    async def test_get_public_calendar_events(self) -> None:
        """Test fetching and parsing public Google iCalendar feeds asynchronously."""
        import httpx

        mock_ics = (
            "BEGIN:VCALENDAR\r\n"
            "VERSION:2.0\r\n"
            "PRODID:-//Google Inc//Google Calendar 70.9054//EN\r\n"
            "BEGIN:VEVENT\r\n"
            "UID:event123@google.com\r\n"
            "DTSTART:20260523T080000Z\r\n"
            "DTEND:20260523T090000Z\r\n"
            "SUMMARY:Meeting with\r\n"
            "  Team\r\n"
            "DESCRIPTION:Project status update\\, discussing\\nkey deliverables.\r\n"
            "LOCATION:Conference Room A\r\n"
            "END:VEVENT\r\n"
            "END:VCALENDAR\r\n"
        )

        def handler(request: httpx.Request) -> httpx.Response:
            self.assertEqual(
                str(request.url),
                "https://calendar.google.com/calendar/ical/test_cal/public/basic.ics",
            )
            return httpx.Response(200, text=mock_ics)

        transport = httpx.MockTransport(handler)
        async with httpx.AsyncClient(transport=transport) as http:
            async with AsyncKeylessGoogleClient(http=http) as client:
                events = await client.get_public_calendar_events("test_cal")
                self.assertEqual(len(events), 1)
                event = events[0]
                self.assertEqual(event["uid"], "event123@google.com")
                self.assertEqual(event["dtstart"], "20260523T080000Z")
                self.assertEqual(event["dtend"], "20260523T090000Z")
                self.assertEqual(event["summary"], "Meeting with Team")
                self.assertEqual(
                    event["description"],
                    "Project status update, discussing\nkey deliverables.",
                )
                self.assertEqual(event["location"], "Conference Room A")

    async def test_export_public_sheet_to_csv(self) -> None:
        """Test exporting a public Google Sheet to CSV asynchronously."""
        import httpx

        mock_csv = "Col1,Col2\nVal1,Val2"

        def handler(request: httpx.Request) -> httpx.Response:
            self.assertEqual(
                str(request.url),
                "https://docs.google.com/spreadsheets/d/sheet123/export?format=csv&gid=456",
            )
            return httpx.Response(200, text=mock_csv)

        transport = httpx.MockTransport(handler)
        async with httpx.AsyncClient(transport=transport) as http:
            async with AsyncKeylessGoogleClient(http=http) as client:
                csv_data = await client.export_public_sheet_to_csv(
                    "sheet123", gid="456"
                )
                self.assertEqual(csv_data, mock_csv)

    async def test_export_public_doc_to_text(self) -> None:
        """Test exporting a public Google Doc to text asynchronously."""
        import httpx

        mock_text = "This is a public Google Doc content."

        def handler(request: httpx.Request) -> httpx.Response:
            self.assertEqual(
                str(request.url),
                "https://docs.google.com/document/d/doc123/export?format=txt",
            )
            return httpx.Response(200, text=mock_text)

        transport = httpx.MockTransport(handler)
        async with httpx.AsyncClient(transport=transport) as http:
            async with AsyncKeylessGoogleClient(http=http) as client:
                doc_text = await client.export_public_doc_to_text("doc123")
                self.assertEqual(doc_text, mock_text)

    async def test_download_public_drive_file(self) -> None:
        """Test downloading a public Google Drive file asynchronously."""
        import httpx

        mock_bytes = b"sample file bytes"

        def handler(request: httpx.Request) -> httpx.Response:
            self.assertEqual(
                str(request.url),
                "https://docs.google.com/uc?export=download&id=file123",
            )
            return httpx.Response(200, content=mock_bytes)

        transport = httpx.MockTransport(handler)
        async with httpx.AsyncClient(transport=transport) as http:
            async with AsyncKeylessGoogleClient(http=http) as client:
                file_data = await client.download_public_drive_file("file123")
                self.assertEqual(file_data, mock_bytes)

    async def test_download_public_drive_file_large(self) -> None:
        """Test downloading a large public Google Drive file asynchronously."""
        import httpx

        calls = 0

        def handler(request: httpx.Request) -> httpx.Response:
            nonlocal calls
            calls += 1
            if calls == 1:
                self.assertEqual(
                    str(request.url),
                    "https://docs.google.com/uc?export=download&id=file123",
                )
                return httpx.Response(
                    200,
                    text="Download warning page... <a href='/uc?export=download&confirm=xyz123&id=file123'>Confirm</a>",
                )
            elif calls == 2:
                self.assertEqual(
                    str(request.url),
                    "https://docs.google.com/uc?export=download&id=file123&confirm=xyz123",
                )
                return httpx.Response(200, content=b"large file bytes")
            else:
                self.fail("Too many calls to handler")

        transport = httpx.MockTransport(handler)
        async with httpx.AsyncClient(transport=transport) as http:
            async with AsyncKeylessGoogleClient(http=http) as client:
                file_data = await client.download_public_drive_file("file123")
                self.assertEqual(file_data, b"large file bytes")

    async def test_api_error_propagation(self) -> None:
        """Test that ApiError is raised on HTTP errors asynchronously."""
        import httpx

        def handler(request: httpx.Request) -> httpx.Response:
            self.assertEqual(
                str(request.url),
                "https://dns.google/resolve?name=fail.com&type=A",
            )
            return httpx.Response(503, text="Service Unavailable")

        transport = httpx.MockTransport(handler)
        async with httpx.AsyncClient(transport=transport) as http:
            async with AsyncKeylessGoogleClient(http=http) as client:
                with self.assertRaises(ApiError) as ctx:
                    await client.resolve_dns("fail.com", "A")
                self.assertEqual(ctx.exception.status, 503)
                self.assertIn("Service Unavailable", ctx.exception.body)
