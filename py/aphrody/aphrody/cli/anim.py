# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Frame animation and spritesheet command group for the aphrody CLI."""

from __future__ import annotations

from pathlib import Path

from aphrody.cli.utils import _emit
from aphrody.errors import AphrodyError


class AnimCommands:
    """``aphrody image anim <action>`` — frame animation + spritesheets.

    Turn sprite frames (e.g. rotation frames) into a looping animated WebP / GIF
    / APNG, or pack them into a uniform-grid spritesheet with a JSON atlas.
    """

    def build(
        self,
        *frames: str,
        out: str = "anim.webp",
        fmt: str | None = None,
        fps: float = 12.0,
        loop: int = 0,
        pingpong: bool = False,
        transparent: bool = True,
    ) -> None:
        """Build a looping animation from explicit frame paths.

        Args:
            *frames: Ordered frame image paths.
            out: Destination path (extension picks the format if --fmt unset).
            fmt: ``webp`` / ``gif`` / ``apng``.
            fps: Frames per second.
            loop: Loop count (0 = infinite).
            pingpong: Append the reversed sequence for a back-and-forth loop.
            transparent: Preserve alpha.
        """
        from aphrody import anim

        if not frames:
            raise AphrodyError("anim build requires at least one frame path")
        path = anim.build_animation(
            list(frames),
            out,
            fmt=fmt,
            fps=fps,
            loop=loop,
            pingpong=pingpong,
            transparent=transparent,
        )
        _emit({"saved": str(path), "frames": len(frames), "fps": fps})

    def turntable(
        self,
        pattern: str,
        out: str = "turntable.webp",
        fmt: str | None = None,
        fps: float = 10.0,
        pingpong: bool = True,
        transparent: bool = True,
    ) -> None:
        """Build a rotation loop from frames matching a glob ``pattern``.

        Frames are ordered by their ``_r<n>`` index (e.g.
        ``assets/aphrody_r*.webp``).

        Args:
            pattern: Glob pattern matching the rotation frames.
            out: Destination animation path.
            fmt: ``webp`` / ``gif`` / ``apng``.
            fps: Frames per second.
            pingpong: Back-and-forth loop (smooth for a <360 sweep).
            transparent: Preserve alpha.
        """
        from aphrody import anim

        path = anim.turntable(
            pattern,
            out,
            fmt=fmt,
            fps=fps,
            pingpong=pingpong,
            transparent=transparent,
        )
        _emit({"saved": str(path), "pattern": pattern, "fps": fps})

    def spritesheet(
        self,
        *frames: str,
        out: str = "spritesheet.png",
        columns: int | None = None,
    ) -> None:
        """Pack frames into a uniform-grid spritesheet + JSON atlas.

        Args:
            *frames: Frame image paths.
            out: Destination PNG path (a sibling ``.json`` atlas is written).
            columns: Grid columns (defaults to near-square).
        """
        from aphrody import anim

        if not frames:
            raise AphrodyError("anim spritesheet requires at least one frame")
        path, atlas = anim.make_spritesheet(list(frames), out, columns=columns)
        _emit(
            {
                "saved": str(path),
                "atlas": str(Path(path).with_suffix(".json")),
                "grid": f"{atlas['columns']}x{atlas['rows']}",
                "frame_size": [atlas["frame_width"], atlas["frame_height"]],
                "count": atlas["count"],
            }
        )
