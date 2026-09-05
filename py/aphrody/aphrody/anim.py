# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Frame animation — turn sprite frames into GIF / animated WebP / APNG + sheets.

Builds looping animations and texture spritesheets from a set of frame images
(e.g. the 8 rotation frames of an Inazuma model viewer sprite). Pillow does all
the encoding — animated **WebP** is the smallest/cleanest with full alpha, GIF
is the most compatible, **APNG** is lossless with alpha. ``make_spritesheet``
packs frames into a uniform grid plus a JSON atlas (Phaser/Cocos-friendly).

    >>> from aphrody.anim import build_animation, make_spritesheet
    >>> build_animation(frames, "turntable.webp", fmt="webp", fps=10, pingpong=True)  # doctest: +SKIP
    >>> make_spritesheet(frames, "sheet.png", columns=4)                              # doctest: +SKIP
"""

from __future__ import annotations

import glob
import json
import logging
import math
import re
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)

#: Animation formats this module can emit.
ANIM_FORMATS: tuple[str, ...] = ("webp", "gif", "apng")
_FRAME_INDEX_RE = re.compile(r"_r(\d+)")


def _require_pillow() -> Any:
    """Import and return ``PIL.Image`` or raise a helpful error."""
    try:
        from PIL import Image
    except ImportError as exc:  # pragma: no cover - depends on env
        raise RuntimeError(
            "Pillow is required for animation; "
            "install the images extra: uv pip install 'aphrody[images]'"
        ) from exc
    return Image


def _open_rgba(path: str | Path) -> Any:
    """Open *path* as a loaded RGBA Pillow image."""
    image_mod = _require_pillow()
    img = image_mod.open(path)
    img.load()
    return img.convert("RGBA")


def _duration_ms(fps: float) -> int:
    """Convert frames-per-second to per-frame duration in milliseconds."""
    if fps <= 0:
        raise ValueError(f"fps must be > 0, got {fps}")
    return max(1, round(1000.0 / fps))


def pingpong_frames(frames: list[Any]) -> list[Any]:
    """Return *frames* followed by their reverse (minus the duplicated ends).

    Produces a smooth back-and-forth loop from a one-directional sequence.
    """
    if len(frames) <= 2:
        return list(frames)
    return list(frames) + list(frames[-2:0:-1])


def sort_frames_by_index(paths: list[str | Path]) -> list[Path]:
    """Sort frame paths by the ``_r<n>`` index in the filename, then by name.

    Args:
        paths: Frame file paths.

    Returns:
        Paths ordered by rotation index (``_r0`` … ``_r7``); files without an
        index sort last by name.
    """

    def key(p: str | Path) -> tuple[int, str]:
        m = _FRAME_INDEX_RE.search(Path(p).stem)
        return (int(m.group(1)) if m else 1_000_000, Path(p).name)

    return [Path(p) for p in sorted(paths, key=key)]


def _to_gif_palette(
    frame: Any, *, transparent: bool, background: tuple[int, int, int]
) -> Any:
    """Convert an RGBA *frame* to a palette ('P') image for GIF encoding."""
    image_mod = _require_pillow()
    if not transparent:
        bg = image_mod.new("RGBA", frame.size, (*background, 255))
        bg.alpha_composite(frame)
        return bg.convert("RGB").convert(
            "P", palette=image_mod.Palette.ADAPTIVE, colors=256
        )
    # Reserve palette index 255 for transparency.
    alpha = frame.getchannel("A")
    p = frame.convert("RGB").convert(
        "P", palette=image_mod.Palette.ADAPTIVE, colors=255
    )
    mask = alpha.point(lambda a: 255 if a <= 128 else 0)
    p.paste(255, mask)
    p.info["transparency"] = 255
    return p


def build_animation(
    frames: list[str | Path],
    out: str | Path,
    *,
    fmt: str | None = None,
    fps: float = 12.0,
    loop: int = 0,
    pingpong: bool = False,
    transparent: bool = True,
    background: tuple[int, int, int] = (255, 255, 255),
    quality: int = 85,
) -> Path:
    """Encode *frames* into a looping animation.

    Args:
        frames: Ordered frame image paths.
        out: Destination path; the extension picks the format if *fmt* is None.
        fmt: ``"webp"`` / ``"gif"`` / ``"apng"`` (else inferred from *out*).
        fps: Frames per second.
        loop: Loop count (0 = infinite).
        pingpong: Append the reversed sequence for a smooth back-and-forth.
        transparent: Preserve alpha (WebP/APNG natively; GIF via a reserved
            palette index). When False, frames are composited over *background*.
        background: Background colour used when *transparent* is False.
        quality: WebP quality (0-100).

    Returns:
        The written animation ``Path``.

    Raises:
        ValueError: If no frames are given or the format is unsupported.
        RuntimeError: If Pillow is not installed.
    """
    if not frames:
        raise ValueError("build_animation: at least one frame is required")
    image_mod = _require_pillow()
    fmt = (fmt or Path(out).suffix.lstrip(".") or "webp").lower()
    if fmt not in ANIM_FORMATS:
        raise ValueError(
            f"unsupported animation format {fmt!r}; choose {', '.join(ANIM_FORMATS)}"
        )

    imgs = [_open_rgba(p) for p in frames]
    if pingpong:
        imgs = pingpong_frames(imgs)
    duration = _duration_ms(fps)
    dest = Path(out)
    dest.parent.mkdir(parents=True, exist_ok=True)

    if fmt == "webp":
        if not transparent:
            flat = []
            for f in imgs:
                bg = image_mod.new("RGBA", f.size, (*background, 255))
                bg.alpha_composite(f)
                flat.append(bg)
            imgs = flat
        imgs[0].save(
            dest,
            format="WEBP",
            save_all=True,
            append_images=imgs[1:],
            duration=duration,
            loop=loop,
            quality=quality,
            method=6,
            minimize_size=True,
        )
    elif fmt == "apng":
        imgs[0].save(
            dest,
            format="PNG",
            save_all=True,
            append_images=imgs[1:],
            duration=duration,
            loop=loop,
            disposal=1,
        )
    else:  # gif
        pal = [
            _to_gif_palette(f, transparent=transparent, background=background)
            for f in imgs
        ]
        save_kw: dict[str, Any] = {
            "format": "GIF",
            "save_all": True,
            "append_images": pal[1:],
            "duration": duration,
            "loop": loop,
            "optimize": False,
            "disposal": 2 if transparent else 0,
        }
        if transparent:
            save_kw["transparency"] = 255
        pal[0].save(dest, **save_kw)

    logger.info(
        "wrote %d-frame %s animation (%.0f fps) -> %s",
        len(imgs),
        fmt,
        fps,
        dest,
    )
    return dest


def make_spritesheet(
    frames: list[str | Path],
    out: str | Path,
    *,
    columns: int | None = None,
    background: tuple[int, int, int] | None = None,
) -> tuple[Path, dict[str, Any]]:
    """Pack *frames* into a uniform grid spritesheet plus a JSON atlas.

    Args:
        frames: Frame image paths (assumed roughly uniform size).
        out: Destination sheet path (PNG keeps alpha). A sibling ``.json``
            atlas is written next to it.
        columns: Grid columns; defaults to ``ceil(sqrt(n))`` (near-square).
        background: Solid background; ``None`` keeps transparency (RGBA sheet).

    Returns:
        ``(sheet_path, atlas)`` where *atlas* describes the frame rectangles.

    Raises:
        ValueError: If no frames are given.
        RuntimeError: If Pillow is not installed.
    """
    if not frames:
        raise ValueError("make_spritesheet: at least one frame is required")
    image_mod = _require_pillow()
    imgs = [_open_rgba(p) for p in frames]
    cell_w = max(f.width for f in imgs)
    cell_h = max(f.height for f in imgs)
    n = len(imgs)
    cols = columns or math.ceil(math.sqrt(n))
    rows = math.ceil(n / cols)

    if background is None:
        sheet = image_mod.new(
            "RGBA", (cols * cell_w, rows * cell_h), (0, 0, 0, 0)
        )
    else:
        sheet = image_mod.new("RGB", (cols * cell_w, rows * cell_h), background)

    atlas: dict[str, Any] = {
        "frame_width": cell_w,
        "frame_height": cell_h,
        "columns": cols,
        "rows": rows,
        "count": n,
        "frames": [],
    }
    for i, (f, src) in enumerate(zip(imgs, frames)):
        x = (i % cols) * cell_w
        y = (i // cols) * cell_h
        # Centre each frame in its cell.
        ox = x + (cell_w - f.width) // 2
        oy = y + (cell_h - f.height) // 2
        sheet.paste(f, (ox, oy), f)
        atlas["frames"].append(
            {
                "index": i,
                "name": Path(src).name,
                "x": x,
                "y": y,
                "w": cell_w,
                "h": cell_h,
            }
        )

    dest = Path(out)
    dest.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(dest)
    atlas_path = dest.with_suffix(".json")
    atlas_path.write_text(json.dumps(atlas, indent=2), encoding="utf-8")
    logger.info(
        "wrote %dx%d spritesheet (%d frames) -> %s (+ atlas %s)",
        cols,
        rows,
        n,
        dest,
        atlas_path.name,
    )
    return dest, atlas


def turntable(
    pattern: str,
    out: str | Path,
    *,
    fmt: str | None = None,
    fps: float = 10.0,
    pingpong: bool = True,
    transparent: bool = True,
) -> Path:
    """Build a rotation animation from frames matching a glob *pattern*.

    Frames are ordered by their ``_r<n>`` index, so e.g.
    ``assets/aphrody_r*.webp`` becomes a smooth turntable loop.

    Args:
        pattern: A glob pattern matching the rotation frames.
        out: Destination animation path.
        fmt: Output format (else inferred from *out*).
        fps: Frames per second.
        pingpong: Back-and-forth loop (good for a <360° rotation sweep).
        transparent: Preserve alpha.

    Returns:
        The written animation ``Path``.

    Raises:
        FileNotFoundError: If *pattern* matches no files.
    """
    matches = glob.glob(pattern)
    if not matches:
        raise FileNotFoundError(f"no frames match {pattern!r}")
    ordered = sort_frames_by_index(matches)
    return build_animation(
        ordered,  # type: ignore[arg-type]
        out,
        fmt=fmt,
        fps=fps,
        pingpong=pingpong,
        transparent=transparent,
    )
