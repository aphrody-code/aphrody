# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.icons` (pure logic + gated encoder paths)."""

from __future__ import annotations

import pytest

from aphrody import icons


def test_style_tables_are_consistent() -> None:
    assert set(icons.STYLE_FOLDERS) == set(icons.M3_STYLES)
    assert set(icons.CSS_CLASSES) == set(icons.M3_STYLES)
    assert icons.STYLE_FOLDERS["outlined"] == "materialsymbolsoutlined"


def test_build_icon_prompt_has_m3_markers() -> None:
    prompt = icons.build_icon_prompt("rocket launch", style="rounded")
    assert "Material Symbols Rounded-style" in prompt
    assert "24dp grid" in prompt
    assert "2dp" in prompt
    assert "no 3D" in prompt and "no gradient" in prompt
    assert "rocket launch" in prompt


def test_build_icon_prompt_fill_and_color() -> None:
    p = icons.build_icon_prompt("home", color="#FF0000", fill=True)
    assert "#FF0000" in p
    assert "FILL 1" in p


def test_build_icon_prompt_bad_style() -> None:
    with pytest.raises(ValueError, match="invalid"):
        icons.build_icon_prompt("home", style="neon")


def test_material_symbols_css() -> None:
    css = icons.material_symbols_css("sharp")
    assert ".material-symbols-sharp" in css
    assert "font-variation-settings" in css
    assert "Material Symbols Sharp" in css


def test_material_symbols_css_bad_style() -> None:
    with pytest.raises(ValueError, match="invalid"):
        icons.material_symbols_css("nope")


def test_coerce_bytes() -> None:
    assert icons._coerce_bytes(b"abc") == b"abc"
    assert icons._coerce_bytes(bytearray(b"xy")) == b"xy"
    assert icons._coerce_bytes([104, 105]) == b"hi"


def test_source_to_png_passthrough_bytes() -> None:
    assert icons._source_to_png(b"raw", size=64, color=None) == b"raw"


def test_catalogue_symbols_synthetic_tree(tmp_path) -> None:
    # Build a minimal symbols/web/<name>/<style_folder>/<name>_24px.svg tree.
    web = tmp_path / "symbols" / "web"
    for name in ("home", "settings"):
        for style, folder in icons.STYLE_FOLDERS.items():
            d = web / name / folder
            d.mkdir(parents=True)
            (d / f"{name}_24px.svg").write_text(
                '<svg xmlns="http://www.w3.org/2000/svg"/>', encoding="utf-8"
            )
    cat = icons.catalogue_symbols(tmp_path)
    assert set(cat) == {"home", "settings"}
    assert set(cat["home"].styles) == set(icons.M3_STYLES)
    assert cat["home"].styles["outlined"].name == "home_24px.svg"


def test_catalogue_symbols_missing_layout(tmp_path) -> None:
    assert icons.catalogue_symbols(tmp_path) == {}


_TINY_SVG = (
    '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" '
    'viewBox="0 0 24 24"><path d="M4 4h16v16H4z"/></svg>'
)


def test_svg_to_png_via_resvg() -> None:
    pytest.importorskip("resvg_py")
    png = icons.svg_to_png(_TINY_SVG, size=64)
    assert isinstance(png, bytes)
    assert png[:8] == b"\x89PNG\r\n\x1a\n"


def test_png_to_ico_roundtrip() -> None:
    image_mod = pytest.importorskip("PIL.Image")
    import io

    img = image_mod.new("RGBA", (256, 256), (0, 120, 215, 255))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    ico = icons.png_to_ico(buf.getvalue(), sizes=(16, 32, 48, 256))
    assert ico[:4] == b"\x00\x00\x01\x00"  # ICO magic
    # Pillow can read it back and enumerate the embedded sizes.
    reopened = image_mod.open(io.BytesIO(ico))
    assert (256, 256) in reopened.ico.sizes()


def test_apply_folder_icon_writes_desktop_ini(tmp_path, monkeypatch) -> None:
    import subprocess

    # Neutralise the Windows `attrib` calls so the tmp dir stays cleanable.
    monkeypatch.setattr(subprocess, "run", lambda *a, **k: None)
    ico = tmp_path / "x.ico"
    ico.write_bytes(b"\x00\x00\x01\x00")
    ini = icons.apply_folder_icon(tmp_path, ico)
    assert ini.exists()
    content = ini.read_text(encoding="utf-8")
    assert "IconResource=" in content
    assert "x.ico" in content


def test_apply_folder_icon_bad_dir(tmp_path) -> None:
    f = tmp_path / "afile.txt"
    f.write_text("x", encoding="utf-8")
    with pytest.raises(NotADirectoryError):
        icons.apply_folder_icon(f, f)
