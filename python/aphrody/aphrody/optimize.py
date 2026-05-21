# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Image optimization for generated PNGs — lossless PNG + WebP/AVIF re-encode.

Nano Banana Pro returns large PNG bytes. This module shrinks them with two
best-in-class tools, both optional (install via the ``aphrody[images]`` extra):

* **pyoxipng** — Rust ``oxipng`` core, multithreaded lossless PNG optimisation.
* **Pillow** (>= 12) — native WebP and AVIF re-encoding (no plugin required).

Everything operates on ``bytes`` in memory so it composes directly with the
generation path, with thin file helpers on top.

    >>> from aphrody.optimize import optimize_png, to_webp, to_avif
    >>> small = optimize_png(png_bytes, level=6)          # doctest: +SKIP
    >>> webp = to_webp(png_bytes, quality=82)             # doctest: +SKIP
    >>> avif = to_avif(png_bytes, quality=70)             # doctest: +SKIP
"""

from __future__ import annotations

import io
import logging
from dataclasses import dataclass
from pathlib import Path

logger = logging.getLogger(__name__)

#: Default oxipng optimisation level (0-6; 6 is the most thorough).
DEFAULT_PNG_LEVEL = 6
#: Default lossy WebP quality (0-100).
DEFAULT_WEBP_QUALITY = 82
#: Default lossy AVIF quality (0-100).
DEFAULT_AVIF_QUALITY = 70

_INSTALL_HINT = (
    "install the image-optimization extra: "
    "uv pip install 'aphrody[images]'  (or: pip install pyoxipng Pillow)"
)


def _require_oxipng() -> object:
    """Import and return the ``oxipng`` module or raise a helpful error.

    Returns:
        The imported ``oxipng`` module.

    Raises:
        RuntimeError: If ``pyoxipng`` is not installed.
    """
    try:
        import oxipng
    except ImportError as exc:  # pragma: no cover - depends on env
        raise RuntimeError(
            f"pyoxipng is required for PNG optimisation; {_INSTALL_HINT}"
        ) from exc
    return oxipng


def _require_pillow() -> object:
    """Import and return the ``PIL.Image`` module or raise a helpful error.

    Returns:
        The imported ``PIL.Image`` module.

    Raises:
        RuntimeError: If Pillow is not installed.
    """
    try:
        from PIL import Image
    except ImportError as exc:  # pragma: no cover - depends on env
        raise RuntimeError(
            f"Pillow is required for WebP/AVIF re-encoding; {_INSTALL_HINT}"
        ) from exc
    return Image


def optimize_png(
    data: bytes, *, level: int = DEFAULT_PNG_LEVEL, strip: bool = True
) -> bytes:
    """Losslessly optimise PNG *data* with oxipng.

    Args:
        data: Raw PNG bytes.
        level: oxipng effort level, 0-6 (default 6).
        strip: When ``True``, strip safe-to-remove metadata chunks.

    Returns:
        Optimised PNG bytes. If optimisation somehow yields larger output the
        original *data* is returned unchanged.

    Raises:
        RuntimeError: If ``pyoxipng`` is not installed.
        ValueError: If *level* is outside 0-6.
    """
    if not 0 <= level <= 6:
        raise ValueError(f"oxipng level must be 0-6, got {level}")
    oxipng = _require_oxipng()
    kwargs: dict = {"level": level, "optimize_alpha": True}
    if strip:
        kwargs["strip"] = oxipng.StripChunks.safe()  # type: ignore[attr-defined]
    out: bytes = oxipng.optimize_from_memory(data, **kwargs)  # type: ignore[attr-defined]
    return out if len(out) < len(data) else data


def _open_image(data: bytes) -> object:
    """Decode *data* into a loaded Pillow ``Image``."""
    image_mod = _require_pillow()
    img = image_mod.open(io.BytesIO(data))  # type: ignore[attr-defined]
    img.load()
    return img


def to_webp(
    data: bytes,
    *,
    lossless: bool = False,
    quality: int = DEFAULT_WEBP_QUALITY,
    method: int = 6,
) -> bytes:
    """Re-encode image *data* as WebP.

    Args:
        data: Raw image bytes (any Pillow-readable format, e.g. PNG).
        lossless: Use lossless WebP (typically ~26% smaller than PNG).
        quality: 0-100. For lossy this is visual fidelity; for lossless it
            trades encode time against size.
        method: Encoder effort 0 (fast) - 6 (slowest, best).

    Returns:
        WebP-encoded bytes.

    Raises:
        RuntimeError: If Pillow is not installed.
    """
    img = _open_image(data)
    buf = io.BytesIO()
    img.save(
        buf, format="WEBP", lossless=lossless, quality=quality, method=method
    )  # type: ignore[attr-defined]
    return buf.getvalue()


def to_avif(
    data: bytes,
    *,
    quality: int = DEFAULT_AVIF_QUALITY,
    speed: int = 6,
) -> bytes:
    """Re-encode image *data* as AVIF (native in Pillow >= 12).

    Args:
        data: Raw image bytes.
        quality: 0-100. ``100`` selects the lossless path.
        speed: libaom encoder speed 0 (slow/best) - 10 (fast).

    Returns:
        AVIF-encoded bytes.

    Raises:
        RuntimeError: If Pillow (or its AVIF support) is not available.
    """
    img = _open_image(data)
    buf = io.BytesIO()
    try:
        img.save(buf, format="AVIF", quality=quality, speed=speed)  # type: ignore[attr-defined]
    except (KeyError, OSError) as exc:  # pragma: no cover - depends on libavif
        raise RuntimeError(
            "AVIF encoding unavailable: Pillow >= 12 with libavif is required "
            f"(or install pillow-avif-plugin). {_INSTALL_HINT}"
        ) from exc
    return buf.getvalue()


@dataclass(frozen=True)
class OptimizeResult:
    """Outcome of optimising a single source image into one or more formats.

    Attributes:
        original_size: Byte length of the source image.
        outputs: Mapping of format name (``"png"``/``"webp"``/``"avif"``) to the
            encoded bytes produced for that format.
    """

    original_size: int
    outputs: dict[str, bytes]

    def ratio(self, fmt: str) -> float:
        """Return the size ratio (output / original) for *fmt*, lower is better."""
        if self.original_size == 0:
            return 1.0
        return len(self.outputs[fmt]) / self.original_size

    def summary(self) -> str:
        """Return a one-line human-readable size-saving summary."""
        bits = [f"src={self.original_size}B"]
        for fmt, blob in self.outputs.items():
            pct = 100.0 * (1.0 - len(blob) / max(self.original_size, 1))
            bits.append(f"{fmt}={len(blob)}B(-{pct:.0f}%)")
        return " ".join(bits)


def optimize_all(
    data: bytes,
    *,
    png: bool = True,
    webp: bool = True,
    avif: bool = False,
    png_level: int = DEFAULT_PNG_LEVEL,
    webp_quality: int = DEFAULT_WEBP_QUALITY,
    avif_quality: int = DEFAULT_AVIF_QUALITY,
) -> OptimizeResult:
    """Produce optimised variants of a single source image.

    Each requested format is encoded independently; a failure in one optional
    encoder is logged and skipped rather than aborting the whole call.

    Args:
        data: Raw source image bytes (PNG from the generator).
        png: Emit a losslessly optimised PNG.
        webp: Emit a lossy WebP.
        avif: Emit a lossy AVIF (requires Pillow AVIF support).
        png_level: oxipng level for the PNG output.
        webp_quality: Quality for the WebP output.
        avif_quality: Quality for the AVIF output.

    Returns:
        An :class:`OptimizeResult` containing every successfully encoded format.
    """
    outputs: dict[str, bytes] = {}
    if png:
        try:
            outputs["png"] = optimize_png(data, level=png_level)
        except RuntimeError as exc:
            logger.warning("PNG optimisation skipped: %s", exc)
    if webp:
        try:
            outputs["webp"] = to_webp(data, quality=webp_quality)
        except RuntimeError as exc:
            logger.warning("WebP encoding skipped: %s", exc)
    if avif:
        try:
            outputs["avif"] = to_avif(data, quality=avif_quality)
        except RuntimeError as exc:
            logger.warning("AVIF encoding skipped: %s", exc)
    return OptimizeResult(original_size=len(data), outputs=outputs)


def optimize_file(
    path: str | Path,
    *,
    webp: bool = True,
    avif: bool = False,
    png_level: int = DEFAULT_PNG_LEVEL,
) -> OptimizeResult:
    """Optimise a PNG file in place and write sibling WebP/AVIF variants.

    The source PNG at *path* is replaced by its optimised form; ``.webp`` and
    ``.avif`` siblings are written next to it when requested.

    Args:
        path: Path to a PNG file on disk.
        webp: Also write a ``.webp`` sibling.
        avif: Also write an ``.avif`` sibling.
        png_level: oxipng level for the in-place PNG optimisation.

    Returns:
        The :class:`OptimizeResult` describing the produced files.
    """
    p = Path(path)
    data = p.read_bytes()
    result = optimize_all(
        data, png=True, webp=webp, avif=avif, png_level=png_level
    )
    if "png" in result.outputs:
        p.write_bytes(result.outputs["png"])
    if "webp" in result.outputs:
        p.with_suffix(".webp").write_bytes(result.outputs["webp"])
    if "avif" in result.outputs:
        p.with_suffix(".avif").write_bytes(result.outputs["avif"])
    logger.info("optimised %s: %s", p.name, result.summary())
    return result
