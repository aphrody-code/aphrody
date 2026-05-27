# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Deep image analysis — decode everything needed to recreate an image.

Extracts the full technical fingerprint of an image (format, mode, animation,
alpha, EXIF, the subject's tight bounding box, the dominant-colour palette and
the mean colour) so it can be recreated and varied pixel-accurately. Pillow does
the decoding; NumPy accelerates the colour histogram when available, with a
pure-Pillow fallback.

    >>> from aphrody.analyze import analyze_image
    >>> report = analyze_image("assets/aphrody-body.webp")   # doctest: +SKIP
    >>> report["size"], report["dominant_colors"][0]["hex"]  # doctest: +SKIP
"""

from __future__ import annotations

import logging
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)

#: Alpha below this is treated as background (transparent).
_ALPHA_BG_THRESHOLD = 16
#: RGB min above this (all channels) is treated as near-white background.
_WHITE_BG_THRESHOLD = 244


def _require_pillow() -> Any:
    """Import and return ``PIL.Image`` or raise a helpful error."""
    try:
        from PIL import Image
    except ImportError as exc:  # pragma: no cover - depends on env
        raise RuntimeError(
            "Pillow is required for image analysis; "
            "install the images extra: uv pip install 'aphrody[images]'"
        ) from exc
    return Image


def _rgb_to_hex(rgb: tuple[int, int, int]) -> str:
    """Format an ``(r, g, b)`` tuple as ``#RRGGBB``."""
    return "#{:02X}{:02X}{:02X}".format(*rgb)


def _is_background(r: int, g: int, b: int, a: int) -> bool:
    """Return ``True`` if a pixel is transparent or near-white background."""
    if a < _ALPHA_BG_THRESHOLD:
        return True
    return (
        r >= _WHITE_BG_THRESHOLD
        and g >= _WHITE_BG_THRESHOLD
        and b >= _WHITE_BG_THRESHOLD
    )


def _flat_pixels(rgba: Any) -> Any:
    """Yield ``(r, g, b, a)`` tuples without the deprecated ``getdata``."""
    getter = getattr(rgba, "get_flattened_data", None)
    return getter() if getter else rgba.getdata()


def dominant_colors(
    img: Any,
    *,
    n: int = 8,
    drop_background: bool = True,
) -> list[dict[str, Any]]:
    """Return the *n* most frequent colours in *img*.

    Uses a vectorised NumPy histogram when NumPy is available, falling back to a
    pure-Pillow ``Counter``.

    Args:
        img: A Pillow ``Image``.
        n: Maximum number of colours to return.
        drop_background: Skip fully-transparent and near-white-background pixels
            so the palette reflects the subject, not the canvas.

    Returns:
        A list of ``{"hex", "rgb", "fraction"}`` dicts, most frequent first.
        ``fraction`` is the share of the *kept* pixels.
    """
    rgba = img.convert("RGBA")
    try:
        import numpy as np

        arr = np.asarray(rgba).reshape(-1, 4)
        r, g, b, a = arr[:, 0], arr[:, 1], arr[:, 2], arr[:, 3]
        if drop_background:
            is_white = (
                (r >= _WHITE_BG_THRESHOLD)
                & (g >= _WHITE_BG_THRESHOLD)
                & (b >= _WHITE_BG_THRESHOLD)
            )
            keep = (a >= _ALPHA_BG_THRESHOLD) & ~is_white
            r, g, b = r[keep], g[keep], b[keep]
        if r.size == 0:
            return []
        packed = (
            (r.astype(np.uint32) << 16)
            | (g.astype(np.uint32) << 8)
            | b.astype(np.uint32)
        )
        values, counts = np.unique(packed, return_counts=True)
        order = np.argsort(counts)[::-1][:n]
        total = int(counts.sum())
        out = []
        for idx in order:
            v = int(values[idx])
            rgb = ((v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF)
            out.append(
                {
                    "hex": _rgb_to_hex(rgb),
                    "rgb": list(rgb),
                    "fraction": round(int(counts[idx]) / total, 4),
                }
            )
        return out
    except ImportError:
        from collections import Counter

        kept = [
            (r, g, b)
            for (r, g, b, a) in _flat_pixels(rgba)
            if not (drop_background and _is_background(r, g, b, a))
        ]
        if not kept:
            return []
        counter = Counter(kept)
        total = sum(counter.values())
        return [
            {
                "hex": _rgb_to_hex(rgb),
                "rgb": list(rgb),
                "fraction": round(count / total, 4),
            }
            for rgb, count in counter.most_common(n)
        ]


def mean_color(img: Any, *, drop_background: bool = True) -> dict[str, Any]:
    """Return the mean subject colour of *img* as ``{"hex", "rgb"}``."""
    rgba = img.convert("RGBA")
    try:
        import numpy as np

        arr = np.asarray(rgba).reshape(-1, 4).astype(np.int64)
        r, g, b, a = arr[:, 0], arr[:, 1], arr[:, 2], arr[:, 3]
        if drop_background:
            is_white = (
                (r >= _WHITE_BG_THRESHOLD)
                & (g >= _WHITE_BG_THRESHOLD)
                & (b >= _WHITE_BG_THRESHOLD)
            )
            keep = (a >= _ALPHA_BG_THRESHOLD) & ~is_white
            arr = arr[keep]
        if arr.shape[0] == 0:
            return {"hex": "#000000", "rgb": [0, 0, 0]}
        rgb = tuple(int(c) for c in arr[:, :3].mean(axis=0).round())
        return {"hex": _rgb_to_hex(rgb), "rgb": list(rgb)}
    except ImportError:
        total = [0, 0, 0]
        kept = 0
        for r, g, b, a in _flat_pixels(rgba):
            if drop_background and _is_background(r, g, b, a):
                continue
            total[0] += r
            total[1] += g
            total[2] += b
            kept += 1
        if kept == 0:
            return {"hex": "#000000", "rgb": [0, 0, 0]}
        rgb = tuple(c // kept for c in total)
        return {"hex": _rgb_to_hex(rgb), "rgb": list(rgb)}


def analyze_image(path: str | Path, *, palette_size: int = 8) -> dict[str, Any]:
    """Produce a full technical fingerprint of the image at *path*.

    Args:
        path: Path to an image file.
        palette_size: Number of dominant colours to extract.

    Returns:
        A JSON-serialisable report: format/mode/size/megapixels, animation
        (frames/duration/loop), alpha presence, file size, the subject's tight
        bounding box and coverage, the dominant-colour palette, the mean colour,
        and any EXIF/DPI metadata.

    Raises:
        RuntimeError: If Pillow is not installed.
    """
    image_mod = _require_pillow()
    p = Path(path)
    img = image_mod.open(p)

    width, height = img.size
    has_alpha = img.mode in ("RGBA", "LA", "PA") or "transparency" in img.info
    n_frames = getattr(img, "n_frames", 1)

    # The subject bbox: tight box of non-background pixels.
    rgba = img.convert("RGBA")
    alpha = rgba.getchannel("A")
    bbox = alpha.getbbox() if has_alpha else img.convert("RGB").getbbox()
    subject = None
    if bbox:
        bw, bh = bbox[2] - bbox[0], bbox[3] - bbox[1]
        subject = {
            "bbox": list(bbox),
            "width": bw,
            "height": bh,
            "coverage_fraction": round((bw * bh) / (width * height), 4),
        }

    report: dict[str, Any] = {
        "path": str(p),
        "file_size_bytes": p.stat().st_size,
        "format": img.format,
        "mode": img.mode,
        "size": [width, height],
        "megapixels": round(width * height / 1_000_000, 3),
        "aspect_ratio": round(width / height, 4) if height else None,
        "has_alpha": bool(has_alpha),
        "animated": bool(getattr(img, "is_animated", False)),
        "n_frames": int(n_frames),
        "duration_ms": img.info.get("duration"),
        "loop": img.info.get("loop"),
        "dpi": list(img.info["dpi"]) if "dpi" in img.info else None,
        "subject": subject,
        "dominant_colors": dominant_colors(img, n=palette_size),
        "mean_color": mean_color(img),
    }
    exif = getattr(img, "getexif", lambda: {})()
    if exif:
        report["exif_tags"] = len(dict(exif))
    return report


def save_palette_swatch(
    colors: list[dict[str, Any]],
    out: str | Path,
    *,
    swatch: int = 64,
) -> Path:
    """Render a horizontal palette strip from *colors* to a PNG.

    Args:
        colors: A list of ``{"rgb": [r, g, b], ...}`` dicts (from
            :func:`dominant_colors`).
        out: Destination PNG path.
        swatch: Square edge of each colour cell in pixels.

    Returns:
        The written ``Path``.

    Raises:
        RuntimeError: If Pillow is not installed.
        ValueError: If *colors* is empty.
    """
    image_mod = _require_pillow()
    if not colors:
        raise ValueError("no colors to render")
    strip = image_mod.new(
        "RGB", (swatch * len(colors), swatch), (255, 255, 255)
    )
    for i, color in enumerate(colors):
        cell = image_mod.new("RGB", (swatch, swatch), tuple(color["rgb"]))
        strip.paste(cell, (i * swatch, 0))
    dest = Path(out)
    dest.parent.mkdir(parents=True, exist_ok=True)
    strip.save(dest, format="PNG")
    return dest
