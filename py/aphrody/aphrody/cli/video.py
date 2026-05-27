# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Video generation command group for the aphrody CLI."""

from __future__ import annotations

from aphrody.cli.utils import _emit


class VideoCommands:
    """``aphrody video <action>`` — video generation features."""

    def gen(
        self,
        prompt: str,
        out: str = "video.mp4",
        aspect: str = "16:9",
        duration: int = 5,
        model: str = "veo-2.0-generate-001",
        dry_run: bool = False,
    ) -> None:
        """Generate one video from a text prompt.

        Args:
            prompt: Description of the video scene.
            out: Destination path.
            aspect: Aspect ratio ('16:9', '1:1', '4:3').
            duration: Duration in seconds.
            model: Veo model id override.
            dry_run: If True, generate offline fallback without calling API.
        """
        from aphrody.media import VideoGenerator

        vg = VideoGenerator()
        saved_path = vg.generate_video(
            prompt,
            out=out,
            aspect_ratio=aspect,
            duration_seconds=duration,
            model=model,
            dry_run=dry_run,
        )

        _emit(
            {
                "action": "video_generation",
                "saved_to": str(saved_path),
                "aspect_ratio": aspect,
                "duration_seconds": duration,
                "prompt": prompt,
                "model": model,
                "dry_run": dry_run,
            }
        )
