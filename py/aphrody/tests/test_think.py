# SPDX-License-Identifier: Apache-2.0
"""Tests for the deep thinking features in GeminiVertex."""

from __future__ import annotations

from unittest import mock

from aphrody.vertex import GeminiVertex


def test_generate_think_explicit_parts() -> None:
    class MockPartThought:
        thought = True
        text = "This is my thought process."

    class MockPartText:
        thought = False
        text = "This is the final response."

    class MockContent:
        parts = (MockPartThought(), MockPartText())

    class MockCandidate:
        content = MockContent()

    class MockResponse:
        candidates = (MockCandidate(),)
        text = "This is the final response."

    # Mock credentials and client call
    with (
        mock.patch("aphrody.auth.credentials.load_google_credentials"),
        mock.patch("google.genai.Client") as mock_client_class,
    ):
        mock_client = mock_client_class.return_value
        mock_client.models.generate_content.return_value = MockResponse()

        gv = GeminiVertex()
        thought, response = gv.generate_think("Do math", budget=100)

        assert thought == "This is my thought process."
        assert response == "This is the final response."


def test_generate_think_tag_fallback() -> None:
    class MockPart:
        thought = False
        text = "<thought>Thinking hard about this</thought>\nThe answer is 42."

    class MockContent:
        parts = (MockPart(),)

    class MockCandidate:
        content = MockContent()

    class MockResponse:
        candidates = (MockCandidate(),)
        text = "<thought>Thinking hard about this</thought>\nThe answer is 42."

    with (
        mock.patch("aphrody.auth.credentials.load_google_credentials"),
        mock.patch("google.genai.Client") as mock_client_class,
    ):
        mock_client = mock_client_class.return_value
        mock_client.models.generate_content.return_value = MockResponse()

        gv = GeminiVertex()
        thought, response = gv.generate_think("Life meaning", budget=100)

        assert thought == "Thinking hard about this"
        assert response == "The answer is 42."
