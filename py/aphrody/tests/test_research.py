# SPDX-License-Identifier: Apache-2.0
"""Tests for Deep Research agentic grounded search loops."""

from __future__ import annotations

from unittest import mock

from aphrody.research import DeepResearcher


def test_researcher_dry_run_fallback(tmp_path) -> None:
    dr = DeepResearcher()
    out = tmp_path / "test_report.md"

    saved = dr.conduct_research(
        "GCP architecture", depth=1, out=out, dry_run=True
    )

    assert saved == out
    assert out.exists()
    content = out.read_text(encoding="utf-8")
    assert "Deep Research Report: GCP architecture" in content
    assert "Bibliography" in content


def test_researcher_api_success(tmp_path) -> None:
    class MockWeb:
        title = "GCP Cloud Computing"
        uri = "https://cloud.google.com"

    class MockGroundingChunk:
        web = MockWeb()

    class MockGroundingMetadata:
        grounding_chunks = (MockGroundingChunk(),)

    class MockCandidate:
        content = mock.Mock()
        grounding_metadata = MockGroundingMetadata()

    class MockResponse:
        candidates = (MockCandidate(),)
        text = "This is synthesized research text."

    with (
        mock.patch("aphrody.auth.credentials.load_google_credentials"),
        mock.patch("google.genai.Client") as mock_client_class,
    ):
        mock_client = mock_client_class.return_value
        mock_client.models.generate_content.return_value = MockResponse()

        dr = DeepResearcher()
        out = tmp_path / "test_report.md"
        saved = dr.conduct_research(
            "Vertex AI Search", depth=1, out=out, dry_run=False
        )

        assert saved == out
        assert out.exists()
        content = out.read_text(encoding="utf-8")
        assert "This is synthesized research text." in content
        assert "[GCP Cloud Computing](https://cloud.google.com)" in content
