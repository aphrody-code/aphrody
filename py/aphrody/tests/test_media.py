# SPDX-License-Identifier: Apache-2.0
"""Tests for video and music generation logic and fallbacks."""

from __future__ import annotations

from unittest import mock

from aphrody.media import MusicGenerator, VideoGenerator


def test_video_generator_dry_run_fallback(tmp_path) -> None:
    vg = VideoGenerator()
    out = tmp_path / "test_video.mp4"

    saved = vg.generate_video("a cute cat playing piano", out=out, dry_run=True)

    assert saved == out
    assert out.exists()
    assert out.stat().st_size > 0


def test_video_generator_api_success(tmp_path) -> None:
    mock_result = mock.Mock()
    mock_result.generated_videos = [
        mock.Mock(video=mock.Mock(image_bytes=b"MP4_DATA"))
    ]

    class MockOperation:
        done = True
        result = mock_result

    with (
        mock.patch("aphrody.auth.credentials.load_google_credentials"),
        mock.patch("google.genai.Client") as mock_client_class,
    ):
        mock_client = mock_client_class.return_value
        mock_client.models.generate_videos.return_value = MockOperation()

        vg = VideoGenerator()
        out = tmp_path / "test_video.mp4"
        saved = vg.generate_video("a cute cat", out=out, dry_run=False)

        assert saved == out
        assert out.exists()
        assert out.read_bytes() == b"MP4_DATA"


def test_music_generator_dry_run_fallback(tmp_path) -> None:
    mg = MusicGenerator()
    out = tmp_path / "test_music.wav"

    saved = mg.generate_music(
        "lofi beats to study to", out=out, duration_seconds=2, dry_run=True
    )

    assert saved == out
    assert out.exists()
    assert out.stat().st_size > 44  # WAV header is 44 bytes


def test_music_generator_api_success(tmp_path) -> None:
    class MockInlineData:
        mime_type = "audio/wav"
        data = b"WAV_DATA"

    class MockPart:
        inline_data = MockInlineData()

    class MockContent:
        parts = (MockPart(),)

    class MockCandidate:
        content = MockContent()

    class MockResponse:
        candidates = (MockCandidate(),)

    with (
        mock.patch("aphrody.auth.credentials.load_google_credentials"),
        mock.patch("google.genai.Client") as mock_client_class,
    ):
        mock_client = mock_client_class.return_value
        mock_client.models.generate_content.return_value = MockResponse()

        mg = MusicGenerator()
        out = tmp_path / "test_music.wav"
        saved = mg.generate_music("lofi beats", out=out, dry_run=False)

        assert saved == out
        assert out.exists()
        assert out.read_bytes() == b"WAV_DATA"
