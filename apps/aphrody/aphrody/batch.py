# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Batch image generation — drive Nano Banana Pro from a declarative spec.

Generate many images in one run from a JSON spec (or a list of
:class:`BatchItem`), each optionally built from a prompt template, passed
through the prompt enhancer, and post-processed by the optimiser. Generation
runs concurrently (image calls are I/O-bound) with a bounded worker pool, and a
JSON manifest of the run is written next to the outputs.

Spec format (JSON)::

    {
      "defaults": {"image_size": "2K", "aspect_ratio": "1:1",
                   "optimize": ["png", "webp"]},
      "items": [
        {"id": "hero", "prompt": "a glowing banana", "image_size": "4K",
         "aspect_ratio": "16:9"},
        {"id": "logo", "template": "logo",
         "vars": {"brand_name": "Aphrody", "industry": "developer tools",
                  "logo_concept": "abstract orbit", "font_style": "geometric sans",
                  "color_palette": "indigo and white"}},
        {"id": "cat", "prompt": "a cat on a sofa", "enhance": "photoreal"}
      ]
    }

    >>> from aphrody.batch import load_spec, generate_batch
    >>> defaults, items = load_spec("shots.json")          # doctest: +SKIP
    >>> results = generate_batch(items, out_dir="out", defaults=defaults)  # doctest: +SKIP
"""

from __future__ import annotations

import json
import logging
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from aphrody import prompts
from aphrody.images import NanoBanana

logger = logging.getLogger(__name__)

#: Default number of concurrent generation workers.
DEFAULT_WORKERS = 3


@dataclass
class BatchItem:
    """One image to generate within a batch run.

    Exactly one of *prompt* or *template* must be provided.

    Attributes:
        id: Stable identifier, used for the output filename.
        prompt: A literal prompt (mutually exclusive with *template*).
        template: A :mod:`aphrody.prompts` template id (filled with *vars*).
        vars: Placeholder values for *template*.
        enhance: Optional style preset applied via
            :func:`aphrody.prompts.enhance_prompt`.
        aspect_ratio: Output aspect ratio.
        image_size: Output resolution (``"1K"``/``"2K"``/``"4K"``).
        negative_prompt: Negative constraints.
        grounding: Enable Google Search grounding.
        n: Number of images to generate for this item.
        optimize: Formats to additionally emit (``"png"``/``"webp"``/``"avif"``).
    """

    id: str
    prompt: str | None = None
    template: str | None = None
    vars: dict[str, str] = field(default_factory=dict)
    enhance: str | None = None
    aspect_ratio: str | None = None
    image_size: str | None = None
    negative_prompt: str | None = None
    grounding: bool = False
    n: int = 1
    optimize: tuple[str, ...] = ()

    def resolve_prompt(self) -> str:
        """Build the final prompt string for this item.

        Returns:
            The prompt: either *prompt* or the rendered *template*, with the
            *enhance* preset applied when set.

        Raises:
            ValueError: If neither or both of *prompt*/*template* are set.
        """
        if bool(self.prompt) == bool(self.template):
            raise ValueError(
                f"item {self.id!r}: provide exactly one of 'prompt' or 'template'"
            )
        text = (
            self.prompt
            if self.prompt
            else prompts.render_template(self.template, **self.vars)  # type: ignore[arg-type]
        )
        if self.enhance:
            text = prompts.enhance_prompt(text, preset=self.enhance)
        return text


@dataclass
class BatchResult:
    """Outcome of generating one :class:`BatchItem`.

    Attributes:
        id: The item id.
        prompt: The resolved prompt that was sent.
        paths: Output file paths produced (primary PNG plus optimised variants).
        model: The model id that actually produced the image.
        error: Error message if the item failed, else ``None``.
    """

    id: str
    prompt: str
    paths: list[Path] = field(default_factory=list)
    model: str | None = None
    error: str | None = None

    @property
    def ok(self) -> bool:
        """Return ``True`` if the item produced at least one image."""
        return self.error is None and bool(self.paths)


def _coerce_item(raw: dict[str, Any]) -> BatchItem:
    """Build a :class:`BatchItem` from a raw spec dict, validating keys."""
    allowed = set(BatchItem.__dataclass_fields__)
    unknown = set(raw) - allowed
    if unknown:
        raise ValueError(f"unknown item keys: {', '.join(sorted(unknown))}")
    if "id" not in raw:
        raise ValueError("each item requires an 'id'")
    data = dict(raw)
    if "optimize" in data and data["optimize"] is not None:
        data["optimize"] = tuple(data["optimize"])
    return BatchItem(**data)


def load_spec(path: str | Path) -> tuple[dict[str, Any], list[BatchItem]]:
    """Load a batch spec from a JSON file.

    Args:
        path: Path to the JSON spec.

    Returns:
        A ``(defaults, items)`` tuple where *defaults* is the optional defaults
        mapping and *items* is the list of :class:`BatchItem`.

    Raises:
        ValueError: If the spec is malformed.
    """
    doc = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(doc, dict) or "items" not in doc:
        raise ValueError("spec must be an object with an 'items' array")
    defaults = doc.get("defaults", {}) or {}
    items = [_coerce_item(it) for it in doc["items"]]
    return defaults, items


def _apply_defaults(item: BatchItem, defaults: dict[str, Any]) -> None:
    """Fill unset *item* fields from *defaults* in place."""
    for key in ("aspect_ratio", "image_size", "negative_prompt"):
        if getattr(item, key) is None and key in defaults:
            setattr(item, key, defaults[key])
    if not item.optimize and defaults.get("optimize"):
        item.optimize = tuple(defaults["optimize"])
    if not item.grounding and defaults.get("grounding"):
        item.grounding = bool(defaults["grounding"])


def _optimize_outputs(png_path: Path, formats: tuple[str, ...]) -> list[Path]:
    """Optimise/convert *png_path* into the requested *formats*.

    Args:
        png_path: The freshly written PNG.
        formats: Subset of ``{"png", "webp", "avif"}``.

    Returns:
        The list of resulting paths (always includes *png_path*).
    """
    from aphrody import optimize as opt

    produced = [png_path]
    want_png = "png" in formats
    want_webp = "webp" in formats
    want_avif = "avif" in formats
    if not (want_png or want_webp or want_avif):
        return produced

    data = png_path.read_bytes()
    result = opt.optimize_all(
        data, png=want_png, webp=want_webp, avif=want_avif
    )
    if "png" in result.outputs:
        png_path.write_bytes(result.outputs["png"])
    if "webp" in result.outputs:
        wp = png_path.with_suffix(".webp")
        wp.write_bytes(result.outputs["webp"])
        produced.append(wp)
    if "avif" in result.outputs:
        ap = png_path.with_suffix(".avif")
        ap.write_bytes(result.outputs["avif"])
        produced.append(ap)
    logger.info("optimised %s: %s", png_path.name, result.summary())
    return produced


def generate_batch(
    items: list[BatchItem],
    *,
    out_dir: str | Path,
    defaults: dict[str, Any] | None = None,
    model: str | None = None,
    max_workers: int = DEFAULT_WORKERS,
    write_manifest: bool = True,
) -> list[BatchResult]:
    """Generate every item concurrently and optionally optimise the outputs.

    Each item is rendered to ``<out_dir>/<id>.png`` (or ``<id>_<k>.png`` when
    ``n > 1``); requested ``optimize`` formats are written alongside. A
    per-thread :class:`~aphrody.images.NanoBanana` is used so credentials are
    read once per worker, never shared across threads.

    Args:
        items: The items to generate.
        out_dir: Destination directory (created if missing).
        defaults: Defaults applied to each item before generation.
        model: Override the image model for the whole run.
        max_workers: Maximum concurrent generation calls.
        write_manifest: Write ``manifest.json`` summarising the run.

    Returns:
        A list of :class:`BatchResult`, one per item, in input order.
    """
    defaults = defaults or {}
    out_path = Path(out_dir)
    out_path.mkdir(parents=True, exist_ok=True)
    run_model = model or defaults.get("model")

    local = threading.local()

    def _client() -> NanoBanana:
        nb = getattr(local, "nb", None)
        if nb is None:
            nb = NanoBanana(model=run_model)
            local.nb = nb
        return nb

    def _run(index: int, item: BatchItem) -> tuple[int, BatchResult]:
        _apply_defaults(item, defaults)
        try:
            prompt = item.resolve_prompt()
        except (ValueError, KeyError) as exc:
            return index, BatchResult(item.id, "", error=str(exc))
        result = BatchResult(item.id, prompt)
        try:
            nb = _client()
            dest = out_path / f"{item.id}.png" if item.n == 1 else out_path
            paths = nb.generate_image(
                prompt,
                out=dest,
                n=item.n,
                aspect_ratio=item.aspect_ratio,
                image_size=item.image_size,
                negative_prompt=item.negative_prompt,
                grounding=item.grounding,
            )
            result.model = nb.last_model
            for p in paths:
                result.paths.extend(_optimize_outputs(Path(p), item.optimize))
        except Exception as exc:
            result.error = f"{type(exc).__name__}: {exc}"
            logger.warning("item %s failed: %s", item.id, result.error)
        return index, result

    results: list[BatchResult | None] = [None] * len(items)
    with ThreadPoolExecutor(max_workers=max(1, max_workers)) as pool:
        futures = [pool.submit(_run, i, it) for i, it in enumerate(items)]
        for fut in as_completed(futures):
            index, res = fut.result()
            results[index] = res
            status = "ok" if res.ok else f"FAIL ({res.error})"
            logger.info("[%d/%d] %s: %s", index + 1, len(items), res.id, status)

    final = [r for r in results if r is not None]
    if write_manifest:
        _write_manifest(out_path / "manifest.json", final, run_model)
    return final


def _write_manifest(
    path: Path, results: list[BatchResult], model: str | None
) -> None:
    """Write a JSON manifest summarising a batch run to *path*."""
    doc = {
        "model": model,
        "total": len(results),
        "ok": sum(1 for r in results if r.ok),
        "failed": sum(1 for r in results if not r.ok),
        "items": [
            {
                "id": r.id,
                "prompt": r.prompt,
                "model": r.model,
                "paths": [str(p) for p in r.paths],
                "error": r.error,
            }
            for r in results
        ],
    }
    path.write_text(
        json.dumps(doc, indent=2, ensure_ascii=False), encoding="utf-8"
    )
    logger.info("wrote manifest -> %s", path)
