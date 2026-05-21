# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
#
# Provenance: image generation / editing logic adapted from
#   https://github.com/zhongweili/nanobanana-mcp-server (MIT License, © 2025 Zhongwei Li)
#   Ported to the keyless Vertex AI path (no API key); original design patterns
#   for response_modalities, inline_data extraction, and edit flow are preserved.
"""Nano Banana / Nano Banana Pro image generation and editing — keyless Vertex.

Text-to-image, editing and multi-image composition backed by Google's image
models on Vertex AI, authenticated through the Antigravity OAuth token — **no
API key required**.

The flagship model is **Nano Banana Pro** (``gemini-3-pro-image-preview``,
Gemini 3 Pro Image): native 1K/2K/4K output, state-of-the-art text rendering,
real-world grounding via Google Search, and up to 14 reference images. If the
account is not entitled to the Pro model the client transparently falls back
down :data:`FALLBACK_CHAIN` to a Flash image model.

Model selection order:
1. ``model`` constructor / function argument
2. ``APHRODY_IMAGE_MODEL`` environment variable
3. :data:`DEFAULT_IMAGE_MODEL` (Nano Banana Pro)

Example:
    >>> from aphrody.images import generate_image
    >>> paths = generate_image(
    ...     "a red banana, studio photo",
    ...     out="out/banana.png",
    ...     image_size="4K",
    ...     aspect_ratio="16:9",
    ... )  # doctest: +SKIP
"""

from __future__ import annotations

import logging
import os
import time
from pathlib import Path
from typing import TYPE_CHECKING, Union

from aphrody.prompts import ASPECT_RATIOS, IMAGE_SIZES, MAX_REFERENCE_IMAGES

if TYPE_CHECKING:
    pass

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Model registry
# ---------------------------------------------------------------------------

#: Nano Banana Pro — Gemini 3 Pro Image. Max fidelity, 1K/2K/4K, typography.
NANO_BANANA_PRO = "gemini-3-pro-image-preview"
#: Nano Banana 2 (Flash) — fast Gemini 3.1 image model with thinking support.
NANO_BANANA_2_FLASH = "gemini-3.1-flash-image-preview"
#: Original Nano Banana — Gemini 2.5 Flash Image, fastest / most widely enabled.
NANO_BANANA_FLASH = "gemini-2.5-flash-image"

#: Default image model (overridable via ``APHRODY_IMAGE_MODEL``).
DEFAULT_IMAGE_MODEL = NANO_BANANA_PRO

#: Ordered degradation path tried when a model is unavailable / unentitled.
FALLBACK_CHAIN: tuple[str, ...] = (
    NANO_BANANA_PRO,
    NANO_BANANA_2_FLASH,
    NANO_BANANA_FLASH,
)

# Type alias for flexible image input accepted by edit/compose.
_ImageInput = Union[str, "Path", bytes]


def _is_pro_family(model: str) -> bool:
    """Return ``True`` if *model* supports Pro-only knobs (e.g. image_size)."""
    return model.startswith("gemini-3") and "image" in model


def _resolve_model(override: str | None = None) -> str:
    """Resolve the image model id to use.

    Args:
        override: An explicit caller-provided model id.

    Returns:
        The resolved model id string.
    """
    return (
        override or os.environ.get("APHRODY_IMAGE_MODEL") or DEFAULT_IMAGE_MODEL
    )


def _fallback_models(primary: str) -> list[str]:
    """Build the ordered model list to try, starting with *primary*.

    Args:
        primary: The first model to attempt.

    Returns:
        ``[primary, *rest_of_chain]`` with duplicates removed, order preserved.
    """
    models = [primary]
    for m in FALLBACK_CHAIN:
        if m not in models:
            models.append(m)
    return models


def _validate_aspect_ratio(aspect_ratio: str | None) -> None:
    """Raise ``ValueError`` if *aspect_ratio* is not a supported value."""
    if aspect_ratio is not None and aspect_ratio not in ASPECT_RATIOS:
        raise ValueError(
            f"aspect_ratio {aspect_ratio!r} unsupported; choose one of "
            f"{', '.join(ASPECT_RATIOS)}"
        )


def _validate_image_size(image_size: str | None) -> None:
    """Raise ``ValueError`` if *image_size* is not 1K/2K/4K."""
    if image_size is not None and image_size not in IMAGE_SIZES:
        raise ValueError(
            f"image_size {image_size!r} unsupported; choose one of "
            f"{', '.join(IMAGE_SIZES)}"
        )


def _image_bytes_to_parts(
    raw_bytes: bytes, mime_type: str = "image/png"
) -> list:
    """Convert raw image bytes into a single-element ``Part`` list.

    Args:
        raw_bytes: The image data.
        mime_type: MIME type of the image (e.g. ``"image/png"``).

    Returns:
        A single-element list containing a ``Part`` built from the bytes.
    """
    from google.genai import types as gx

    return [gx.Part.from_bytes(data=raw_bytes, mime_type=mime_type)]


def _extract_images(response: object) -> list[bytes]:
    """Extract all image byte payloads from a ``generate_content`` response.

    Iterates ``response.candidates[0].content.parts`` and collects every
    ``inline_data.data`` that is present.

    Args:
        response: A ``google.genai`` ``GenerateContentResponse``.

    Returns:
        A (possibly empty) list of raw image bytes, one per image part.
    """
    images: list[bytes] = []
    candidates = getattr(response, "candidates", None) or []
    if not candidates:
        return images
    content = getattr(candidates[0], "content", None)
    if content is None:
        return images
    for part in getattr(content, "parts", []) or []:
        inline = getattr(part, "inline_data", None)
        if inline is not None:
            data = getattr(inline, "data", None)
            if data:
                images.append(data)
    return images


def _response_text(response: object) -> str:
    """Concatenate any text parts from a response (for logging / grounding)."""
    parts_text: list[str] = []
    candidates = getattr(response, "candidates", None) or []
    if not candidates:
        return ""
    content = getattr(candidates[0], "content", None)
    if content is None:
        return ""
    for part in getattr(content, "parts", []) or []:
        text = getattr(part, "text", None)
        if text:
            parts_text.append(text)
    return "".join(parts_text)


def _write_png(data: bytes, dest: Path) -> None:
    """Write *data* to *dest* atomically, creating parent directories.

    Args:
        data: Raw image bytes.
        dest: Destination ``Path``.
    """
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = dest.with_suffix(".tmp")
    try:
        tmp.write_bytes(data)
        tmp.replace(dest)
    except Exception:
        if tmp.exists():
            tmp.unlink(missing_ok=True)
        raise


def _resolve_outputs(
    out: str | Path | None,
    n_images: int,
    prefix: str,
) -> list[Path | None]:
    """Build the list of output paths for *n_images*.

    Args:
        out: Caller-supplied output hint (file path, directory, or ``None``).
        n_images: Number of images expected.
        prefix: Filename prefix used when generating automatic names.

    Returns:
        A list of ``Path`` objects (one per image), or ``[None] * n_images``
        if the caller did not request file output.
    """
    if out is None:
        return [None] * n_images

    p = Path(out)
    if n_images == 1 and not p.is_dir() and p.suffix:
        return [p]

    base_dir = p if (p.is_dir() or not p.suffix) else p.parent
    ts = time.strftime("%Y%m%d_%H%M%S")
    return [base_dir / f"{prefix}_{ts}_{i + 1}.png" for i in range(n_images)]


def _load_image_bytes(image: _ImageInput) -> bytes:
    """Load image bytes from a path string, ``Path``, or raw bytes."""
    if isinstance(image, bytes):
        return image
    return Path(image).read_bytes()


def _detect_mime(image: _ImageInput) -> str:
    """Detect the MIME type of *image*, defaulting to ``image/png``."""
    import mimetypes

    if isinstance(image, bytes):
        return "image/png"
    mime, _ = mimetypes.guess_type(str(image))
    if mime and mime.startswith("image/"):
        return mime
    return "image/png"


class NanoBanana:
    """Keyless Gemini image generation and editing via Vertex AI.

    Wraps a :class:`~aphrody.vertex.GeminiVertex` client and exposes high-level
    ``generate_image`` / ``edit_image`` / ``compose_images`` methods targeting
    Nano Banana Pro by default, with automatic degradation to Flash models.

    Example:
        >>> nb = NanoBanana()  # doctest: +SKIP
        >>> nb.generate_image("a glowing neon banana", out="out/neon.png",
        ...                   image_size="4K")  # doctest: +SKIP
    """

    def __init__(
        self,
        *,
        model: str | None = None,
        project: str | None = None,
        location: str | None = None,
    ) -> None:
        """Initialise the client.

        Args:
            model: Image model id. Defaults to :data:`DEFAULT_IMAGE_MODEL`
                (overridable via ``APHRODY_IMAGE_MODEL``).
            project: Vertex AI project id.
            location: Vertex AI region.
        """
        from aphrody.vertex import GeminiVertex

        self._model = _resolve_model(model)
        if location is None and _is_pro_family(self._model):
            # Nano Banana Pro (Gemini 3 Pro Image) is only served from the
            # `global` Vertex location; regional endpoints return 404.
            location = "global"
        self._vertex = GeminiVertex(
            project=project, location=location, model=self._model
        )
        self._client = self._vertex.client
        #: The model id actually used by the most recent successful call.
        self.last_model: str | None = None

    # ------------------------------------------------------------------
    # Internal generation core
    # ------------------------------------------------------------------

    @staticmethod
    def _build_config(
        model: str,
        *,
        aspect_ratio: str | None,
        image_size: str | None,
        person_generation: str | None,
        grounding: bool,
    ) -> object:
        """Build a ``GenerateContentConfig`` tailored to *model*.

        Pro-only fields (``image_size``) are dropped for Flash models so a
        fallback call never fails on an unsupported field.
        """
        from google.genai import types as gx

        image_config_kw: dict = {}
        if aspect_ratio:
            image_config_kw["aspect_ratio"] = aspect_ratio
        if image_size and _is_pro_family(model):
            image_config_kw["image_size"] = image_size
        if person_generation:
            image_config_kw["person_generation"] = person_generation

        cfg_kw: dict = {"response_modalities": ["TEXT", "IMAGE"]}
        if image_config_kw:
            cfg_kw["image_config"] = gx.ImageConfig(**image_config_kw)
        if grounding:
            cfg_kw["tools"] = [gx.Tool(google_search=gx.GoogleSearch())]
        return gx.GenerateContentConfig(**cfg_kw)

    @staticmethod
    def _is_unavailable_error(exc: Exception) -> bool:
        """Return ``True`` if *exc* indicates the model is missing/unentitled."""
        code = getattr(exc, "code", None)
        if code in (400, 403, 404):
            return True
        text = str(exc).lower()
        return any(
            s in text
            for s in (
                "not found",
                "not_found",
                "permission",
                "not allowed",
                "is not supported",
                "does not exist",
                "not enabled",
            )
        )

    def _generate(
        self, contents: object, config: object, *, models: list[str]
    ) -> list[bytes]:
        """Run one generation, walking *models* until one yields an image.

        Args:
            contents: The ``generate_content`` contents argument.
            config: A ``GenerateContentConfig`` (rebuilt per-model for Pro knobs).
            models: Ordered candidate model ids.

        Returns:
            The image bytes produced by the first model that succeeds.

        Raises:
            RuntimeError: If every candidate model fails or returns no image.
        """
        from google.genai import errors as genai_errors

        last_exc: Exception | None = None
        for model in models:
            try:
                response = self._client.models.generate_content(
                    model=model, contents=contents, config=config
                )
            except genai_errors.APIError as exc:
                if model != models[-1] and self._is_unavailable_error(exc):
                    logger.warning(
                        "model %s unavailable (%s); falling back",
                        model,
                        getattr(exc, "code", "?"),
                    )
                    last_exc = exc
                    continue
                raise
            imgs = _extract_images(response)
            if imgs:
                self.last_model = model
                return imgs
            note = _response_text(response)[:160]
            logger.warning(
                "model %s returned no image%s",
                model,
                f" (said: {note!r})" if note else "",
            )
            last_exc = RuntimeError(f"{model} returned no image part")
        raise RuntimeError(f"no image produced by any of {models}: {last_exc}")

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def generate_image(
        self,
        prompt: str,
        *,
        out: str | Path | None = None,
        n: int = 1,
        aspect_ratio: str | None = None,
        image_size: str | None = None,
        negative_prompt: str | None = None,
        person_generation: str | None = None,
        grounding: bool = False,
    ) -> list[Path] | list[bytes]:
        """Generate one or more images from a text prompt.

        Args:
            prompt: The generation prompt.
            out: Output file or directory path. ``None`` returns raw ``bytes``.
            n: Number of independent generation calls (one image each).
            aspect_ratio: One of :data:`~aphrody.prompts.ASPECT_RATIOS`.
            image_size: ``"1K"``, ``"2K"`` or ``"4K"`` (Nano Banana Pro only).
            negative_prompt: Constraints appended to the prompt.
            person_generation: Vertex people-policy, e.g. ``"allow_adult"``.
            grounding: Enable Google Search grounding for factual images.

        Returns:
            A list of ``Path`` (when *out* is given) or ``bytes`` otherwise.

        Raises:
            ValueError: For an unsupported aspect ratio or image size.
            RuntimeError: When no image is returned by any model.
        """
        _validate_aspect_ratio(aspect_ratio)
        _validate_image_size(image_size)

        full_prompt = prompt
        if negative_prompt:
            full_prompt = f"{prompt}\n\nConstraints (avoid): {negative_prompt}"

        models = _fallback_models(self._model)

        all_data: list[bytes] = []
        for i in range(n):
            cfg = self._build_config(
                models[0],
                aspect_ratio=aspect_ratio,
                image_size=image_size,
                person_generation=person_generation,
                grounding=grounding,
            )
            logger.debug(
                "generate call %d/%d (model=%s) prompt=%r",
                i + 1,
                n,
                models[0],
                prompt[:60],
            )
            all_data.extend(self._generate(full_prompt, cfg, models=models))

        return self._emit(all_data, out, "gen")

    def edit_image(
        self,
        image: _ImageInput,
        prompt: str,
        *,
        out: str | Path | None = None,
        aspect_ratio: str | None = None,
        image_size: str | None = None,
    ) -> Path | bytes:
        """Edit an existing image using a natural-language instruction.

        Args:
            image: Source image — a path (``str``/``Path``) or raw ``bytes``.
            prompt: Editing instruction (e.g. ``"make the background blue"``).
            out: Output file path. ``None`` returns raw ``bytes``.
            aspect_ratio: Optional output aspect ratio.
            image_size: Optional output resolution (Pro only).

        Returns:
            A ``Path`` to the written PNG, or raw ``bytes`` when *out* is None.

        Raises:
            RuntimeError: When the model returns no image.
        """
        _validate_aspect_ratio(aspect_ratio)
        _validate_image_size(image_size)

        raw = _load_image_bytes(image)
        mime = _detect_mime(image)
        contents = [*_image_bytes_to_parts(raw, mime), prompt]
        models = _fallback_models(self._model)
        cfg = self._build_config(
            models[0],
            aspect_ratio=aspect_ratio,
            image_size=image_size,
            person_generation=None,
            grounding=False,
        )
        logger.debug("edit: %d-byte %s, prompt=%r", len(raw), mime, prompt[:60])
        data = self._generate(contents, cfg, models=models)[0]
        return self._emit_one(data, out)

    def compose_images(
        self,
        images: list[_ImageInput],
        prompt: str,
        *,
        out: str | Path | None = None,
        aspect_ratio: str | None = None,
        image_size: str | None = None,
    ) -> Path | bytes:
        """Compose or blend multiple input images with a text instruction.

        Args:
            images: Source images (paths or bytes). Up to
                :data:`~aphrody.prompts.MAX_REFERENCE_IMAGES` (14) are accepted.
            prompt: Compositing instruction with an explicit relationship between
                the references (e.g. "use image 1 as structure, image 2 style").
            out: Output path. ``None`` returns raw ``bytes``.
            aspect_ratio: Optional output aspect ratio.
            image_size: Optional output resolution (Pro only).

        Returns:
            A ``Path`` or raw ``bytes``.

        Raises:
            ValueError: If *images* is empty or exceeds the reference limit.
            RuntimeError: When the model returns no image.
        """
        if not images:
            raise ValueError("compose_images: at least one image is required")
        if len(images) > MAX_REFERENCE_IMAGES:
            raise ValueError(
                f"compose_images: at most {MAX_REFERENCE_IMAGES} reference "
                f"images, got {len(images)}"
            )
        _validate_aspect_ratio(aspect_ratio)
        _validate_image_size(image_size)

        parts: list = []
        for img in images:
            raw = _load_image_bytes(img)
            parts.extend(_image_bytes_to_parts(raw, _detect_mime(img)))
        contents = [*parts, prompt]
        models = _fallback_models(self._model)
        cfg = self._build_config(
            models[0],
            aspect_ratio=aspect_ratio,
            image_size=image_size,
            person_generation=None,
            grounding=False,
        )
        data = self._generate(contents, cfg, models=models)[0]
        return self._emit_one(data, out)

    # ------------------------------------------------------------------
    # Output helpers
    # ------------------------------------------------------------------

    def _emit(
        self, all_data: list[bytes], out: str | Path | None, prefix: str
    ) -> list[Path] | list[bytes]:
        """Write *all_data* to disk (if *out*) or return the raw bytes list."""
        out_paths = _resolve_outputs(out, len(all_data), prefix)
        results: list[Path] | list[bytes] = []
        for data, dest in zip(all_data, out_paths):
            if dest is not None:
                _write_png(data, dest)
                logger.info("wrote %d bytes -> %s", len(data), dest)
                results.append(dest)  # type: ignore[arg-type]
            else:
                results.append(data)  # type: ignore[arg-type]
        return results

    def _emit_one(self, data: bytes, out: str | Path | None) -> Path | bytes:
        """Write a single image to *out* or return the bytes."""
        if out is None:
            return data
        dest = Path(out)
        _write_png(data, dest)
        logger.info("wrote %d bytes -> %s", len(data), dest)
        return dest


# ---------------------------------------------------------------------------
# Module-level convenience functions
# ---------------------------------------------------------------------------


def generate_image(
    prompt: str,
    out: str | Path | None = None,
    *,
    n: int = 1,
    model: str | None = None,
    aspect_ratio: str | None = None,
    image_size: str | None = None,
    negative_prompt: str | None = None,
    person_generation: str | None = None,
    grounding: bool = False,
) -> list[Path] | list[bytes]:
    """Generate images from *prompt* using a one-shot :class:`NanoBanana`.

    Args:
        prompt: The generation prompt.
        out: Output path (file or directory). ``None`` returns raw bytes.
        n: Number of images to generate.
        model: Override the image model id.
        aspect_ratio: One of :data:`~aphrody.prompts.ASPECT_RATIOS`.
        image_size: ``"1K"``/``"2K"``/``"4K"`` (Nano Banana Pro only).
        negative_prompt: Negative constraints.
        person_generation: Vertex people-policy.
        grounding: Enable Google Search grounding.

    Returns:
        List of ``Path`` (when *out* set) or ``bytes`` otherwise.
    """
    nb = NanoBanana(model=model)
    return nb.generate_image(
        prompt,
        out=out,
        n=n,
        aspect_ratio=aspect_ratio,
        image_size=image_size,
        negative_prompt=negative_prompt,
        person_generation=person_generation,
        grounding=grounding,
    )


def edit_image(
    image: _ImageInput,
    prompt: str,
    out: str | Path | None = None,
    *,
    model: str | None = None,
    aspect_ratio: str | None = None,
    image_size: str | None = None,
) -> Path | bytes:
    """Edit *image* following *prompt*, one-shot convenience wrapper.

    Args:
        image: Source image (path or bytes).
        prompt: Editing instruction.
        out: Output path. ``None`` returns raw bytes.
        model: Override the image model id.
        aspect_ratio: Optional output aspect ratio.
        image_size: Optional output resolution (Pro only).

    Returns:
        ``Path`` or ``bytes``.
    """
    nb = NanoBanana(model=model)
    return nb.edit_image(
        image, prompt, out=out, aspect_ratio=aspect_ratio, image_size=image_size
    )


def compose_images(
    images: list[_ImageInput],
    prompt: str,
    out: str | Path | None = None,
    *,
    model: str | None = None,
    aspect_ratio: str | None = None,
    image_size: str | None = None,
) -> Path | bytes:
    """Compose multiple *images* per *prompt*, one-shot convenience wrapper.

    Args:
        images: Source images (paths or bytes), up to 14.
        prompt: Compositing instruction.
        out: Output path. ``None`` returns raw bytes.
        model: Override the image model id.
        aspect_ratio: Optional output aspect ratio.
        image_size: Optional output resolution (Pro only).

    Returns:
        ``Path`` or ``bytes``.
    """
    nb = NanoBanana(model=model)
    return nb.compose_images(
        images,
        prompt,
        out=out,
        aspect_ratio=aspect_ratio,
        image_size=image_size,
    )
