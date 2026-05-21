# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.optimize`.

Pure-logic tests run unconditionally; encoder tests are skipped unless the
optional ``aphrody[images]`` dependencies (oxipng / Pillow) are installed.
"""

from __future__ import annotations

import io

import pytest
from aphrody import optimize


def test_optimize_result_ratio_and_summary() -> None:
    res = optimize.OptimizeResult(
        original_size=1000, outputs={"png": b"x" * 400, "webp": b"y" * 250}
    )
    assert res.ratio("png") == 0.4
    assert res.ratio("webp") == 0.25
    summary = res.summary()
    assert "src=1000B" in summary
    assert "png=400B(-60%)" in summary


def test_optimize_result_zero_size() -> None:
    res = optimize.OptimizeResult(original_size=0, outputs={"png": b""})
    assert res.ratio("png") == 1.0


def test_optimize_png_rejects_bad_level() -> None:
    # Level validation happens before any import, so no oxipng needed.
    with pytest.raises(ValueError, match="level must be 0-6"):
        optimize.optimize_png(b"data", level=9)


def _tiny_png() -> bytes:
    """Build a minimal valid PNG via Pillow, or skip the calling test."""
    image_mod = pytest.importorskip("PIL.Image")
    img = image_mod.new("RGBA", (8, 8), (255, 0, 0, 255))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def test_optimize_png_roundtrip() -> None:
    pytest.importorskip("oxipng")
    png = _tiny_png()
    out = optimize.optimize_png(png, level=2)
    assert isinstance(out, bytes)
    assert out[:8] == b"\x89PNG\r\n\x1a\n"  # still a PNG


def test_to_webp_roundtrip() -> None:
    image_mod = pytest.importorskip("PIL.Image")
    png = _tiny_png()
    webp = optimize.to_webp(png, quality=80)
    assert webp[:4] == b"RIFF"
    # Decodes back to an image of the same size.
    assert image_mod.open(io.BytesIO(webp)).size == (8, 8)


def test_optimize_all_collects_requested_formats() -> None:
    pytest.importorskip("oxipng")
    pytest.importorskip("PIL.Image")
    png = _tiny_png()
    res = optimize.optimize_all(png, png=True, webp=True, avif=False)
    assert "png" in res.outputs
    assert "webp" in res.outputs
    assert "avif" not in res.outputs
    assert res.original_size == len(png)
