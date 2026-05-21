# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""2D -> 3D — turn an image into a vertex-coloured ``.glb`` mesh.

Two backends, both converging on a vectorised heightmap-to-mesh builder:

* **relief** (default, no GPU, no ML): derive a pseudo-depth from the subject
  silhouette (distance-transform "inflation" via SciPy, or a blurred-alpha
  fallback) modulated by luminance, then displace a grid. Great for stickers /
  flat characters; needs only ``aphrody[to3d]`` (trimesh + numpy [+ scipy]).
* **depth**: monocular depth from **Depth Anything V2** (``transformers`` +
  ``torch``, CPU-capable), then the same grid displacement. Needs
  ``aphrody[depth]``.

Output is a ``.glb`` (glTF binary) viewable in Blender, three.js, or the
Windows 3D Viewer.

    >>> from aphrody.to3d import image_to_mesh
    >>> image_to_mesh("assets/aphrody_body_r0.webp", "aphrody.glb")   # doctest: +SKIP
"""

from __future__ import annotations

import logging
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)

#: Default Depth Anything V2 checkpoint for the ``depth`` backend.
DEFAULT_DEPTH_MODEL = "depth-anything/Depth-Anything-V2-Small-hf"
#: Backends understood by :func:`image_to_mesh`.
BACKENDS: tuple[str, ...] = ("relief", "depth")
#: Default longest-edge the mesh grid is downsampled to (vertex budget).
DEFAULT_MAX_DIM = 200
#: Near-white background cutoff (per channel) when there is no alpha.
_WHITE_BG = 244
#: Alpha cutoff below which a pixel is background.
_ALPHA_BG = 16


def _require(module: str, extra: str) -> Any:
    """Import *module* or raise a RuntimeError pointing at the right extra."""
    import importlib

    try:
        return importlib.import_module(module)
    except ImportError as exc:  # pragma: no cover - depends on env
        raise RuntimeError(
            f"{module} is required; install: uv pip install 'aphrody[{extra}]'"
        ) from exc


def _load_rgba_and_mask(
    image: str | Path | bytes, *, max_dim: int
) -> tuple[Any, Any, Any]:
    """Load *image*, downsample to *max_dim*, return ``(rgb, alpha, mask)``.

    Args:
        image: Path or raw bytes.
        max_dim: Longest edge after downsampling.

    Returns:
        ``(rgb, alpha, mask)`` numpy arrays — ``rgb`` is ``HxWx3`` uint8,
        ``alpha`` is ``HxW`` uint8, ``mask`` is ``HxW`` bool (subject pixels).
    """
    import io

    np = _require("numpy", "to3d")
    image_mod = _require("PIL.Image", "images")

    if isinstance(image, bytes):
        img = image_mod.open(io.BytesIO(image))
    else:
        img = image_mod.open(image)
    img = img.convert("RGBA")

    # Downsample so the longest edge == max_dim (keep aspect).
    w, h = img.size
    scale = max_dim / max(w, h)
    if scale < 1.0:
        img = img.resize(
            (max(1, round(w * scale)), max(1, round(h * scale))),
            image_mod.Resampling.LANCZOS,
        )

    arr = np.asarray(img)
    rgb = arr[:, :, :3]
    alpha = arr[:, :, 3]
    has_alpha = bool((alpha < 255).any())
    if has_alpha:
        mask = alpha >= _ALPHA_BG
    else:
        is_white = (rgb >= _WHITE_BG).all(axis=2)
        mask = ~is_white
    return rgb, alpha, mask


def _relief_depth(mask: Any, rgb: Any) -> Any:
    """Build a pseudo-depth (HxW float in [0,1]) from a silhouette *mask*.

    Uses a distance transform ("inflation" — interior points bulge outward) when
    SciPy is available, else a blurred-alpha fallback, then darkens recessed
    (low-luminance) interior pixels slightly so shading reads as relief.
    """
    np = _require("numpy", "to3d")

    try:
        from scipy.ndimage import distance_transform_edt

        dist = distance_transform_edt(mask)
        peak = dist.max()
        inflate = np.sqrt(dist / peak) if peak > 0 else dist
    except ImportError:  # pragma: no cover - scipy is in the to3d extra
        image_mod = _require("PIL.Image", "images")
        from PIL import ImageFilter

        m = image_mod.fromarray((mask * 255).astype(np.uint8))
        radius = max(2, min(mask.shape) // 16)
        blurred = np.asarray(m.filter(ImageFilter.GaussianBlur(radius)))
        inflate = blurred.astype(np.float32) / 255.0

    # Luminance relief: brighter surfaces sit slightly proud.
    lum = (
        0.299 * rgb[:, :, 0] + 0.587 * rgb[:, :, 1] + 0.114 * rgb[:, :, 2]
    ) / 255.0
    depth = 0.8 * inflate + 0.2 * (lum * (mask.astype(np.float32)))
    depth[~mask] = 0.0
    peak = depth.max()
    return depth / peak if peak > 0 else depth


def estimate_depth(
    image: str | Path | bytes,
    *,
    model: str = DEFAULT_DEPTH_MODEL,
    max_dim: int = DEFAULT_MAX_DIM,
) -> tuple[Any, Any, Any]:
    """Estimate monocular depth with Depth Anything V2.

    Args:
        image: Path or raw bytes.
        model: Hugging Face checkpoint id.
        max_dim: Longest edge of the returned grids.

    Returns:
        ``(depth, rgb, mask)`` — *depth* is ``HxW`` float in [0,1] (1 = nearest).

    Raises:
        RuntimeError: If ``transformers``/``torch`` are not installed.
    """
    np = _require("numpy", "to3d")
    transformers = _require("transformers", "depth")
    torch = _require("torch", "depth")

    rgb, _alpha, mask = _load_rgba_and_mask(image, max_dim=max_dim)
    image_mod = _require("PIL.Image", "images")
    pil = image_mod.fromarray(rgb)

    # Exploit the NVIDIA GPU when a CUDA-enabled torch is installed; else CPU.
    device = 0 if torch.cuda.is_available() else -1
    logger.info("Depth Anything device=%s", "cuda" if device == 0 else "cpu")
    pipe = transformers.pipeline("depth-estimation", model=model, device=device)
    out = pipe(pil)
    depth = np.asarray(out["depth"], dtype=np.float32)
    if depth.shape != mask.shape:  # pragma: no cover - pipeline may resize
        depth = np.asarray(
            image_mod.fromarray(depth).resize(
                (mask.shape[1], mask.shape[0]), image_mod.Resampling.BILINEAR
            ),
            dtype=np.float32,
        )
    peak = depth.max()
    depth = depth / peak if peak > 0 else depth
    depth[~mask] = 0.0
    return depth, rgb, mask


def _heightmap_to_glb(
    depth: Any,
    rgb: Any,
    mask: Any,
    out: str | Path,
    *,
    depth_scale: float,
    texture_image: Any = None,
) -> Path:
    """Displace a grid by *depth* and export a ``.glb``.

    Only in-*mask* pixels become vertices; adjacent in-mask quads become two
    triangles each. Fully vectorised with NumPy. When *texture_image* is given
    the mesh is UV-mapped and textured with that image (a "complete" textured
    model); otherwise per-vertex colours are baked.

    Args:
        depth: ``HxW`` float in [0,1].
        rgb: ``HxWx3`` uint8 colours.
        mask: ``HxW`` bool subject mask.
        out: Destination ``.glb`` path.
        depth_scale: Z displacement as a fraction of the longest edge.
        texture_image: Optional Pillow image to map as a UV texture.

    Returns:
        The written ``.glb`` ``Path``.

    Raises:
        ValueError: If the mask is empty.
        RuntimeError: If trimesh is not installed.
    """
    np = _require("numpy", "to3d")
    trimesh = _require("trimesh", "to3d")

    h, w = mask.shape
    ys, xs = np.nonzero(mask)
    if ys.size == 0:
        raise ValueError("empty subject mask: nothing to turn into 3D")

    vid = -np.ones((h, w), dtype=np.int64)
    vid[ys, xs] = np.arange(ys.size)

    z = depth[ys, xs] * (depth_scale * max(h, w))
    # x right, y up (flip rows), z out of the plane; centre the model on origin.
    verts = np.column_stack([xs - w / 2.0, (h - 1 - ys) - h / 2.0, z]).astype(
        np.float32
    )

    a = vid[:-1, :-1]
    b = vid[:-1, 1:]
    c = vid[1:, :-1]
    d = vid[1:, 1:]
    tri1 = np.stack([a, c, b], axis=-1).reshape(-1, 3)
    ok1 = ((a >= 0) & (b >= 0) & (c >= 0)).reshape(-1)
    tri2 = np.stack([b, c, d], axis=-1).reshape(-1, 3)
    ok2 = ((b >= 0) & (c >= 0) & (d >= 0)).reshape(-1)
    faces = np.vstack([tri1[ok1], tri2[ok2]])

    if texture_image is not None:
        from trimesh.visual import TextureVisuals
        from trimesh.visual.material import PBRMaterial

        denom_w = max(w - 1, 1)
        denom_h = max(h - 1, 1)
        uv = np.column_stack([xs / denom_w, 1.0 - ys / denom_h]).astype(
            np.float32
        )
        material = PBRMaterial(
            baseColorTexture=texture_image,
            alphaMode="BLEND",
            doubleSided=True,
        )
        visual = TextureVisuals(uv=uv, image=texture_image, material=material)
        mesh = trimesh.Trimesh(
            vertices=verts, faces=faces, visual=visual, process=False
        )
    else:
        colors = np.column_stack(
            [rgb[ys, xs], np.full(ys.size, 255, dtype=np.uint8)]
        ).astype(np.uint8)
        mesh = trimesh.Trimesh(
            vertices=verts, faces=faces, vertex_colors=colors, process=False
        )
    dest = Path(out)
    dest.parent.mkdir(parents=True, exist_ok=True)
    mesh.export(dest, file_type="glb")
    logger.info(
        "wrote mesh (%d verts, %d faces) -> %s",
        len(verts),
        len(faces),
        dest,
    )
    return dest


def image_to_mesh(
    image: str | Path | bytes,
    out: str | Path,
    *,
    method: str = "relief",
    depth_scale: float = 0.15,
    max_dim: int = DEFAULT_MAX_DIM,
    model: str = DEFAULT_DEPTH_MODEL,
    texture: bool = False,
) -> Path:
    """Convert *image* into a ``.glb`` mesh (vertex-coloured or textured).

    Args:
        image: Source image (path or bytes); transparency or a near-white
            background defines the subject silhouette.
        out: Destination ``.glb`` path.
        method: ``"relief"`` (no GPU/ML) or ``"depth"`` (Depth Anything V2).
        depth_scale: Z displacement as a fraction of the longest edge.
        max_dim: Longest grid edge (vertex budget).
        model: Depth Anything checkpoint (``method="depth"`` only).
        texture: When ``True`` the sprite is UV-mapped as a full-image texture
            (a "complete" textured model) instead of baked per-vertex colours.

    Returns:
        The written ``.glb`` ``Path``.

    Raises:
        ValueError: For an unknown *method* or an empty subject.
        RuntimeError: If the backend's dependencies are missing.
    """
    if method not in BACKENDS:
        raise ValueError(
            f"unknown method {method!r}; choose {', '.join(BACKENDS)}"
        )
    alpha: Any = None
    if method == "depth":
        depth, rgb, mask = estimate_depth(image, model=model, max_dim=max_dim)
    else:
        rgb, alpha, mask = _load_rgba_and_mask(image, max_dim=max_dim)
        depth = _relief_depth(mask, rgb)

    texture_image = None
    if texture:
        np = _require("numpy", "to3d")
        image_mod = _require("PIL.Image", "images")
        if alpha is None:
            alpha = np.full(mask.shape, 255, dtype=np.uint8)
        rgba = np.dstack([rgb, alpha]).astype(np.uint8)
        texture_image = image_mod.fromarray(rgba, "RGBA")

    return _heightmap_to_glb(
        depth,
        rgb,
        mask,
        out,
        depth_scale=depth_scale,
        texture_image=texture_image,
    )
