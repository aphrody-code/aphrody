# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.to3d` (relief backend gated on trimesh/numpy)."""

from __future__ import annotations

import pytest

from aphrody import to3d


def test_backends_constant() -> None:
    assert to3d.BACKENDS == ("relief", "depth")


def test_image_to_mesh_unknown_method(tmp_path) -> None:
    with pytest.raises(ValueError, match="unknown method"):
        to3d.image_to_mesh(b"x", tmp_path / "m.glb", method="nope")


def _rgba_png(tmp_path, name="s.png", size=(12, 12)):
    image_mod = pytest.importorskip("PIL.Image")
    img = image_mod.new("RGBA", size, (0, 0, 0, 0))
    # A filled opaque square in the centre = the subject silhouette.
    for y in range(3, size[1] - 3):
        for x in range(3, size[0] - 3):
            img.putpixel((x, y), (200, 120, 40, 255))
    p = tmp_path / name
    img.save(p)
    return p


def test_load_rgba_and_mask(tmp_path) -> None:
    pytest.importorskip("numpy")
    pytest.importorskip("PIL.Image")
    p = _rgba_png(tmp_path)
    rgb, _alpha, mask = to3d._load_rgba_and_mask(p, max_dim=12)
    assert rgb.shape[2] == 3
    assert mask.dtype == bool
    assert mask.any()  # the central square is subject
    assert not mask[0, 0]  # corner is transparent background


def test_heightmap_to_glb(tmp_path) -> None:
    np = pytest.importorskip("numpy")
    pytest.importorskip("trimesh")
    mask = np.ones((4, 4), dtype=bool)
    depth = np.linspace(0, 1, 16, dtype=np.float32).reshape(4, 4)
    rgb = np.full((4, 4, 3), 128, dtype=np.uint8)
    out = to3d._heightmap_to_glb(
        depth, rgb, mask, tmp_path / "m.glb", depth_scale=0.2
    )
    assert out.exists()
    assert out.read_bytes()[:4] == b"glTF"  # glb container magic


def test_heightmap_empty_mask_raises(tmp_path) -> None:
    np = pytest.importorskip("numpy")
    pytest.importorskip("trimesh")
    mask = np.zeros((4, 4), dtype=bool)
    depth = np.zeros((4, 4), dtype=np.float32)
    rgb = np.zeros((4, 4, 3), dtype=np.uint8)
    with pytest.raises(ValueError, match="empty subject mask"):
        to3d._heightmap_to_glb(
            depth, rgb, mask, tmp_path / "m.glb", depth_scale=0.2
        )


def test_image_to_mesh_relief_end_to_end(tmp_path) -> None:
    pytest.importorskip("numpy")
    pytest.importorskip("trimesh")
    pytest.importorskip("PIL.Image")
    p = _rgba_png(tmp_path, size=(20, 20))
    out = to3d.image_to_mesh(
        p, tmp_path / "relief.glb", method="relief", max_dim=20
    )
    assert out.exists()
    assert out.read_bytes()[:4] == b"glTF"


def test_image_to_mesh_textured(tmp_path) -> None:
    pytest.importorskip("numpy")
    pytest.importorskip("trimesh")
    pytest.importorskip("PIL.Image")
    p = _rgba_png(tmp_path, size=(24, 24))
    out = to3d.image_to_mesh(
        p, tmp_path / "tex.glb", method="relief", max_dim=24, texture=True
    )
    assert out.exists()
    assert out.read_bytes()[:4] == b"glTF"  # textured glb container


def test_estimate_depth_mocked(tmp_path, monkeypatch) -> None:
    np = pytest.importorskip("numpy")
    pytest.importorskip("PIL.Image")
    import importlib

    p = _rgba_png(tmp_path, size=(16, 16))

    class _FakePipe:
        def __call__(self, _pil):
            return {"depth": np.full((16, 16), 0.5, dtype=np.float32)}

    fake_transformers = type(
        "_M", (), {"pipeline": staticmethod(lambda *a, **k: _FakePipe())}
    )

    import types

    fake_torch = types.SimpleNamespace(
        cuda=types.SimpleNamespace(is_available=lambda: False)
    )

    def fake_require(module, extra):
        if module == "transformers":
            return fake_transformers
        if module == "torch":
            return fake_torch
        return importlib.import_module(module)

    monkeypatch.setattr(to3d, "_require", fake_require)
    depth, _rgb, mask = to3d.estimate_depth(p, max_dim=16)
    assert depth.shape == mask.shape
    assert depth.max() <= 1.0
    assert (depth[~mask] == 0.0).all()  # background flattened
