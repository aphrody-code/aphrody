# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.analyze` (Pillow-gated)."""

from __future__ import annotations

import pytest


def _img(mod, size=(10, 10), color=(0, 120, 215, 255)):
    return mod.new("RGBA", size, color)


def test_analyze_image_basic(tmp_path) -> None:
    image_mod = pytest.importorskip("PIL.Image")
    p = tmp_path / "blue.png"
    _img(image_mod, (20, 10), (0, 120, 215, 255)).save(p)

    from aphrody import analyze

    report = analyze.analyze_image(p)
    assert report["size"] == [20, 10]
    assert report["mode"] == "RGBA"
    assert report["has_alpha"] is True
    assert report["aspect_ratio"] == 2.0
    assert report["format"] == "PNG"
    assert report["dominant_colors"][0]["hex"] == "#0078D7"
    assert report["mean_color"]["hex"] == "#0078D7"
    assert report["subject"]["coverage_fraction"] == 1.0


def test_dominant_colors_drops_background() -> None:
    image_mod = pytest.importorskip("PIL.Image")
    from aphrody import analyze

    # Half pure-white background, half red subject.
    img = image_mod.new("RGBA", (2, 1), (255, 255, 255, 255))
    img.putpixel((0, 0), (200, 10, 10, 255))
    colors = analyze.dominant_colors(img, drop_background=True)
    assert colors[0]["hex"] == "#C80A0A"
    assert all(c["hex"] != "#FFFFFF" for c in colors)


def test_dominant_colors_keeps_background_when_asked() -> None:
    image_mod = pytest.importorskip("PIL.Image")
    from aphrody import analyze

    img = image_mod.new("RGBA", (4, 1), (255, 255, 255, 255))
    colors = analyze.dominant_colors(img, drop_background=False)
    assert colors[0]["hex"] == "#FFFFFF"


def test_save_palette_swatch(tmp_path) -> None:
    image_mod = pytest.importorskip("PIL.Image")
    from aphrody import analyze

    colors = [{"rgb": [255, 0, 0]}, {"rgb": [0, 255, 0]}, {"rgb": [0, 0, 255]}]
    out = analyze.save_palette_swatch(colors, tmp_path / "pal.png", swatch=16)
    assert out.exists()
    assert image_mod.open(out).size == (48, 16)


def test_save_palette_swatch_empty(tmp_path) -> None:
    from aphrody import analyze

    with pytest.raises(ValueError, match="no colors"):
        analyze.save_palette_swatch([], tmp_path / "x.png")
