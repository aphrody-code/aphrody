# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.anim` (Pillow-gated for the encoders)."""

from __future__ import annotations

import pytest
from aphrody import anim


def test_duration_ms() -> None:
    assert anim._duration_ms(10) == 100
    assert anim._duration_ms(12) == 83
    with pytest.raises(ValueError, match="fps must be"):
        anim._duration_ms(0)


def test_pingpong_frames() -> None:
    assert anim.pingpong_frames([1, 2, 3, 4]) == [1, 2, 3, 4, 3, 2]
    assert anim.pingpong_frames([1, 2]) == [1, 2]  # too short to bounce


def test_sort_frames_by_index() -> None:
    paths = ["x_r10.webp", "x_r2.webp", "x_r0.webp", "noindex.webp"]
    out = [p.name for p in anim.sort_frames_by_index(paths)]
    assert out == ["x_r0.webp", "x_r2.webp", "x_r10.webp", "noindex.webp"]


def _frames(tmp_path, n=3):
    image_mod = pytest.importorskip("PIL.Image")
    paths = []
    for i in range(n):
        img = image_mod.new("RGBA", (16, 16), (i * 40, 0, 0, 255))
        p = tmp_path / f"f_r{i}.png"
        img.save(p)
        paths.append(str(p))
    return paths


def test_build_animation_webp(tmp_path) -> None:
    image_mod = pytest.importorskip("PIL.Image")
    out = anim.build_animation(_frames(tmp_path), tmp_path / "a.webp", fps=10)
    data = out.read_bytes()
    assert data[:4] == b"RIFF" and data[8:12] == b"WEBP"
    reopened = image_mod.open(out)
    assert getattr(reopened, "n_frames", 1) == 3


def test_build_animation_gif_and_apng(tmp_path) -> None:
    pytest.importorskip("PIL.Image")
    gif = anim.build_animation(_frames(tmp_path), tmp_path / "a.gif", fps=8)
    assert gif.read_bytes()[:4] == b"GIF8"
    apng = anim.build_animation(_frames(tmp_path), tmp_path / "a.apng", fps=8)
    assert apng.read_bytes()[:8] == b"\x89PNG\r\n\x1a\n"


def test_build_animation_pingpong_frame_count(tmp_path) -> None:
    image_mod = pytest.importorskip("PIL.Image")
    out = anim.build_animation(
        _frames(tmp_path, 4), tmp_path / "p.webp", fps=10, pingpong=True
    )
    # 4 frames -> ping-pong 4 + 2 = 6
    assert getattr(image_mod.open(out), "n_frames", 1) == 6


def test_build_animation_errors(tmp_path) -> None:
    with pytest.raises(ValueError, match="at least one frame"):
        anim.build_animation([], tmp_path / "x.webp")
    with pytest.raises(ValueError, match="unsupported animation format"):
        anim.build_animation(_frames(tmp_path), tmp_path / "x.bmp", fmt="bmp")


def test_make_spritesheet(tmp_path) -> None:
    image_mod = pytest.importorskip("PIL.Image")
    sheet, atlas = anim.make_spritesheet(
        _frames(tmp_path, 4), tmp_path / "sheet.png", columns=2
    )
    assert sheet.exists()
    assert atlas["columns"] == 2 and atlas["rows"] == 2
    assert atlas["count"] == 4
    assert image_mod.open(sheet).size == (32, 32)  # 2x2 of 16px cells
    assert (tmp_path / "sheet.json").exists()


def test_turntable_no_match(tmp_path) -> None:
    with pytest.raises(FileNotFoundError, match="no frames match"):
        anim.turntable(str(tmp_path / "nope_r*.webp"), tmp_path / "t.webp")
