# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Material 3 icon tooling — generate, rasterise and package as Windows ``.ico``.

Three capabilities, all keyless and dependency-light:

1. **Generate** authentic Material 3 / Material Symbols icons on demand with
   Nano Banana Pro (:mod:`aphrody.images`) using prompts distilled from the M3
   design spec (24dp grid, 2dp stroke, outlined/rounded/sharp, single colour,
   transparent background, no 3D/shadow/gradient).
2. **Convert** existing Material Symbols SVGs — from a cloned
   ``material-design-icons`` checkout or fetched by name via Iconify — into
   losslessly-optimised PNGs and multi-resolution Windows ``.ico`` files.
3. **Catalogue** a local checkout: index icon names, styles and the canonical
   self-host CSS classes.

Optional deps live in the ``aphrody[icons]`` extra (resvg-py, scour, pyconify;
plus Pillow + pyoxipng from ``aphrody[images]``).

    >>> from aphrody.icons import generate_icon, make_windows_ico
    >>> png = generate_icon("rocket launch", style="rounded")      # doctest: +SKIP
    >>> make_windows_ico(png, "rocket.ico")                        # doctest: +SKIP
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from pathlib import Path

logger = logging.getLogger(__name__)

#: The three Material Symbols styles.
M3_STYLES: tuple[str, ...] = ("outlined", "rounded", "sharp")

#: Map of style -> the repo folder name under ``symbols/web/<name>/``.
STYLE_FOLDERS: dict[str, str] = {
    "outlined": "materialsymbolsoutlined",
    "rounded": "materialsymbolsrounded",
    "sharp": "materialsymbolssharp",
}

#: Canonical self-host CSS class per style (for the "useful CSS" catalogue).
CSS_CLASSES: dict[str, str] = {
    "outlined": "material-symbols-outlined",
    "rounded": "material-symbols-rounded",
    "sharp": "material-symbols-sharp",
}

#: Default multi-resolution sizes baked into a Windows ``.ico``.
DEFAULT_ICO_SIZES: tuple[int, ...] = (16, 24, 32, 48, 64, 128, 256)

#: Negative tokens appended to every generated-icon prompt.
_ICON_NEGATIVES = (
    "no 3D, no isometric, no perspective, no tilt, no shadow, no drop shadow, "
    "no gradient, no photorealism, no texture, no bevel, no glow, no reflection, "
    "no multiple colors, no background scene, no text, no labels"
)

# Per-style corner rule, straight from the M3 designing-icons spec.
_STYLE_CORNERS = {
    "outlined": (
        "exterior corners with 2dp radius and square (non-rounded) interior "
        "corners, squared flat stroke terminals"
    ),
    "rounded": (
        "both exterior and interior corners rounded with 2dp radius, soft "
        "rounded terminals"
    ),
    "sharp": (
        "all corners straight with 0dp radius (crisp square corners), squared "
        "flat stroke terminals"
    ),
}


def build_icon_prompt(
    subject: str,
    *,
    style: str = "outlined",
    color: str = "#1F1F1F",
    background: str = "transparent",
    fill: bool = False,
    weight: str = "regular",
) -> str:
    """Build a Material 3 icon prompt for an image model.

    Encodes the M3 geometry rules (24dp grid, 20dp live area, 2dp padding,
    2dp stroke, style-specific corners, forward-facing, single colour).

    Args:
        subject: What the icon depicts (e.g. ``"rocket launch"``).
        style: One of :data:`M3_STYLES`.
        color: Solid icon colour (hex or name).
        background: ``"transparent"`` or ``"white"``.
        fill: ``True`` for the filled (FILL 1) variant, else outlined (FILL 0).
        weight: Stroke weight name (``thin``/``regular``/``bold``).

    Returns:
        The full prompt string.

    Raises:
        ValueError: If *style* is not a Material Symbols style.
    """
    if style not in M3_STYLES:
        raise ValueError(
            f"style {style!r} invalid; choose one of {', '.join(M3_STYLES)}"
        )
    fill_state = "filled (FILL 1)" if fill else "outlined / unfilled (FILL 0)"
    return (
        f"A single Material Symbols {style.capitalize()}-style icon of "
        f"{subject}, flat 2D vector icon, {fill_state}, drawn on a 24dp grid "
        "with content inside a 20dp x 20dp live area and 2dp padding, uniform "
        f"2dp stroke weight ({weight} weight 400), {_STYLE_CORNERS[style]}, "
        "geometric and bold, simplified, facing forward (no perspective), "
        f"single solid {color} color on a {background} background, centered, "
        "pixel-aligned, consistent line weight, monochrome, crisp edges, high "
        f"legibility, clean Google Material Symbols iconography. {_ICON_NEGATIVES}."
    )


# ---------------------------------------------------------------------------
# Generation via Nano Banana Pro
# ---------------------------------------------------------------------------


def generate_icon(
    subject: str,
    *,
    style: str = "outlined",
    color: str = "#1F1F1F",
    background: str = "transparent",
    fill: bool = False,
    out: str | Path | None = None,
    image_size: str = "1K",
    model: str | None = None,
) -> Path | bytes:
    """Generate a Material 3 icon for *subject* with Nano Banana Pro.

    Args:
        subject: What the icon depicts.
        style: One of :data:`M3_STYLES`.
        color: Solid icon colour.
        background: ``"transparent"`` or ``"white"``.
        fill: Filled vs outlined variant.
        out: Output PNG path; ``None`` returns raw bytes.
        image_size: ``"1K"``/``"2K"``/``"4K"``.
        model: Image model id override.

    Returns:
        A ``Path`` (when *out* given) or raw PNG ``bytes``.
    """
    from aphrody.images import NanoBanana

    prompt = build_icon_prompt(
        subject,
        style=style,
        color=color,
        background=background,
        fill=fill,
    )
    nb = NanoBanana(model=model)
    result = nb.generate_image(
        prompt, out=out, image_size=image_size, aspect_ratio="1:1"
    )
    # generate_image returns a list; an icon is a single image.
    return result[0]


def iconify_from_image(
    image: str | Path | bytes,
    *,
    subject_hint: str | None = None,
    style: str = "outlined",
    color: str = "#1F1F1F",
    background: str = "transparent",
    out: str | Path | None = None,
    model: str | None = None,
) -> Path | bytes:
    """Redraw an arbitrary *image* as a Material 3 icon (image-to-icon).

    Uses Nano Banana Pro's editing path: the source image conditions the
    generation while the instruction enforces the M3 icon style.

    Args:
        image: Source image (path or bytes).
        subject_hint: Optional hint about what the icon should represent.
        style: One of :data:`M3_STYLES`.
        color: Solid icon colour.
        background: ``"transparent"`` or ``"white"``.
        out: Output PNG path; ``None`` returns raw bytes.
        model: Image model id override.

    Returns:
        A ``Path`` (when *out* given) or raw PNG ``bytes``.
    """
    from aphrody.images import NanoBanana

    subject = subject_hint or "the main subject of the reference image"
    instruction = (
        f"Redraw this image as {build_icon_prompt(subject, style=style, color=color, background=background)} "
        "Preserve the recognisable silhouette of the reference but reduce it to "
        "a clean, single-colour Material Symbols glyph."
    )
    nb = NanoBanana(model=model)
    return nb.edit_image(image, instruction, out=out, aspect_ratio="1:1")


# ---------------------------------------------------------------------------
# SVG rasterisation (resvg-py, no system Cairo) and ICO packaging (Pillow)
# ---------------------------------------------------------------------------


def _coerce_bytes(value: object) -> bytes:
    """Coerce a resvg-py return (``bytes`` or ``list[int]``) into ``bytes``."""
    if isinstance(value, bytes):
        return value
    if isinstance(value, bytearray):
        return bytes(value)
    return bytes(value)  # list[int] / sequence of ints


def svg_to_png(
    svg: str | Path,
    *,
    size: int = 256,
    color: str | None = None,
    background: str | None = None,
) -> bytes:
    """Rasterise an SVG into PNG bytes at *size* x *size* via resvg-py.

    Args:
        svg: SVG markup (``str`` starting with ``<``) or a path to an ``.svg``.
        size: Output square edge in pixels.
        color: Optional fill override, injected as a CSS stylesheet
            (``path { fill: <color> }``) — useful to recolour monochrome
            Material Symbols.
        background: Optional solid background colour (else transparent).

    Returns:
        PNG-encoded bytes.

    Raises:
        RuntimeError: If resvg-py is not installed.
    """
    try:
        import resvg_py
    except ImportError as exc:  # pragma: no cover - depends on env
        raise RuntimeError(
            "resvg-py is required for SVG rasterisation; "
            "install the icons extra: uv pip install 'aphrody[icons]'"
        ) from exc

    kwargs: dict = {"width": size, "height": size}
    if background is not None:
        kwargs["background"] = background
    if color is not None:
        kwargs["style_sheet"] = f"path {{ fill: {color} }}"

    is_markup = isinstance(svg, str) and svg.lstrip().startswith("<")
    if is_markup:
        raw = resvg_py.svg_to_bytes(svg_string=svg, **kwargs)
    else:
        raw = resvg_py.svg_to_bytes(svg_path=str(svg), **kwargs)
    return _coerce_bytes(raw)


def png_to_ico(
    png_bytes: bytes, *, sizes: tuple[int, ...] = DEFAULT_ICO_SIZES
) -> bytes:
    """Convert PNG bytes into a multi-resolution Windows ``.ico``.

    Args:
        png_bytes: Source PNG (should be at least as large as ``max(sizes)``).
        sizes: Square icon sizes embedded in the ``.ico``.

    Returns:
        ICO-encoded bytes containing every requested size.

    Raises:
        RuntimeError: If Pillow is not installed.
    """
    import io

    try:
        from PIL import Image
    except ImportError as exc:  # pragma: no cover - depends on env
        raise RuntimeError(
            "Pillow is required for .ico packaging; "
            "install the images extra: uv pip install 'aphrody[images]'"
        ) from exc

    img = Image.open(io.BytesIO(png_bytes))
    img.load()
    if img.mode != "RGBA":
        img = img.convert("RGBA")
    # Upscale source to the largest target if it is smaller, so every entry is
    # generated from real pixels rather than being silently dropped by Pillow.
    largest = max(sizes)
    if max(img.size) < largest:
        img = img.resize((largest, largest), Image.Resampling.LANCZOS)
    buf = io.BytesIO()
    img.save(buf, format="ICO", sizes=[(s, s) for s in sizes])
    return buf.getvalue()


def make_windows_ico(
    source: str | Path | bytes,
    out: str | Path,
    *,
    sizes: tuple[int, ...] = DEFAULT_ICO_SIZES,
    color: str | None = None,
) -> Path:
    """Produce a Windows ``.ico`` from a PNG/SVG *source*.

    Args:
        source: A ``.svg`` path / SVG markup, a ``.png`` path, or raw bytes.
        out: Destination ``.ico`` path.
        sizes: Icon sizes to embed.
        color: Optional fill recolour (SVG sources only).

    Returns:
        The written ``.ico`` ``Path``.
    """
    png = _source_to_png(source, size=max(sizes), color=color)
    ico = png_to_ico(png, sizes=sizes)
    dest = Path(out)
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(ico)
    logger.info("wrote %d-size .ico -> %s", len(sizes), dest)
    return dest


def _source_to_png(
    source: str | Path | bytes, *, size: int, color: str | None
) -> bytes:
    """Normalise a PNG/SVG/bytes *source* into PNG bytes at *size*."""
    if isinstance(source, bytes):
        return source
    text = str(source)
    if text.lstrip().startswith("<") or text.lower().endswith(".svg"):
        return svg_to_png(source, size=size, color=color)
    return Path(source).read_bytes()


# ---------------------------------------------------------------------------
# Cloned material-design-icons checkout: catalogue + bulk conversion
# ---------------------------------------------------------------------------


@dataclass
class SymbolEntry:
    """One Material Symbol available in a local checkout.

    Attributes:
        name: snake_case icon name (e.g. ``arrow_forward``).
        styles: Mapping of style -> the default 24px SVG path for that style.
    """

    name: str
    styles: dict[str, Path] = field(default_factory=dict)


def catalogue_symbols(repo_dir: str | Path) -> dict[str, SymbolEntry]:
    """Index the Material Symbols SVGs in a ``material-design-icons`` checkout.

    Walks ``symbols/web/<name>/<style_folder>/<name>_24px.svg``.

    Args:
        repo_dir: Path to the repository root (the clone).

    Returns:
        Mapping of icon name -> :class:`SymbolEntry`. Empty if the expected
        ``symbols/web`` layout is absent.
    """
    root = Path(repo_dir) / "symbols" / "web"
    catalogue: dict[str, SymbolEntry] = {}
    if not root.is_dir():
        logger.warning("no symbols/web under %s", repo_dir)
        return catalogue
    for icon_dir in sorted(p for p in root.iterdir() if p.is_dir()):
        entry = SymbolEntry(name=icon_dir.name)
        for style, folder in STYLE_FOLDERS.items():
            svg = icon_dir / folder / f"{icon_dir.name}_24px.svg"
            if svg.is_file():
                entry.styles[style] = svg
        if entry.styles:
            catalogue[entry.name] = entry
    logger.info("catalogued %d symbols from %s", len(catalogue), root)
    return catalogue


def material_symbols_css(style: str = "outlined") -> str:
    """Return the canonical self-host CSS rule for a Material Symbols *style*.

    Args:
        style: One of :data:`M3_STYLES`.

    Returns:
        A CSS snippet declaring the ``.material-symbols-<style>`` class.

    Raises:
        ValueError: If *style* is invalid.
    """
    if style not in M3_STYLES:
        raise ValueError(f"style {style!r} invalid")
    family = f"Material Symbols {style.capitalize()}"
    return (
        f".{CSS_CLASSES[style]} {{\n"
        f"  font-family: '{family}';\n"
        "  font-weight: normal;\n"
        "  font-style: normal;\n"
        "  font-size: 24px;\n"
        "  line-height: 1;\n"
        "  letter-spacing: normal;\n"
        "  text-transform: none;\n"
        "  display: inline-block;\n"
        "  white-space: nowrap;\n"
        "  direction: ltr;\n"
        "  font-variation-settings: 'FILL' 0, 'wght' 400, 'GRAD' 0, 'opsz' 24;\n"
        "}\n"
    )


@dataclass
class ConvertResult:
    """Outcome of converting one symbol to ``.ico``.

    Attributes:
        name: Icon name.
        ico: Path to the produced ``.ico`` (``None`` on failure).
        png: Path to the optimised intermediate PNG (``None`` if not kept).
        error: Error message on failure, else ``None``.
    """

    name: str
    ico: Path | None = None
    png: Path | None = None
    error: str | None = None

    @property
    def ok(self) -> bool:
        """Return ``True`` if an ``.ico`` was produced."""
        return self.error is None and self.ico is not None


def convert_symbols(
    repo_dir: str | Path,
    names: list[str],
    *,
    out_dir: str | Path,
    style: str = "outlined",
    color: str | None = None,
    sizes: tuple[int, ...] = DEFAULT_ICO_SIZES,
    keep_png: bool = True,
) -> list[ConvertResult]:
    """Convert named Material Symbols from a checkout into Windows ``.ico``.

    Pipeline per icon: SVG -> resvg raster (max size) -> lossless PNG (oxipng)
    -> multi-resolution ``.ico``.

    Args:
        repo_dir: The cloned repository root.
        names: Icon names to convert (snake_case). Empty means *all* catalogued.
        out_dir: Destination directory for ``.ico`` (and PNG if kept).
        style: One of :data:`M3_STYLES`.
        color: Optional fill recolour.
        sizes: ``.ico`` sizes.
        keep_png: Also write the optimised PNG alongside the ``.ico``.

    Returns:
        A list of :class:`ConvertResult`.
    """
    from aphrody import optimize as opt

    catalogue = catalogue_symbols(repo_dir)
    targets = names or list(catalogue)
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    largest = max(sizes)

    results: list[ConvertResult] = []
    for name in targets:
        res = ConvertResult(name=name)
        entry = catalogue.get(name)
        if entry is None or style not in entry.styles:
            res.error = f"not found (style={style})"
            results.append(res)
            continue
        try:
            png = svg_to_png(entry.styles[style], size=largest, color=color)
            png = opt.optimize_png(png)
            if keep_png:
                png_path = out / f"{name}.png"
                png_path.write_bytes(png)
                res.png = png_path
            ico_path = out / f"{name}.ico"
            ico_path.write_bytes(png_to_ico(png, sizes=sizes))
            res.ico = ico_path
        except (RuntimeError, OSError, ValueError) as exc:
            res.error = f"{type(exc).__name__}: {exc}"
            logger.warning("convert %s failed: %s", name, res.error)
        results.append(res)
    logger.info(
        "converted %d/%d symbols (style=%s) -> %s",
        sum(1 for r in results if r.ok),
        len(results),
        style,
        out,
    )
    return results


def fetch_symbol_svg(
    name: str, *, style: str = "outlined", fill: bool = False
) -> str:
    """Fetch a Material Symbols SVG by name via Iconify (no local checkout).

    Args:
        name: Icon name (snake_case or kebab; e.g. ``"home"``, ``"arrow_forward"``).
        style: One of :data:`M3_STYLES`.
        fill: Filled variant.

    Returns:
        SVG markup as a string.

    Raises:
        RuntimeError: If pyconify is not installed.
        KeyError: If Iconify has no such icon.
    """
    try:
        import pyconify
    except ImportError as exc:  # pragma: no cover - depends on env
        raise RuntimeError(
            "pyconify is required to fetch icons by name; "
            "install the icons extra: uv pip install 'aphrody[icons]'"
        ) from exc

    # Iconify prefixes: material-symbols (outlined baseline); rounded/sharp are
    # suffixed on the icon name; FILL via the -fill suffix.
    base = name.replace("_", "-")
    suffix = ""
    if style == "rounded":
        suffix = "-rounded"
    elif style == "sharp":
        suffix = "-sharp"
    fill_suffix = "-fill" if fill else ""
    key = f"material-symbols:{base}{suffix}{fill_suffix}"
    svg = pyconify.svg(key)
    return svg.decode("utf-8") if isinstance(svg, bytes) else svg


def apply_folder_icon(
    folder: str | Path, ico: str | Path, *, icon_index: int = 0
) -> Path:
    """Set a Windows folder's icon to *ico* via a ``desktop.ini`` (reversible).

    This is the safe, per-folder way to swap in a Material 3 icon: it writes a
    ``desktop.ini`` pointing at *ico* and marks it hidden+system so Explorer
    honours it. **Fully reversible** — delete the ``desktop.ini`` and clear the
    folder's read-only flag to restore the default icon. This intentionally
    does NOT touch system icons (``shell32.dll`` / registry), which would be
    destructive and hard to undo.

    Args:
        folder: Target folder whose icon to change.
        ico: Path to a ``.ico`` file (an absolute path is recorded).
        icon_index: Icon index within the ``.ico`` (usually 0).

    Returns:
        The path to the written ``desktop.ini``.

    Raises:
        NotADirectoryError: If *folder* is not a directory.
    """
    import os
    import subprocess

    folder_p = Path(folder)
    if not folder_p.is_dir():
        raise NotADirectoryError(str(folder_p))
    ico_p = Path(ico).resolve()
    ini = folder_p / "desktop.ini"
    ini.write_text(
        "[.ShellClassInfo]\n"
        f"IconResource={ico_p},{icon_index}\n"
        "ConfirmFileOp=0\n",
        encoding="utf-8",
    )
    if os.name == "nt":  # pragma: no cover - Windows-only shell attributes
        subprocess.run(["attrib", "+h", "+s", str(ini)], check=False)
        subprocess.run(["attrib", "+r", str(folder_p)], check=False)
    logger.info("applied folder icon %s -> %s", ico_p.name, folder_p)
    return ini
