# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
#
# Provenance: image generation / editing logic adapted from
#   https://github.com/zhongweili/nanobanana-mcp-server (MIT License, © 2025 Zhongwei Li)
#   Ported to the keyless Vertex AI path (no API key); original design patterns
#   for response_modalities, inline_data extraction, and edit flow are preserved.
"""Nano-banana image generation and editing via keyless Vertex AI.

This module provides text-to-image generation and image editing backed by
``gemini-2.5-flash-image`` on Vertex AI, authenticated through the Antigravity
OAuth token — **no API key required**.

The primary entry-point for scripted use is the module-level
:func:`generate_image` convenience function.  For more control use
:class:`NanoBanana` directly.

Model selection order:
1. ``model`` constructor argument
2. ``APHRODY_IMAGE_MODEL`` environment variable
3. :data:`DEFAULT_IMAGE_MODEL` (``gemini-2.5-flash-image``)

Example:
    >>> from aphrody.images import generate_image
    >>> paths = generate_image("a red banana, studio photo", out="out/banana.png")
    >>> print(paths[0])
    out/banana.png
"""

from __future__ import annotations

import logging
import os
import time
from pathlib import Path
from typing import TYPE_CHECKING, Union

if TYPE_CHECKING:
    pass

logger = logging.getLogger(__name__)

#: Default Gemini image model (overridable via ``APHRODY_IMAGE_MODEL``).
DEFAULT_IMAGE_MODEL = "gemini-2.5-flash-image"

# Type alias for flexible image input accepted by edit_image.
_ImageInput = Union[str, "Path", bytes]


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


def _image_bytes_to_parts(
    client: object, raw_bytes: bytes, mime_type: str = "image/png"
) -> list:
    """Convert raw image bytes into a ``google.genai.types.Part`` list.

    Args:
        client: An initialised ``google.genai.Client`` (used only for type access).
        raw_bytes: The image data.
        mime_type: MIME type of the image (e.g. ``"image/png"``).

    Returns:
        A single-element list containing a ``Part`` built from the bytes.
    """
    from google.genai import types as gx

    part = gx.Part.from_bytes(data=raw_bytes, mime_type=mime_type)
    return [part]


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


def _write_png(data: bytes, dest: Path) -> None:
    """Write *data* to *dest*, creating parent directories as needed.

    Uses an atomic rename via a ``.tmp`` sibling to avoid partial writes.

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
    # If the caller gave an explicit file path *and* expects a single image,
    # use it directly.  For multiple images or a directory, synthesise names.
    if n_images == 1 and not p.is_dir() and p.suffix:
        return [p]

    # Treat as directory.
    base_dir = p if (p.is_dir() or not p.suffix) else p.parent
    ts = time.strftime("%Y%m%d_%H%M%S")
    return [base_dir / f"{prefix}_{ts}_{i + 1}.png" for i in range(n_images)]


def _load_image_bytes(image: _ImageInput) -> bytes:
    """Load image bytes from a path string, ``Path``, or raw bytes.

    Args:
        image: Source image — a filesystem path (``str`` or ``Path``) or
            already-loaded ``bytes``.

    Returns:
        Raw image bytes.
    """
    if isinstance(image, bytes):
        return image
    return Path(image).read_bytes()


def _detect_mime(image: _ImageInput) -> str:
    """Detect the MIME type of *image*.

    Falls back to ``image/png`` when detection fails.

    Args:
        image: Source image input (path or bytes).

    Returns:
        A MIME type string such as ``"image/jpeg"``.
    """
    import mimetypes

    if isinstance(image, bytes):
        return "image/png"
    mime, _ = mimetypes.guess_type(str(image))
    if mime and mime.startswith("image/"):
        return mime
    return "image/png"


class NanoBanana:
    """Keyless Gemini image generation and editing via Vertex AI.

    Wraps a :class:`~aphrody.vertex.GeminiVertex` client and exposes
    high-level ``generate_image`` / ``edit_image`` methods.

    Example:
        >>> nb = NanoBanana()
        >>> paths = nb.generate_image("a glowing neon banana", out="out/neon.png")
    """

    def __init__(
        self,
        *,
        model: str | None = None,
        project: str | None = None,
        location: str | None = None,
    ) -> None:
        """Initialise the Nano-banana client.

        Args:
            model: Image model id. Defaults to :data:`DEFAULT_IMAGE_MODEL`
                (overridable via ``APHRODY_IMAGE_MODEL``).
            project: Vertex AI project id (forwarded to
                :class:`~aphrody.vertex.GeminiVertex`).
            location: Vertex AI region.
        """
        from aphrody.vertex import GeminiVertex

        self._model = _resolve_model(model)
        self._vertex = GeminiVertex(
            project=project, location=location, model=self._model
        )
        self._client = self._vertex.client

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
        negative_prompt: str | None = None,
    ) -> list[Path] | list[bytes]:
        """Generate one or more images from a text prompt.

        Calls ``generate_content`` with ``response_modalities=["TEXT","IMAGE"]``
        and collects every image part from the response.  When *out* is given
        the images are written to disk as PNG files and their ``Path`` objects
        are returned; otherwise raw ``bytes`` are returned.

        Args:
            prompt: The generation prompt.
            out: Output file or directory path (optional).  If ``None``,
                raw ``bytes`` are returned instead of files.
            n: Number of independent generation calls to make.  Each call
                may return one image; total output equals *n* images.
            aspect_ratio: Optional aspect ratio hint passed to ``ImageConfig``
                (e.g. ``"16:9"``, ``"1:1"``).  Ignored if not supported.
            negative_prompt: Optional constraints appended to the prompt.

        Returns:
            A list of ``Path`` objects when *out* is given, or a list of
            ``bytes`` objects otherwise.

        Raises:
            RuntimeError: When no image is returned by the model for any call.
        """
        from google.genai import types as gx

        full_prompt = prompt
        if negative_prompt:
            full_prompt = f"{prompt}\n\nConstraints (avoid): {negative_prompt}"

        image_config_kw: dict = {}
        if aspect_ratio:
            image_config_kw["aspect_ratio"] = aspect_ratio

        cfg = gx.GenerateContentConfig(
            response_modalities=["TEXT", "IMAGE"],
            image_config=gx.ImageConfig(**image_config_kw)
            if image_config_kw
            else None,
        )

        all_data: list[bytes] = []
        for i in range(n):
            logger.debug(
                "nano-banana generate call %d/%d for prompt=%r",
                i + 1,
                n,
                prompt[:60],
            )
            response = self._client.models.generate_content(
                model=self._model,
                contents=full_prompt,
                config=cfg,
            )
            imgs = _extract_images(response)
            if not imgs:
                logger.warning(
                    "nano-banana: call %d/%d returned no image parts (model=%s)",
                    i + 1,
                    n,
                    self._model,
                )
            all_data.extend(imgs)

        if not all_data:
            raise RuntimeError(
                f"nano-banana: no images returned after {n} call(s) "
                f"(model={self._model!r}, prompt={prompt[:80]!r})"
            )

        out_paths = _resolve_outputs(out, len(all_data), "gen")
        results: list[Path] | list[bytes] = []
        for data, dest in zip(all_data, out_paths):
            if dest is not None:
                _write_png(data, dest)
                logger.info(
                    "nano-banana: wrote %d bytes -> %s", len(data), dest
                )
                results.append(dest)  # type: ignore[arg-type]
            else:
                results.append(data)  # type: ignore[arg-type]

        return results

    def edit_image(
        self,
        image: _ImageInput,
        prompt: str,
        *,
        out: str | Path | None = None,
    ) -> Path | bytes:
        """Edit an existing image using a natural-language instruction.

        Sends *image* together with *prompt* to the model and returns the
        edited result.  The call uses ``response_modalities=["TEXT","IMAGE"]``
        so the model can reply with both text and the modified image.

        Args:
            image: Source image — a filesystem path (``str`` or ``Path``) or
                raw image ``bytes``.
            prompt: Editing instruction (e.g. ``"make the background blue"``).
            out: Output file path (optional).  If ``None``, raw ``bytes`` are
                returned.

        Returns:
            A ``Path`` pointing to the written PNG, or raw ``bytes`` when *out*
            is ``None``.

        Raises:
            RuntimeError: When the model returns no image.
        """
        from google.genai import types as gx

        raw = _load_image_bytes(image)
        mime = _detect_mime(image)

        parts = _image_bytes_to_parts(self._client, raw, mime)
        contents = [*parts, prompt]

        cfg = gx.GenerateContentConfig(
            response_modalities=["TEXT", "IMAGE"],
        )

        logger.debug(
            "nano-banana edit: %d-byte image, mime=%s, prompt=%r",
            len(raw),
            mime,
            prompt[:60],
        )

        response = self._client.models.generate_content(
            model=self._model,
            contents=contents,
            config=cfg,
        )
        imgs = _extract_images(response)
        if not imgs:
            raise RuntimeError(
                f"nano-banana: edit returned no image (model={self._model!r}, "
                f"prompt={prompt[:80]!r})"
            )

        data = imgs[0]
        if out is not None:
            dest = Path(out)
            _write_png(data, dest)
            logger.info(
                "nano-banana: edit wrote %d bytes -> %s", len(data), dest
            )
            return dest
        return data

    def compose_images(
        self,
        images: list[_ImageInput],
        prompt: str,
        *,
        out: str | Path | None = None,
    ) -> Path | bytes:
        """Compose or blend multiple input images with a text instruction.

        Sends all *images* as inline parts followed by *prompt*.  Useful for
        style transfer, compositing, or multi-reference generation.

        Args:
            images: List of source images (paths or bytes).  Up to 3 images
                work well; the model has a 20 MB per-request limit.
            prompt: Compositing instruction.
            out: Output path (optional).

        Returns:
            A ``Path`` or raw ``bytes``, same convention as :meth:`edit_image`.

        Raises:
            ValueError: When *images* is empty.
            RuntimeError: When the model returns no image.
        """
        from google.genai import types as gx

        if not images:
            raise ValueError(
                "nano-banana compose_images: at least one image is required"
            )

        parts: list = []
        for img in images:
            raw = _load_image_bytes(img)
            mime = _detect_mime(img)
            parts.extend(_image_bytes_to_parts(self._client, raw, mime))
        contents = [*parts, prompt]

        cfg = gx.GenerateContentConfig(
            response_modalities=["TEXT", "IMAGE"],
        )

        response = self._client.models.generate_content(
            model=self._model,
            contents=contents,
            config=cfg,
        )
        imgs = _extract_images(response)
        if not imgs:
            raise RuntimeError(
                f"nano-banana: compose returned no image (model={self._model!r})"
            )

        data = imgs[0]
        if out is not None:
            dest = Path(out)
            _write_png(data, dest)
            logger.info(
                "nano-banana: compose wrote %d bytes -> %s", len(data), dest
            )
            return dest
        return data


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
    negative_prompt: str | None = None,
) -> list[Path] | list[bytes]:
    """Generate images from *prompt* using a one-shot :class:`NanoBanana`.

    This is the simplest entry-point for script or CLI use.

    Args:
        prompt: The generation prompt.
        out: Output path (file or directory).  ``None`` returns raw bytes.
        n: Number of images to generate.
        model: Override the image model id.
        aspect_ratio: Optional aspect ratio (e.g. ``"16:9"``).
        negative_prompt: Optional negative constraints.

    Returns:
        List of ``Path`` objects (when *out* is set) or ``bytes`` otherwise.
    """
    nb = NanoBanana(model=model)
    return nb.generate_image(
        prompt,
        out=out,
        n=n,
        aspect_ratio=aspect_ratio,
        negative_prompt=negative_prompt,
    )


def edit_image(
    image: _ImageInput,
    prompt: str,
    out: str | Path | None = None,
    *,
    model: str | None = None,
) -> Path | bytes:
    """Edit *image* following *prompt*, one-shot convenience wrapper.

    Args:
        image: Source image (path or bytes).
        prompt: Editing instruction.
        out: Output path.  ``None`` returns raw bytes.
        model: Override the image model id.

    Returns:
        ``Path`` or ``bytes``.
    """
    nb = NanoBanana(model=model)
    return nb.edit_image(image, prompt, out=out)
