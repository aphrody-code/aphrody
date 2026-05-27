# SPDX-License-Identifier: Apache-2.0
"""Tests for CLI routing of new think, video, music, and research commands."""

from __future__ import annotations

from unittest import mock

from aphrody.cli import Aphrody


def test_cli_think_routing() -> None:
    cli = Aphrody()
    with mock.patch("aphrody.vertex.GeminiVertex") as mock_vertex_class:
        mock_vertex = mock_vertex_class.return_value
        mock_vertex.generate_think.return_value = ("thought", "response")

        cli.think("explain relativity", budget=1000)

        mock_vertex.generate_think.assert_called_once_with(
            "explain relativity", budget=1000
        )


def test_cli_video_gen_routing() -> None:
    cli = Aphrody()
    with mock.patch("aphrody.media.VideoGenerator") as mock_vg_class:
        mock_vg = mock_vg_class.return_value
        mock_vg.generate_video.return_value = "saved_video.mp4"

        video_cmds = cli.video()
        video_cmds.gen(
            "a falling leaf",
            out="leaf.mp4",
            aspect="16:9",
            duration=3,
            dry_run=True,
        )

        mock_vg.generate_video.assert_called_once_with(
            "a falling leaf",
            out="leaf.mp4",
            aspect_ratio="16:9",
            duration_seconds=3,
            model="veo-2.0-generate-001",
            dry_run=True,
        )


def test_cli_music_gen_routing() -> None:
    cli = Aphrody()
    with mock.patch("aphrody.media.MusicGenerator") as mock_mg_class:
        mock_mg = mock_mg_class.return_value
        mock_mg.generate_music.return_value = "saved_music.wav"

        music_cmds = cli.music()
        music_cmds.gen("fast jazz", out="jazz.wav", duration=5, dry_run=True)

        mock_mg.generate_music.assert_called_once_with(
            "fast jazz",
            out="jazz.wav",
            duration_seconds=5,
            model="audio-generation",
            dry_run=True,
        )


def test_cli_research_routing() -> None:
    cli = Aphrody()
    with mock.patch("aphrody.research.DeepResearcher") as mock_dr_class:
        mock_dr = mock_dr_class.return_value
        mock_dr.conduct_research.return_value = "report.md"

        research_cmds = cli.research()
        research_cmds("python 2026", depth=3, out="report.md", dry_run=True)

        mock_dr.conduct_research.assert_called_once_with(
            "python 2026",
            depth=3,
            out="report.md",
            dry_run=True,
        )
