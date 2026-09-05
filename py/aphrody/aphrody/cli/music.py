# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Music generation command group for the aphrody CLI."""

from __future__ import annotations

from aphrody.cli.utils import _emit


class MusicCommands:
    """``aphrody music <action>`` — music generation features."""

    def gen(
        self,
        prompt: str,
        out: str = "music.wav",
        duration: int = 10,
        model: str = "audio-generation",
        dry_run: bool = False,
    ) -> None:
        """Generate one music track from a text prompt.

        Args:
            prompt: Musical style, instruments, or description.
            out: Destination path.
            duration: Duration in seconds.
            model: Audio generation model override.
            dry_run: If True, generate offline fallback without calling API.
        """
        from aphrody.media import MusicGenerator

        mg = MusicGenerator()
        saved_path = mg.generate_music(
            prompt,
            out=out,
            duration_seconds=duration,
            model=model,
            dry_run=dry_run,
        )

        _emit(
            {
                "action": "music_generation",
                "saved_to": str(saved_path),
                "duration_seconds": duration,
                "prompt": prompt,
                "model": model,
                "dry_run": dry_run,
            }
        )
