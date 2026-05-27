# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Material 3 icon generation and packaging command group for the aphrody CLI."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from aphrody.cli.utils import _emit


class IconCommands:
    """``aphrody image icon <action>`` — Material 3 icon generation + packaging.

    Generate authentic Material Symbols icons with Nano Banana Pro, redraw an
    image as an icon, rasterise SVGs, and package multi-resolution Windows
    ``.ico`` files (to swap in M3 iconography). Conversion reads a cloned
    ``material-design-icons`` checkout or fetches by name via Iconify.
    """

    def gen(
        self,
        subject: str,
        out: str = "icon.png",
        style: str = "outlined",
        color: str = "#1F1F1F",
        background: str = "transparent",
        fill: bool = False,
        size: str = "1K",
        model: str | None = None,
        ico: bool = False,
    ) -> None:
        """Generate a Material 3 icon for ``subject``.

        Args:
            subject: What the icon depicts (e.g. "rocket launch").
            out: Output PNG path.
            style: ``outlined`` / ``rounded`` / ``sharp``.
            color: Solid icon colour (hex or name).
            background: ``transparent`` or ``white``.
            fill: Generate the filled (FILL 1) variant.
            size: Resolution ``1K`` / ``2K`` / ``4K``.
            model: Image model id override.
            ico: Also write a multi-resolution Windows ``.ico`` sibling.
        """
        from aphrody import icons

        path = icons.generate_icon(
            subject,
            style=style,
            color=color,
            background=background,
            fill=fill,
            out=out,
            image_size=size,
            model=model,
        )
        result: dict[str, Any] = {"style": style, "saved": str(path)}
        if ico:
            ico_path = icons.make_windows_ico(
                path, Path(out).with_suffix(".ico")
            )
            result["ico"] = str(ico_path)
        _emit(result)

    def from_image(
        self,
        image: str,
        out: str = "icon.png",
        subject: str | None = None,
        style: str = "outlined",
        color: str = "#1F1F1F",
        model: str | None = None,
        ico: bool = False,
    ) -> None:
        """Redraw an existing image as a Material 3 icon.

        Args:
            image: Path to the source/base image.
            out: Output PNG path.
            subject: Optional hint about what the icon should represent.
            style: ``outlined`` / ``rounded`` / ``sharp``.
            color: Solid icon colour.
            model: Image model id override.
            ico: Also write a Windows ``.ico`` sibling.
        """
        from aphrody import icons

        path = icons.iconify_from_image(
            image,
            subject_hint=subject,
            style=style,
            color=color,
            out=out,
            model=model,
        )
        result: dict[str, Any] = {"style": style, "saved": str(path)}
        if ico:
            ico_path = icons.make_windows_ico(
                path, Path(out).with_suffix(".ico")
            )
            result["ico"] = str(ico_path)
        _emit(result)

    def svg2ico(
        self,
        svg: str,
        out: str,
        size: int = 256,
        color: str | None = None,
    ) -> None:
        """Convert an SVG file into a multi-resolution Windows ``.ico``.

        Args:
            svg: Path to an ``.svg`` file.
            out: Destination ``.ico`` path.
            size: Rasterisation edge for the largest embedded size.
            color: Optional fill recolour.
        """
        from aphrody import icons

        sizes = tuple(s for s in icons.DEFAULT_ICO_SIZES if s <= size)
        ico_path = icons.make_windows_ico(
            svg, out, sizes=sizes or icons.DEFAULT_ICO_SIZES, color=color
        )
        _emit({"ico": str(ico_path)})

    def convert(
        self,
        repo_dir: str,
        out_dir: str = "out/icons",
        style: str = "outlined",
        names: str | None = None,
        color: str | None = None,
    ) -> None:
        """Bulk-convert Material Symbols from a checkout into Windows ``.ico``.

        Args:
            repo_dir: Path to a cloned ``material-design-icons`` checkout.
            out_dir: Destination directory for the ``.ico`` (and PNG) files.
            style: ``outlined`` / ``rounded`` / ``sharp``.
            names: Comma-separated icon names; omit to convert the whole set.
            color: Optional fill recolour.
        """
        from aphrody import icons

        wanted = (
            [n.strip() for n in names.split(",") if n.strip()] if names else []
        )
        results = icons.convert_symbols(
            repo_dir, wanted, out_dir=out_dir, style=style, color=color
        )
        _emit(
            {
                "out_dir": out_dir,
                "style": style,
                "ok": sum(1 for r in results if r.ok),
                "failed": sum(1 for r in results if not r.ok),
                "icons": [
                    {
                        "name": r.name,
                        "ico": str(r.ico) if r.ico else None,
                        "error": r.error,
                    }
                    for r in results
                ],
            }
        )

    def catalogue(self, repo_dir: str, limit: int = 50) -> None:
        """Index the Material Symbols available in a local checkout.

        Args:
            repo_dir: Path to a cloned ``material-design-icons`` checkout.
            limit: Maximum icon names to list in the output.
        """
        from aphrody import icons

        cat = icons.catalogue_symbols(repo_dir)
        names = sorted(cat)
        _emit(
            {
                "total": len(names),
                "styles": list(icons.STYLE_FOLDERS),
                "css_classes": icons.CSS_CLASSES,
                "sample": names[:limit],
            }
        )

    def css(self, style: str = "outlined") -> None:
        """Print the canonical self-host CSS for a Material Symbols style.

        Args:
            style: ``outlined`` / ``rounded`` / ``sharp``.
        """
        from aphrody import icons

        print(icons.material_symbols_css(style))

    def fetch(
        self,
        name: str,
        out: str | None = None,
        style: str = "outlined",
        fill: bool = False,
        ico: bool = False,
    ) -> None:
        """Fetch a Material Symbols SVG by name via Iconify (no checkout).

        Args:
            name: Icon name (e.g. ``home``, ``arrow_forward``).
            out: Output ``.svg`` path; omit to print the SVG markup.
            style: ``outlined`` / ``rounded`` / ``sharp``.
            fill: Fetch the filled variant.
            ico: Also write a Windows ``.ico`` (requires ``--out``).
        """
        from aphrody import icons

        svg = icons.fetch_symbol_svg(name, style=style, fill=fill)
        if not out:
            print(svg)
            return
        Path(out).write_text(svg, encoding="utf-8")
        result: dict[str, Any] = {"saved": out}
        if ico:
            ico_path = icons.make_windows_ico(
                out, Path(out).with_suffix(".ico")
            )
            result["ico"] = str(ico_path)
        _emit(result)

    def apply_folder(self, folder: str, ico: str) -> None:
        """Set a Windows folder's icon to an ``.ico`` (reversible desktop.ini).

        Safe and per-folder: writes a ``desktop.ini`` pointing at *ico*. Undo by
        deleting that file. Does not touch system icons.

        Args:
            folder: Target folder.
            ico: Path to the ``.ico`` to apply.
        """
        from aphrody import icons

        ini = icons.apply_folder_icon(folder, ico)
        _emit({"folder": folder, "ico": ico, "desktop_ini": str(ini)})
