# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""The ``aphrody`` command-line interface.

A keyless CLI for Google's AI Ultra stack, built with python-fire (Google's
own CLI generator). Every command authenticates with the local Antigravity
OAuth token — there is no API key anywhere.

Examples:
    $ aphrody whoami
    $ aphrody token
    $ aphrody chat "Explain OAuth refresh in one sentence."
    $ aphrody image gen "a banana-shaped spaceship, studio render" --out ship.png --size 4K
    $ aphrody image template logo --brand_name Aphrody --out logo.png
    $ aphrody image batch shots.json --out_dir out
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

import httpx

from aphrody import __version__, endpoints, gemini_web, vertex
from aphrody.auth import cookies as cookies_store
from aphrody.auth import credentials, oauth
from aphrody.client import AphrodyClient
from aphrody.errors import AphrodyError, ApiError


class _CookieCommands:
    """``aphrody cookies <action>`` — manage the keyless Google cookie jar.

    Values are never printed: :meth:`status` reports metadata only.
    """

    def status(self) -> None:
        """Show the stored cookie jar metadata (names/domains, never values)."""
        _emit(cookies_store.status())

    def load(self, file: str) -> None:
        """Import cookies from a Cookie-Editor JSON export *file*.

        Args:
            file: Path to a Cookie-Editor (or compatible) JSON export.
        """
        text = Path(file).read_text(encoding="utf-8")
        jar = cookies_store.import_cookie_editor(text)
        _emit({"imported": len(jar), **cookies_store.status(jar)})

    def extract(self, domain: str = "google.com") -> None:
        """Extract cookies straight from local Chrome (best effort).

        Args:
            domain: Cookie domain filter (default ``"google.com"``).
        """
        jar = cookies_store.extract_from_chrome(domain)
        _emit({"extracted": len(jar), **cookies_store.status(jar)})


class _ImageCommands:
    """``aphrody image <action>`` — the keyless Nano Banana Pro image suite.

    Backed by Gemini 3 Pro Image (``gemini-3-pro-image-preview``) on Vertex AI
    with automatic fallback to Flash image models. Resolution (``--size``) is
    ``1K``/``2K``/``4K``; ``--aspect`` is one of the ten supported ratios.
    """

    def gen(
        self,
        prompt: str,
        out: str = "nanobanana.png",
        size: str | None = None,
        aspect: str | None = None,
        n: int = 1,
        model: str | None = None,
        negative: str | None = None,
        grounding: bool = False,
        enhance: str | None = None,
        optimize: Any = False,
    ) -> None:
        """Generate one or more images from a text prompt.

        Args:
            prompt: The image description.
            out: Output PNG path (or directory when ``n > 1``).
            size: Resolution ``1K`` / ``2K`` / ``4K`` (Nano Banana Pro).
            aspect: Aspect ratio, e.g. ``16:9`` (see ``image models``).
            n: Number of images to generate.
            model: Image model id override.
            negative: Negative constraints to avoid.
            grounding: Enable Google Search grounding for factual images.
            enhance: Apply a style preset (e.g. ``photoreal``, ``cinematic``).
            optimize: Post-optimise outputs; ``True`` for png+webp, or a list
                / comma string like ``png,webp,avif``.
        """
        from aphrody.images import NanoBanana

        text = prompt
        if enhance:
            from aphrody import prompts as prompt_lib

            text = prompt_lib.enhance_prompt(prompt, preset=enhance)

        nb = NanoBanana(model=model)
        paths = nb.generate_image(
            text,
            out=out,
            n=n,
            aspect_ratio=aspect,
            image_size=size,
            negative_prompt=negative,
            grounding=grounding,
        )
        saved = [str(p) for p in _as_list(paths)]
        result: dict[str, Any] = {
            "model": nb.last_model,
            "saved": saved,
            "prompt": text,
        }

        fmts = _parse_formats(optimize)
        if fmts:
            from aphrody import optimize as opt

            summaries = []
            for path in saved:
                res = opt.optimize_file(
                    path, webp="webp" in fmts, avif="avif" in fmts
                )
                summaries.append({"path": path, "summary": res.summary()})
            result["optimized"] = summaries
        _emit(result)

    def edit(
        self,
        image: str,
        prompt: str,
        out: str | None = None,
        size: str | None = None,
        aspect: str | None = None,
        model: str | None = None,
    ) -> None:
        """Edit an existing image with a natural-language instruction.

        Args:
            image: Path to the source image.
            prompt: Editing instruction (e.g. "make the tie green").
            out: Output PNG path (omit to report byte count only).
            size: Output resolution (Pro only).
            aspect: Output aspect ratio.
            model: Image model id override.
        """
        from aphrody import images

        res = images.edit_image(
            image,
            prompt,
            out=out,
            model=model,
            aspect_ratio=aspect,
            image_size=size,
        )
        _emit({"saved": str(res)} if out else {"bytes": len(res)})

    def compose(
        self,
        prompt: str,
        *images_: str,
        out: str | None = None,
        size: str | None = None,
        aspect: str | None = None,
        model: str | None = None,
    ) -> None:
        """Compose/blend up to 14 reference images per a text instruction.

        Args:
            prompt: Compositing instruction stating how the references relate.
            *images_: One or more source image paths.
            out: Output PNG path (omit to report byte count only).
            size: Output resolution (Pro only).
            aspect: Output aspect ratio.
            model: Image model id override.
        """
        from aphrody import images as img

        if not images_:
            raise AphrodyError("compose requires at least one image path")
        res = img.compose_images(
            list(images_),
            prompt,
            out=out,
            model=model,
            aspect_ratio=aspect,
            image_size=size,
        )
        _emit({"saved": str(res)} if out else {"bytes": len(res)})

    def optimize(
        self,
        path: str,
        webp: bool = True,
        avif: bool = False,
        level: int = 6,
    ) -> None:
        """Losslessly optimise a PNG and emit WebP/AVIF siblings.

        Args:
            path: Path to a PNG file (optimised in place).
            webp: Also write a ``.webp`` sibling.
            avif: Also write an ``.avif`` sibling.
            level: oxipng effort level 0-6.
        """
        from aphrody import optimize as opt

        res = opt.optimize_file(path, webp=webp, avif=avif, png_level=level)
        _emit(
            {
                "path": str(path),
                "summary": res.summary(),
                "outputs": list(res.outputs),
            }
        )

    def batch(
        self,
        spec: str,
        out_dir: str = "out",
        workers: int = 3,
        model: str | None = None,
    ) -> None:
        """Generate many images from a JSON spec, concurrently.

        Args:
            spec: Path to a batch spec JSON file (``{"items": [...]}``).
            out_dir: Destination directory.
            workers: Maximum concurrent generation calls.
            model: Image model id override for the whole run.
        """
        from aphrody import batch as batch_mod

        defaults, items = batch_mod.load_spec(spec)
        results = batch_mod.generate_batch(
            items,
            out_dir=out_dir,
            defaults=defaults,
            model=model,
            max_workers=workers,
        )
        _emit(
            {
                "out_dir": out_dir,
                "total": len(results),
                "ok": sum(1 for r in results if r.ok),
                "failed": sum(1 for r in results if not r.ok),
                "items": [
                    {
                        "id": r.id,
                        "ok": r.ok,
                        "model": r.model,
                        "paths": [str(p) for p in r.paths],
                        "error": r.error,
                    }
                    for r in results
                ],
            }
        )

    def prompts(self, category: str | None = None) -> None:
        """List the built-in Nano Banana Pro prompt templates.

        Args:
            category: Optional category filter (e.g. ``product``, ``portrait``).
        """
        from aphrody import prompts as prompt_lib

        _emit(
            [
                {
                    "id": t.id,
                    "category": t.category,
                    "placeholders": list(t.placeholders),
                }
                for t in prompt_lib.list_templates(category)
            ]
        )

    def template(
        self,
        template_id: str,
        out: str | None = None,
        size: str | None = None,
        aspect: str | None = None,
        n: int = 1,
        model: str | None = None,
        enhance: str | None = None,
        **variables: str,
    ) -> None:
        """Render a prompt template and (when ``--out`` is given) generate it.

        Without ``--out`` this prints the resolved prompt (a dry run); with it,
        the image is generated. Template placeholders are passed as ``--name
        value`` flags.

        Args:
            template_id: The template id (see ``aphrody image prompts``).
            out: Output PNG path; omit for a dry run.
            size: Output resolution (Pro only).
            aspect: Output aspect ratio.
            n: Number of images to generate.
            model: Image model id override.
            enhance: Apply a style preset to the rendered prompt.
            **variables: Placeholder substitutions for the template.
        """
        from aphrody import prompts as prompt_lib

        text = prompt_lib.render_template(template_id, **variables)
        if enhance:
            text = prompt_lib.enhance_prompt(text, preset=enhance)

        if not out:
            _emit({"template": template_id, "prompt": text})
            return

        from aphrody import images

        paths = images.generate_image(
            text,
            out=out,
            n=n,
            model=model,
            aspect_ratio=aspect,
            image_size=size,
        )
        _emit(
            {
                "template": template_id,
                "prompt": text,
                "saved": [str(p) for p in _as_list(paths)],
            }
        )

    def enhance(
        self,
        prompt: str,
        preset: str | None = None,
        lens: str | None = None,
        lighting: str | None = None,
        camera: str | None = None,
        quality: str = "4K",
        negatives: bool = False,
    ) -> None:
        """Enhance a base prompt with best-practice modifiers and print it.

        Args:
            prompt: The base description.
            preset: A style preset (``photoreal``/``cinematic``/``product``/...).
            lens: Explicit lens descriptor.
            lighting: Explicit lighting descriptor.
            camera: Explicit camera / film-look descriptor.
            quality: Quality suffix (default ``4K``).
            negatives: Append a default negative-constraint clause.
        """
        from aphrody import prompts as prompt_lib

        _emit(
            {
                "prompt": prompt_lib.enhance_prompt(
                    prompt,
                    preset=preset,
                    lens=lens,
                    lighting=lighting,
                    camera=camera,
                    quality=quality,
                    negatives=negatives,
                )
            }
        )

    def analyze(
        self,
        path: str,
        palette: str | None = None,
        palette_size: int = 8,
    ) -> None:
        """Deep-analyze an image: metadata, subject bbox, dominant palette.

        Args:
            path: Path to the image to analyse.
            palette: Optional PNG path to write a palette swatch to.
            palette_size: Number of dominant colours to extract.
        """
        from aphrody import analyze as analyze_mod

        report = analyze_mod.analyze_image(path, palette_size=palette_size)
        if palette and report["dominant_colors"]:
            swatch = analyze_mod.save_palette_swatch(
                report["dominant_colors"], palette
            )
            report["palette_swatch"] = str(swatch)
        _emit(report)

    def anim(self) -> _AnimCommands:
        """Animation suite (build / turntable / spritesheet)."""
        return _AnimCommands()

    def to3d(
        self,
        image: str,
        out: str = "model.glb",
        method: str = "relief",
        depth_scale: float = 0.15,
        max_dim: int = 200,
        texture: bool = False,
    ) -> None:
        """Convert a 2D image into a 3D ``.glb`` mesh.

        Args:
            image: Source image (transparency / near-white bg defines the
                subject silhouette).
            out: Destination ``.glb`` path.
            method: ``relief`` (no GPU/ML) or ``depth`` (Depth Anything V2).
            depth_scale: Z displacement as a fraction of the longest edge.
            max_dim: Longest grid edge (vertex budget).
            texture: UV-map the sprite as a full-image texture (textured model)
                instead of baked per-vertex colours.
        """
        from aphrody import to3d as to3d_mod

        path = to3d_mod.image_to_mesh(
            image,
            out,
            method=method,
            depth_scale=depth_scale,
            max_dim=max_dim,
            texture=texture,
        )
        _emit(
            {
                "saved": str(path),
                "method": method,
                "max_dim": max_dim,
                "texture": texture,
            }
        )

    def actions(
        self,
        image: str,
        out_dir: str = "actions",
        actions: str | None = None,
        size: str = "1K",
        model: str | None = None,
        fps: float = 6.0,
        delay: float = 0.0,
    ) -> None:
        """Generate identity-preserving action animations from a sprite.

        Nano Banana Pro redraws the same character performing each action
        (walk / run / jump / crouch / fly / kick_ball) as a short motion cycle,
        assembled into looping WebPs + a combined showreel. Pair with
        ``aphrody blender multiview "<out_dir>/<action>_r*.png"`` for a 3D render.

        Args:
            image: Reference full-body sprite (defines the character identity).
            out_dir: Destination directory.
            actions: Comma-separated subset (e.g. ``run,jump,kick_ball``); omit
                for all.
            size: Output resolution ``1K`` / ``2K`` / ``4K``.
            model: Image model id override.
            fps: Loop playback frame rate.
            delay: Seconds between calls (pace under a per-minute quota).
        """
        from aphrody.sprite_anim import generate_actions

        chosen = (
            [a.strip() for a in actions.split(",") if a.strip()]
            if actions
            else None
        )
        manifest = generate_actions(
            image,
            out_dir,
            actions=chosen,
            image_size=size,
            model=model,
            fps=fps,
            delay=delay,
        )
        _emit(manifest)

    def icon(self) -> _IconCommands:
        """Material 3 icon suite (gen / from_image / svg2ico / convert / css)."""
        return _IconCommands()

    def models(self) -> None:
        """List the image models and the fallback chain."""
        from aphrody import images

        _emit(
            {
                "default": images.DEFAULT_IMAGE_MODEL,
                "fallback_chain": list(images.FALLBACK_CHAIN),
                "aspect_ratios": list(_image_aspect_ratios()),
                "image_sizes": list(_image_sizes()),
                "models": {
                    images.NANO_BANANA_PRO: "Gemini 3 Pro Image — max fidelity, 1K/2K/4K, typography, grounding",
                    images.NANO_BANANA_2_FLASH: "Gemini 3.1 Flash Image — fast, thinking support",
                    images.NANO_BANANA_FLASH: "Gemini 2.5 Flash Image — fastest, most widely enabled",
                },
            }
        )


class _IconCommands:
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


class _AnimCommands:
    """``aphrody image anim <action>`` — frame animation + spritesheets.

    Turn sprite frames (e.g. rotation frames) into a looping animated WebP / GIF
    / APNG, or pack them into a uniform-grid spritesheet with a JSON atlas.
    """

    def build(
        self,
        *frames: str,
        out: str = "anim.webp",
        fmt: str | None = None,
        fps: float = 12.0,
        loop: int = 0,
        pingpong: bool = False,
        transparent: bool = True,
    ) -> None:
        """Build a looping animation from explicit frame paths.

        Args:
            *frames: Ordered frame image paths.
            out: Destination path (extension picks the format if --fmt unset).
            fmt: ``webp`` / ``gif`` / ``apng``.
            fps: Frames per second.
            loop: Loop count (0 = infinite).
            pingpong: Append the reversed sequence for a back-and-forth loop.
            transparent: Preserve alpha.
        """
        from aphrody import anim

        if not frames:
            raise AphrodyError("anim build requires at least one frame path")
        path = anim.build_animation(
            list(frames),
            out,
            fmt=fmt,
            fps=fps,
            loop=loop,
            pingpong=pingpong,
            transparent=transparent,
        )
        _emit({"saved": str(path), "frames": len(frames), "fps": fps})

    def turntable(
        self,
        pattern: str,
        out: str = "turntable.webp",
        fmt: str | None = None,
        fps: float = 10.0,
        pingpong: bool = True,
        transparent: bool = True,
    ) -> None:
        """Build a rotation loop from frames matching a glob ``pattern``.

        Frames are ordered by their ``_r<n>`` index (e.g.
        ``assets/aphrody_r*.webp``).

        Args:
            pattern: Glob pattern matching the rotation frames.
            out: Destination animation path.
            fmt: ``webp`` / ``gif`` / ``apng``.
            fps: Frames per second.
            pingpong: Back-and-forth loop (smooth for a <360 sweep).
            transparent: Preserve alpha.
        """
        from aphrody import anim

        path = anim.turntable(
            pattern,
            out,
            fmt=fmt,
            fps=fps,
            pingpong=pingpong,
            transparent=transparent,
        )
        _emit({"saved": str(path), "pattern": pattern, "fps": fps})

    def spritesheet(
        self,
        *frames: str,
        out: str = "spritesheet.png",
        columns: int | None = None,
    ) -> None:
        """Pack frames into a uniform-grid spritesheet + JSON atlas.

        Args:
            *frames: Frame image paths.
            out: Destination PNG path (a sibling ``.json`` atlas is written).
            columns: Grid columns (defaults to near-square).
        """
        from aphrody import anim

        if not frames:
            raise AphrodyError("anim spritesheet requires at least one frame")
        path, atlas = anim.make_spritesheet(list(frames), out, columns=columns)
        _emit(
            {
                "saved": str(path),
                "atlas": str(Path(path).with_suffix(".json")),
                "grid": f"{atlas['columns']}x{atlas['rows']}",
                "frame_size": [atlas["frame_width"], atlas["frame_height"]],
                "count": atlas["count"],
            }
        )


def _image_aspect_ratios() -> tuple[str, ...]:
    """Return the supported aspect ratios (indirection keeps imports lazy)."""
    from aphrody.prompts import ASPECT_RATIOS

    return ASPECT_RATIOS


def _image_sizes() -> tuple[str, ...]:
    """Return the supported image-size tiers."""
    from aphrody.prompts import IMAGE_SIZES

    return IMAGE_SIZES


def _parse_formats(value: Any) -> tuple[str, ...]:
    """Normalise an ``optimize`` flag into a tuple of format names.

    Args:
        value: ``False``/``None`` (none), ``True`` (png+webp default), a list,
            or a comma-separated string like ``"png,webp,avif"``.

    Returns:
        A tuple of lowercase format names.
    """
    if not value:
        return ()
    if value is True:
        return ("png", "webp")
    if isinstance(value, (list, tuple)):
        return tuple(str(v).strip().lower() for v in value if str(v).strip())
    return tuple(s.strip().lower() for s in str(value).split(",") if s.strip())


def _autocomplete_dry_run(
    prefix: str | None,
    file: str | None,
    suffix: str,
    line: int | None,
    col: int | None,
    lang: str | None,
    marker: str | None,
    n: int,
    model: str,
) -> dict[str, Any]:
    """Resolve an autocomplete request offline (no model call) for smoke tests.

    Returns the normalised request + the exact prompt that *would* be sent, so
    the command can be exercised without burning live quota or needing network.
    """
    from aphrody import autocomplete as ac

    if file is not None:
        text = Path(file).read_text(encoding="utf-8")
        eff_marker = marker if marker is not None else ac.DEFAULT_CURSOR_MARKER
        pre, suf = ac.split_at_cursor(
            text, line=line, col=col, marker=eff_marker
        )
        req = ac.CompletionRequest(
            prefix=pre,
            suffix=suf,
            language=lang or ac.language_for_path(file),
            path=file,
        )
    else:
        req = ac.CompletionRequest(
            prefix=prefix or "", suffix=suffix, language=lang
        )
    system, user = ac.build_prompt(req)
    return {
        "dry_run": True,
        "model": model,
        "n": n,
        "language": req.language,
        "mode": "fill-in-the-middle" if req.suffix.strip() else "continuation",
        "prefix": req.prefix,
        "suffix": req.suffix,
        "system_instruction": system,
        "user_prompt": user,
    }


def _emit(value: Any) -> None:
    """Print a result as indented JSON (dict/list) or plain text."""
    if isinstance(value, (dict, list)):
        print(json.dumps(value, indent=2, ensure_ascii=False))
    else:
        print(value)


class _BlenderCommands:
    """``aphrody blender <action>`` — drive a running Blender via blender-mcp.

    Requires Blender open with the blender-mcp addon server started (its
    BlenderMCP panel > Connect, default ``localhost:9876``).
    """

    def scene(self, host: str = "localhost", port: int = 9876) -> None:
        """Print the current Blender scene summary.

        Args:
            host: Addon server host.
            port: Addon server port.
        """
        from aphrody.blender import BlenderClient

        with BlenderClient(host, port) as bl:
            _emit(bl.get_scene_info())

    def exec(
        self,
        code: str,
        file: bool = False,
        host: str = "localhost",
        port: int = 9876,
    ) -> None:
        """Run Python ``code`` inside Blender; print its stdout.

        Args:
            code: Python source, or a path to a ``.py`` file when ``--file``.
            file: Treat ``code`` as a file path to read.
            host: Addon server host.
            port: Addon server port.
        """
        from aphrody.blender import BlenderClient

        source = Path(code).read_text(encoding="utf-8") if file else code
        with BlenderClient(host, port) as bl:
            _emit({"stdout": bl.execute_code(source)})

    def import_glb(
        self, path: str, host: str = "localhost", port: int = 9876
    ) -> None:
        """Import a ``.glb``/``.gltf`` into Blender; print the new objects.

        Args:
            path: Path to the mesh file.
            host: Addon server host.
            port: Addon server port.
        """
        from aphrody.blender import BlenderClient

        with BlenderClient(host, port) as bl:
            _emit({"imported": bl.import_glb(path)})

    def render(
        self,
        out: str = "blender_render.png",
        width: int = 800,
        height: int = 800,
        engine: str | None = None,
        setup: bool = True,
        host: str = "localhost",
        port: int = 9876,
    ) -> None:
        """Render the current scene to a PNG.

        Args:
            out: Output PNG path.
            width: Render width in pixels.
            height: Render height in pixels.
            engine: Optional render engine id (e.g. ``CYCLES``).
            setup: First add a framing camera + sun light.
            host: Addon server host.
            port: Addon server port.
        """
        from aphrody.blender import BlenderClient

        with BlenderClient(host, port) as bl:
            if setup:
                bl.setup_camera_light()
            path = bl.render_still(
                out, resolution=(width, height), engine=engine
            )
        _emit({"saved": str(path)})

    def turntable(
        self,
        out_dir: str = "blender_turntable",
        frames: int = 16,
        target: str | None = None,
        width: int = 600,
        height: int = 600,
        engine: str | None = None,
        host: str = "localhost",
        port: int = 9876,
    ) -> None:
        """Render an orbiting turntable image sequence.

        Args:
            out_dir: Output directory for the frames.
            frames: Frames per full revolution.
            target: Object to spin (else the first mesh).
            width: Frame width in pixels.
            height: Frame height in pixels.
            engine: Optional render engine id.
            host: Addon server host.
            port: Addon server port.
        """
        from aphrody.blender import BlenderClient

        with BlenderClient(host, port) as bl:
            paths = bl.turntable(
                out_dir,
                frames=frames,
                target=target,
                resolution=(width, height),
                engine=engine,
            )
        _emit({"out_dir": out_dir, "frames": [str(p) for p in paths]})

    # -- headless runner (no running Blender / addon needed) --------------

    def bin(self) -> None:
        """Show the resolved Blender executable (headless runner)."""
        from aphrody.bpy_runner import resolve_blender_bin

        _emit({"blender_bin": resolve_blender_bin()})

    def headless(self, script: str, *args: str) -> None:
        """Run a ``bpy`` script via an installed Blender, headless.

        Args:
            script: Path to the Python script Blender runs (``-b -P``).
            *args: Arguments passed after ``--``.
        """
        from aphrody.bpy_runner import BlenderRunner

        result = BlenderRunner().run_script(script, list(args))
        _emit(
            {
                "returncode": result.returncode,
                "ok": result.ok,
                "stdout": result.stdout[-2000:],
                "stderr": result.stderr[-1000:],
            }
        )

    def sprite3d(
        self,
        image: str,
        out: str = "model.glb",
        frames: int = 48,
        thickness: float = 0.06,
        cross: int = 2,
        render: str | None = None,
    ) -> None:
        """Convert a sprite to an animated textured GLB via headless Blender.

        Args:
            image: Source sprite path.
            out: Destination ``.glb`` path.
            frames: Spin frames.
            thickness: Solidify depth (volume).
            cross: Fanned planes (1=flat card, 2=cross billboard, always visible).
            render: Optional directory to also render the turntable PNGs.
        """
        from aphrody.bpy_runner import run_sprite_to_3d

        path = run_sprite_to_3d(
            image,
            out,
            frames=frames,
            thickness=thickness,
            cross=cross,
            render=render,
        )
        _emit({"saved": str(path), "frames": frames, "cross": cross})

    def optimize_glb(
        self,
        input_glb: str,
        out: str,
        decimate: float = 1.0,
        merge: float = 0.0001,
    ) -> None:
        """Optimise a GLB (weld + recalc normals + decimate) via Blender.

        Args:
            input_glb: Source ``.glb`` path.
            out: Destination ``.glb`` path.
            decimate: Decimate ratio (``<1`` reduces polygons).
            merge: Merge-by-distance threshold.
        """
        from aphrody.bpy_runner import optimize_glb as run_optimize

        path = run_optimize(
            input_glb, out, decimate_ratio=decimate, merge_distance=merge
        )
        _emit({"saved": str(path)})

    def gpu(self) -> None:
        """List the Cycles GPU/CPU compute devices Blender can use (NVIDIA OPTIX/CUDA)."""
        from aphrody.bpy_runner import list_gpu_devices

        _emit(list_gpu_devices())

    def render_gpu(
        self,
        glb: str,
        out_dir: str = "gpu_frames",
        frames: int = 24,
        samples: int = 64,
        resolution: int = 512,
        device: str = "AUTO",
    ) -> None:
        """GPU-render (Cycles OPTIX/CUDA) a turntable of a ``.glb``.

        Args:
            glb: Source ``.glb`` to import and spin.
            out_dir: Destination directory for the PNG frames.
            frames: Frames in the revolution.
            samples: Cycles samples per frame.
            resolution: Square render resolution.
            device: ``AUTO`` / ``OPTIX`` / ``CUDA`` / ``CPU``.
        """
        from aphrody.bpy_runner import render_turntable_gpu

        info = render_turntable_gpu(
            glb,
            out_dir,
            frames=frames,
            samples=samples,
            resolution=resolution,
            device=device,
        )
        _emit(info)

    def showcase(
        self,
        image: str,
        out: str = "showcase.webp",
        frames: int = 24,
        thickness: float = 0.06,
        cross: int = 2,
        samples: int = 48,
        resolution: int = 384,
        fps: float = 12.0,
    ) -> None:
        """End-to-end GPU showcase: sprite → animated textured GLB → GPU render → WebP.

        Args:
            image: Source sprite path.
            out: Destination animated ``.webp`` path.
            frames: Spin/render frames.
            thickness: Standee Solidify depth.
            cross: Fanned planes (2=cross billboard, visible from all angles).
            samples: Cycles samples per frame.
            resolution: Square render resolution.
            fps: Output animation frame rate.
        """
        from aphrody.bpy_runner import showcase_sprite

        info = showcase_sprite(
            image,
            out,
            frames=frames,
            thickness=thickness,
            cross=cross,
            samples=samples,
            resolution=resolution,
            fps=fps,
        )
        _emit(info)

    def multiview(
        self,
        pattern: str,
        out: str = "multiview.webp",
        frames: int = 24,
        samples: int = 48,
        resolution: int = 512,
        ground: bool = True,
        fps: float = 12.0,
    ) -> None:
        """Solid multi-view impostor turntable from real rotation views, GPU.

        Uses the actual rotation frames (matching ``pattern``, ordered by
        ``_r<n>``) on a billboard with ground + shadow, GPU-rendered (OPTIX/CUDA)
        and assembled into an animated WebP — a true "solid" character spin.

        Args:
            pattern: Glob for the rotation views, e.g.
                ``assets/aphrody_body_r*.webp``.
            out: Destination animated ``.webp`` path.
            frames: Output frame count (views cycled across them).
            samples: Cycles samples per frame.
            resolution: Square render resolution.
            ground: Render a ground plane + shadow (else transparent).
            fps: Output animation frame rate.
        """
        from aphrody.bpy_runner import render_multiview_turntable

        info = render_multiview_turntable(
            pattern,
            out,
            frames=frames,
            samples=samples,
            resolution=resolution,
            ground=ground,
            fps=fps,
        )
        _emit(info)


class Aphrody:
    """Keyless access to Gemini, Cloud Code and Vertex AI (no API key).

    Credentials are read from the Antigravity OAuth token already present on
    the machine and refreshed transparently.
    """

    def version(self) -> None:
        """Print the aphrody version."""
        _emit(__version__)

    def whoami(self) -> None:
        """Show the signed-in Google account (email + name)."""
        with AphrodyClient.from_credential_manager() as client:
            info = client.userinfo()
        _emit({"email": info.get("email"), "name": info.get("name")})

    def token(self) -> None:
        """Show token status (scopes, expiry) WITHOUT revealing the token."""
        with httpx.Client(timeout=30.0) as http:
            token = credentials.load_token(http=http)
            info = oauth.tokeninfo(token.access_token, http=http)
        _emit(
            {
                "email": info.get("email"),
                "client_id": info.get("aud"),
                "expires_in_seconds": info.get("expires_in"),
                "expiry": token.expiry,
                "scopes": info.get("scope", "").split(),
                "has_refresh_token": token.refresh_token is not None,
            }
        )

    def models(self) -> None:
        """List Cloud Code models for the account (with a tier fallback).

        ``loadCodeAssist`` resolves the entitlement tier and the user's
        Code Assist project; ``fetchAvailableModels`` is then queried for that
        project. Consumer tiers gate ``fetchAvailableModels`` (HTTP 403); in
        that case the working tier summary is returned instead of failing.
        """
        endpoint = endpoints.CloudCodeEndpoint.PROD
        with AphrodyClient.from_credential_manager(
            cloud_code_endpoint=endpoint
        ) as client:
            info = client.load_code_assist(
                {"metadata": {"pluginType": "GEMINI"}}
            )
            project = info.get("cloudaicompanionProject")
            try:
                body = {"project": project} if project else {}
                _emit(client.fetch_available_models(body))
            except ApiError as exc:
                tier = info.get("currentTier", {})
                paid = info.get("paidTier", {})
                _emit(
                    {
                        "tier": tier.get("name"),
                        "tier_id": tier.get("id"),
                        "paid_tier": paid.get("name"),
                        "cloudaicompanion_project": project,
                        "note": (
                            f"fetchAvailableModels denied (HTTP {exc.status}); "
                            "this tier gates the model list. Showing the tier "
                            "from loadCodeAssist instead."
                        ),
                    }
                )

    def chat(self, prompt: str, model: str = vertex.DEFAULT_MODEL) -> None:
        """Generate a Gemini response for ``prompt`` via Vertex AI.

        Args:
            prompt: The text prompt.
            model: The Gemini model id.
        """
        _emit(vertex.GeminiVertex(model=model).generate(prompt))

    def autocomplete(
        self,
        prefix: str | None = None,
        file: str | None = None,
        suffix: str = "",
        line: int | None = None,
        col: int | None = None,
        lang: str | None = None,
        marker: str | None = None,
        n: int = 1,
        model: str = vertex.DEFAULT_MODEL,
        stream: bool = False,
        auto: bool = False,
        write: bool = False,
        dry_run: bool = False,
    ) -> None:
        """Keyless automatic code completion ("auto-recomplete").

        Predicts the code at a cursor with Gemini (keyless, via Vertex AI) and
        prints structured completions as JSON. Provide either ``--prefix`` (a
        code fragment to continue) or ``--file`` (with ``--line``/``--col`` or a
        cursor marker). With ``--suffix`` (or surrounding file context) it does
        fill-in-the-middle. ``--auto`` walks files and fills the cursor marker
        (``<|cursor|>``) in each; pair with ``--write`` to apply in place.

        Args:
            prefix: Code before the cursor (continuation mode).
            file: Source file to complete inside (mutually exclusive with
                ``--prefix``). In ``--auto`` mode, the file(s) to fill.
            suffix: Code after the cursor (enables fill-in-the-middle).
            line: 1-based cursor line within ``--file``.
            col: 1-based cursor column within the line.
            lang: Language hint (e.g. ``python``); inferred from ``--file``.
            marker: Cursor marker substring (default ``<|cursor|>`` for files).
            n: Number of candidates to return.
            model: Gemini model id.
            stream: Stream the first candidate token-by-token (low TTFT).
            auto: Auto-fill the cursor marker in the file(s) (batch loop).
            write: In ``--auto`` mode, write completions back to disk.
            dry_run: Do not call the model; echo the resolved request (offline
                smoke test, burns no live quota).
        """
        from aphrody import autocomplete as ac

        if dry_run:
            _emit(
                _autocomplete_dry_run(
                    prefix, file, suffix, line, col, lang, marker, n, model
                )
            )
            return

        if auto:
            if not file:
                raise AphrodyError("--auto requires --file")
            results = ac.recomplete_paths(
                [file],
                marker=marker or ac.DEFAULT_CURSOR_MARKER,
                model=model,
                write=write,
            )
            _emit({"auto": True, "write": write, "results": results})
            return

        if stream:
            completer = ac.CodeCompleter(model=model)
            if file is not None:
                text = Path(file).read_text(encoding="utf-8")
                eff_marker = (
                    marker if marker is not None else ac.DEFAULT_CURSOR_MARKER
                )
                pre, suf = ac.split_at_cursor(
                    text, line=line, col=col, marker=eff_marker
                )
                req = ac.CompletionRequest(
                    prefix=pre,
                    suffix=suf,
                    language=lang or ac.language_for_path(file),
                    path=file,
                )
            else:
                req = ac.CompletionRequest(
                    prefix=prefix or "", suffix=suffix, language=lang
                )
            for delta in completer.stream(req):
                sys.stdout.write(delta)
                sys.stdout.flush()
            sys.stdout.write("\n")
            return

        candidates = ac.complete(
            prefix=prefix,
            suffix=suffix,
            file=file,
            line=line,
            col=col,
            language=lang,
            marker=marker,
            n=n,
            model=model,
        )
        _emit(
            {
                "model": model,
                "count": len(candidates),
                "candidates": [c.to_dict() for c in candidates],
            }
        )

    # ``recomplete`` is an alias for ``autocomplete`` (the auto loop).
    recomplete = autocomplete

    def web(self, prompt: str) -> None:
        """Ask the Gemini *web app* (gemini.google.com) via session cookies.

        The keyless cookie path: the same Boq backend the browser talks to,
        with no API key and no OAuth token — only the stored Google cookies.

        Args:
            prompt: The message to send.
        """
        with gemini_web.GeminiWebClient() as client:
            _emit(client.generate(prompt, keep_context=False))

    def cookies(self) -> _CookieCommands:
        """Manage the Google cookie jar: ``status`` / ``load`` / ``extract``."""
        return _CookieCommands()

    def image(self) -> _ImageCommands:
        """Nano Banana Pro image suite.

        Subcommands: ``gen`` / ``edit`` / ``compose`` / ``optimize`` /
        ``batch`` / ``prompts`` / ``template`` / ``enhance`` / ``analyze`` /
        ``anim`` / ``to3d`` / ``icon`` / ``models``.
        """
        return _ImageCommands()

    def blender(self) -> _BlenderCommands:
        """Drive a running Blender via the blender-mcp addon socket.

        Subcommands: ``scene`` / ``exec`` / ``import_glb`` / ``render`` /
        ``turntable``.
        """
        return _BlenderCommands()

    def serve(
        self,
        root: str,
        host: str = "0.0.0.0",
        port: int = 8080,
        spa: bool = True,
        cache: bool = False,
        proxy: str | None = None,
        proxy_prefix: str = "/api",
    ) -> None:
        """Serve a built static / SPA site (e.g. a React ``dist``) — VPS-ready.

        Pure-stdlib threaded server with single-page-app fallback, optional
        in-RAM caching, and an optional reverse proxy (front a Rust/other
        backend from one process). Designed to run under systemd (see ``deploy/``).

        Args:
            root: Directory of the built site to serve.
            host: Bind address (``0.0.0.0`` for a VPS).
            port: Bind port.
            spa: Serve ``index.html`` for unknown routes (client-side routing).
            cache: Preload every file into RAM for zero-disk-hit serving.
            proxy: Backend base URL (e.g. ``http://127.0.0.1:3000``) to forward
                ``proxy_prefix`` requests to (e.g. a Rust API).
            proxy_prefix: URL prefix routed to the backend (default ``/api``).
        """
        from aphrody.serve import serve as _serve

        _serve(
            root,
            host,
            port,
            spa=spa,
            cache=cache,
            proxy=proxy,
            proxy_prefix=proxy_prefix,
        )

    def voice(
        self,
        host: str = "127.0.0.1",
        port: int = 8789,
        whisper_model: str = "base",
        kokoro_model: str | None = None,
        voices_path: str | None = None,
        voice_name: str = "ff_siwis",
        models_dir: str | None = None,
        ui: bool = True,
        ui_port: int = 8790,
    ) -> None:
        """Start the local voice-to-voice WebSocket server (offline Whisper + Kokoro) and serve the UI.

        Args:
            host: Host address to bind to (default "127.0.0.1").
            port: Port to run the WebSocket server on (default 8789).
            whisper_model: Size or path of the Whisper model (default "base").
            kokoro_model: Path to Kokoro ONNX file.
            voices_path: Path to voices.json configuration.
            voice_name: Default Kokoro voice name (default "ff_siwis").
            models_dir: Directory to store downloaded models (default ~/.aphrody/models).
            ui: Launch the web UI automatically (default True).
            ui_port: Port to run the web UI server on if --ui is True (default 8790).
        """
        import asyncio

        from aphrody.voice_server import start_voice_server

        try:
            asyncio.run(
                start_voice_server(
                    host=host,
                    port=port,
                    whisper_model=whisper_model,
                    kokoro_model=kokoro_model,
                    voices_path=voices_path,
                    voice_name=voice_name,
                    models_dir=models_dir,
                    ui=ui,
                    ui_port=ui_port,
                )
            )
        except KeyboardInterrupt:
            print("\nExiting voice server...")


def _as_list(value: Any) -> list:
    """Wrap a scalar in a list; pass lists through unchanged."""
    return value if isinstance(value, list) else [value]


def main() -> None:
    """Entry point for the ``aphrody`` console script."""
    import fire

    # LLM-first: always emit UTF-8, regardless of the host console code page
    # (Windows consoles default to cp1252 and would mangle accented replies).
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8")  # type: ignore[attr-defined]
        except (AttributeError, ValueError):
            pass

    try:
        fire.Fire(Aphrody, name="aphrody")
    except AphrodyError as exc:
        print(f"error: {exc}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
