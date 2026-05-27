# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Identity-preserving action animations for a character sprite.

Combines the two strongest aphrody creations: Nano Banana Pro's
**identity-preserving** image editing (same character, new pose) and the
**3D multi-view billboard** render. Given one full-body sprite, it generates the
character performing a set of actions (walk, run, jump, crouch, fly, kick a
ball), each as a short motion cycle, and assembles looping animations — which the
``aphrody blender multiview`` 3D pipeline can then render on the GPU with a
ground + shadow.

    >>> from aphrody.sprite_anim import generate_actions
    >>> generate_actions("assets/aphrody-body.webp", "var/actions")   # doctest: +SKIP
"""

from __future__ import annotations

import logging
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence

logger = logging.getLogger(__name__)

#: The exact look to hold constant across every generated pose.
IDENTITY = (
    "the EXACT same anime character with 100% identical identity — same "
    "olive-green long flowing hair, same red eyes, white tunic with a grey "
    "diagonal sash, white shorts, dark knee-high socks and blue-white sneakers, "
    "same cel-shaded art style and body proportions"
)

#: Each action maps to an ordered list of motion-phase prompts (one per frame).
ACTIONS: dict[str, list[str]] = {
    "walk": [
        "walking, mid-step with the left foot forward, relaxed arms",
        "walking, passing pose with feet close together, upright",
        "walking, mid-step with the right foot forward, relaxed arms",
    ],
    "run": [
        "sprinting hard, leaning forward, left leg driving back, arms pumping",
        "sprinting, airborne mid-stride with both knees bent, full of motion",
        "sprinting, right leg forward about to land, arms pumping",
    ],
    "jump": [
        "crouching low to jump, knees deeply bent, arms back",
        "launching upward off the ground, arms thrown up, body stretching",
        "at the peak of a high jump, legs tucked, joyful",
    ],
    "crouch": [
        "beginning to crouch down, knees bending, leaning forward",
        "in a deep low crouch, hands near the ground, compact",
        "rising back up from the crouch, mid-motion",
    ],
    "fly": [
        "taking off into flight, arms forward like a superhero, legs trailing",
        "flying fast horizontally, hair and clothes streaming back, dynamic",
        "soaring upward through the air, triumphant, looking up",
    ],
    "kick_ball": [
        "winding up to kick, kicking leg pulled back, a white soccer ball ahead",
        "striking a white soccer ball powerfully with the foot, ball blurring away",
        "follow-through after kicking the soccer ball, balanced on one leg",
    ],
}


def build_action_prompt(phase: str) -> str:
    """Build an identity-preserving edit instruction for one motion *phase*.

    Args:
        phase: A short description of the pose / motion phase.

    Returns:
        The full editing instruction string.
    """
    return (
        f"Redraw this image keeping {IDENTITY}, but show the character {phase}. "
        "Full body, centered, plain solid white background, dynamic anime pose, "
        "clean cel-shaded line art, consistent character design."
    )


def _edit_with_retry(
    nb: object,
    base_image: str | Path,
    prompt: str,
    dest: Path,
    image_size: str,
    *,
    max_retries: int,
    retry_delay: float,
) -> None:
    """Run one identity-preserving edit, backing off on 429 quota errors."""
    import time

    last: Exception | None = None
    for attempt in range(max_retries):
        try:
            nb.edit_image(base_image, prompt, out=dest, image_size=image_size)  # type: ignore[attr-defined]
            return
        except Exception as exc:  # narrow to quota below, re-raise otherwise
            code = getattr(exc, "code", None)
            quota = code == 429 or "RESOURCE_EXHAUSTED" in str(exc)
            if quota and attempt < max_retries - 1:
                logger.warning(
                    "quota 429 — backing off %.0fs (attempt %d/%d)",
                    retry_delay,
                    attempt + 1,
                    max_retries,
                )
                time.sleep(retry_delay)
                last = exc
                continue
            raise
    if last is not None:  # pragma: no cover - loop always returns or raises
        raise last


def generate_action_frames(
    base_image: str | Path,
    action: str,
    out_dir: str | Path,
    *,
    image_size: str = "1K",
    model: str | None = None,
    phases: Sequence[str] | None = None,
    skip_existing: bool = True,
    delay: float = 0.0,
    max_retries: int = 4,
    retry_delay: float = 20.0,
) -> list[Path]:
    """Generate the motion frames of one *action* from *base_image*.

    Each frame is an identity-preserving Nano Banana Pro edit. Frames are named
    ``<action>_r{i}.png`` so the 3D ``multiview`` pipeline (which orders by the
    ``_r<n>`` index) can consume them directly. Resumable and quota-resilient.

    Args:
        base_image: The reference full-body sprite (defines the identity).
        action: Action key (used for filenames; phases come from *phases* or
            :data:`ACTIONS`).
        out_dir: Destination directory.
        image_size: Output resolution (``1K``/``2K``/``4K``).
        model: Image model id override.
        phases: Explicit motion-phase prompts; defaults to ``ACTIONS[action]``.
        skip_existing: Reuse already-generated frames (resume after a failure).
        delay: Seconds to wait between calls (pace under per-minute quota).
        max_retries: Attempts per frame on a 429 quota error.
        retry_delay: Seconds to back off between quota retries.

    Returns:
        The generated frame paths, in motion order.

    Raises:
        KeyError: If *action* is unknown and no *phases* are given.
    """
    import time

    from aphrody.images import NanoBanana

    phase_list = list(phases) if phases is not None else ACTIONS[action]
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    nb = NanoBanana(model=model)

    frames: list[Path] = []
    for i, phase in enumerate(phase_list):
        dest = out / f"{action}_r{i}.png"
        if skip_existing and dest.exists() and dest.stat().st_size > 0:
            frames.append(dest)
            continue
        logger.info("generating %s frame %d/%d", action, i + 1, len(phase_list))
        _edit_with_retry(
            nb,
            base_image,
            build_action_prompt(phase),
            dest,
            image_size,
            max_retries=max_retries,
            retry_delay=retry_delay,
        )
        frames.append(dest)
        if delay > 0:
            time.sleep(delay)
    return frames


def build_action_loop(
    frames: Sequence[str | Path],
    out: str | Path,
    *,
    fps: float = 6.0,
    pingpong: bool = True,
) -> Path:
    """Assemble *frames* into a looping animated WebP.

    Args:
        frames: Ordered frame paths.
        out: Destination ``.webp`` path.
        fps: Playback frame rate.
        pingpong: Back-and-forth loop (smooths short cycles).

    Returns:
        The written ``.webp`` ``Path``.
    """
    from aphrody import anim

    anim.build_animation(
        [str(f) for f in frames], out, fmt="webp", fps=fps, pingpong=pingpong
    )
    return Path(out)


def generate_actions(
    base_image: str | Path,
    out_dir: str | Path,
    *,
    actions: Sequence[str] | None = None,
    image_size: str = "1K",
    model: str | None = None,
    fps: float = 6.0,
    skip_existing: bool = True,
    delay: float = 0.0,
) -> dict[str, object]:
    """Generate every action's frames + loop, plus a combined showreel.

    Args:
        base_image: The reference full-body sprite.
        out_dir: Destination directory.
        actions: Subset of :data:`ACTIONS` to generate (default: all).
        image_size: Output resolution.
        model: Image model id override.
        fps: Playback frame rate for the loops.
        skip_existing: Reuse already-generated frames (resume after a failure).
        delay: Seconds between calls (pace under a per-minute quota).

    Returns:
        A manifest ``{action: {frames, loop}, "showreel": path}``.
    """
    chosen = list(actions) if actions is not None else list(ACTIONS)
    out = Path(out_dir)
    manifest: dict[str, object] = {}
    all_frames: list[Path] = []

    for action in chosen:
        frames = generate_action_frames(
            base_image,
            action,
            out,
            image_size=image_size,
            model=model,
            skip_existing=skip_existing,
            delay=delay,
        )
        loop = build_action_loop(frames, out / f"{action}.webp", fps=fps)
        manifest[action] = {
            "frames": [str(f) for f in frames],
            "loop": str(loop),
        }
        all_frames.extend(frames)

    if all_frames:
        showreel = build_action_loop(
            all_frames, out / "showreel.webp", fps=fps, pingpong=False
        )
        manifest["showreel"] = str(showreel)
    return manifest
