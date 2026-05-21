# SPDX-License-Identifier: Apache-2.0
"""Tests for the pure-logic helpers in :mod:`aphrody.images`."""

from __future__ import annotations

from aphrody import images


def test_resolve_outputs_single_file(tmp_path) -> None:
    out = tmp_path / "x.png"
    assert images._resolve_outputs(out, 1, "gen") == [out]


def test_resolve_outputs_none() -> None:
    assert images._resolve_outputs(None, 2, "gen") == [None, None]


def test_resolve_outputs_directory(tmp_path) -> None:
    paths = images._resolve_outputs(tmp_path, 2, "gen")
    assert len(paths) == 2
    assert all(str(p).endswith(".png") for p in paths)


def test_extract_images_empty() -> None:
    class _Resp:
        candidates: tuple = ()

    assert images._extract_images(_Resp()) == []


def test_extract_images() -> None:
    class _Inline:
        data = b"PNG"

    class _Part:
        inline_data = _Inline()

    class _Content:
        parts = (_Part(),)

    class _Cand:
        content = _Content()

    class _Resp:
        candidates = (_Cand(),)

    assert images._extract_images(_Resp()) == [b"PNG"]


def test_detect_mime() -> None:
    assert images._detect_mime(b"x") == "image/png"
    assert images._detect_mime("photo.jpg") == "image/jpeg"
