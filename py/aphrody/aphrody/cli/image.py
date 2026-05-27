# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Image generation and editing command group for the aphrody CLI."""

from __future__ import annotations

from typing import Any

from aphrody.cli.utils import (
    _as_list,
    _emit,
    _image_aspect_ratios,
    _image_sizes,
    _parse_formats,
)
from aphrody.errors import AphrodyError


class ImageCommands:
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

    def anim(self) -> Any:
        """Animation suite (build / turntable / spritesheet)."""
        from aphrody.cli.anim import AnimCommands

        return AnimCommands()

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

    def icon(self) -> Any:
        """Material 3 icon suite (gen / from_image / svg2ico / convert / css)."""
        from aphrody.cli.icon import IconCommands

        return IconCommands()

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
