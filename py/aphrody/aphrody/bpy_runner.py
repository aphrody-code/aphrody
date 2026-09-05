# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Run native ``bpy`` scripts through an installed Blender, fully headless.

The other Blender paths need either a *running* Blender (the ``aphrody.blender``
socket bridge) or the heavy pip ``bpy`` wheel (Python 3.13). This module takes
the third route: shell out to an installed **Blender** binary in background mode
(``blender -b --factory-startup -P script.py -- args``), which gives the full
native API (`bpy`, `bmesh`, `mathutils`, `gpu`, …) with **no running Blender, no
add-on install, and no Python-ABI matching**.

Binary resolution order: ``$APHRODY_BLENDER_BIN`` → known install paths →
``blender`` on ``PATH``.

    >>> from aphrody.bpy_runner import run_sprite_to_3d
    >>> run_sprite_to_3d("assets/aphrody.webp", "model.glb", frames=48)  # doctest: +SKIP
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from aphrody.errors import AphrodyError

#: Probed standard Blender binary locations constructed dynamically.
_KNOWN_BINARIES: list[str] = []
if os.name == "nt":
    _pf = os.environ.get("ProgramFiles") or r"C:\Program Files"
    _pf_dir = Path(_pf) / "Blender Foundation"
    for _ver in ("5.1", "5.0", "4.2"):
        _KNOWN_BINARIES.append(str(_pf_dir / f"Blender {_ver}" / "blender.exe"))
else:
    _KNOWN_BINARIES.extend(
        [
            "/usr/bin/blender",
            "/usr/local/bin/blender",
            "/Applications/Blender.app/Contents/MacOS/Blender",
        ]
    )

#: Bundled bpy scripts (repo layout: apps/aphrody/blender_addon/scripts).
_SCRIPTS_DIR = Path(__file__).resolve().parents[1] / "blender_addon" / "scripts"


class BlenderRunnerError(AphrodyError):
    """Raised when Blender cannot be found or a headless run fails."""


def resolve_blender_bin(override: str | None = None) -> str | None:
    """Resolve the Blender executable path.

    Args:
        override: An explicit path (wins if it exists).

    Returns:
        The path to a Blender binary, or ``None`` if none is found.
    """
    for cand in (override, os.environ.get("APHRODY_BLENDER_BIN")):
        if cand and Path(cand).exists():
            return cand

    for cand in _KNOWN_BINARIES:
        if Path(cand).exists():
            return cand
    return shutil.which("blender")


@dataclass(frozen=True)
class RunResult:
    """Result of a headless Blender run.

    Attributes:
        returncode: The process exit code.
        stdout: Captured standard output.
        stderr: Captured standard error.
    """

    returncode: int
    stdout: str
    stderr: str

    @property
    def ok(self) -> bool:
        """Return ``True`` on a zero exit code."""
        return self.returncode == 0


class BlenderRunner:
    """Drives an installed Blender binary in headless background mode."""

    def __init__(self, blender_bin: str | None = None) -> None:
        """Resolve the binary, raising if Blender is not found.

        Args:
            blender_bin: Explicit binary path override.

        Raises:
            BlenderRunnerError: If no Blender binary can be resolved.
        """
        self.bin = resolve_blender_bin(blender_bin)
        if not self.bin:
            raise BlenderRunnerError(
                "no Blender binary found — set $APHRODY_BLENDER_BIN, install "
                "Blender, or put 'blender' on PATH"
            )

    def run_script(
        self,
        script: str | Path,
        args: list[str] | tuple[str, ...] = (),
        *,
        timeout: float = 600.0,
        factory_startup: bool = True,
    ) -> RunResult:
        """Run a ``bpy`` script headlessly: ``blender -b -P script -- args``.

        Args:
            script: Path to the Python script Blender will run.
            args: Arguments passed after ``--`` (read via ``sys.argv``).
            timeout: Seconds before the run is killed.
            factory_startup: Start from factory settings (no user add-ons/prefs).

        Returns:
            A :class:`RunResult`.

        Raises:
            BlenderRunnerError: If the script path does not exist or times out.
        """
        script_path = Path(script)
        if not script_path.exists():
            raise BlenderRunnerError(f"script not found: {script_path}")
        cmd = [self.bin, "-b"]
        if factory_startup:
            cmd.append("--factory-startup")
        cmd += ["-P", str(script_path), "--", *(str(a) for a in args)]
        try:
            proc = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=timeout,
                check=False,
            )
        except subprocess.TimeoutExpired as exc:
            raise BlenderRunnerError(
                f"Blender run timed out after {timeout}s: {script_path.name}"
            ) from exc
        return RunResult(proc.returncode, proc.stdout, proc.stderr)


def _bundled_script(name: str) -> Path:
    """Return a bundled bpy script path or raise a helpful error."""
    script = _SCRIPTS_DIR / name
    if not script.exists():
        raise BlenderRunnerError(
            f"bundled script {name!r} not found at {script} (needs the repo "
            "layout; pass an explicit script path otherwise)"
        )
    return script


def run_sprite_to_3d(
    image: str | Path,
    out: str | Path,
    *,
    frames: int = 48,
    thickness: float = 0.06,
    cross: int = 2,
    render: str | Path | None = None,
    blender_bin: str | None = None,
    timeout: float = 600.0,
) -> Path:
    """Convert a sprite to an animated textured GLB via headless Blender.

    Runs the bundled ``sprite_to_3d.py`` through Blender.

    Args:
        image: Source sprite path.
        out: Destination ``.glb`` path.
        frames: Spin frames.
        thickness: Solidify depth (volume).
        cross: Fanned planes (1=flat card, 2=cross billboard always visible).
        render: Optional directory to also render the turntable PNGs into.
        blender_bin: Explicit Blender binary override.
        timeout: Seconds before the run is killed.

    Returns:
        The output ``.glb`` ``Path``.

    Raises:
        BlenderRunnerError: If Blender is missing or the run fails.
    """
    script = _bundled_script("sprite_to_3d.py")
    args = [
        "--image",
        os.path.abspath(image),
        "--out",
        os.path.abspath(out),
        "--frames",
        frames,
        "--thickness",
        thickness,
        "--cross",
        cross,
    ]
    if render is not None:
        args += ["--render", os.path.abspath(render)]
    result = BlenderRunner(blender_bin).run_script(
        script, args, timeout=timeout
    )
    if not result.ok:
        tail = (result.stderr or result.stdout)[-600:]
        raise BlenderRunnerError(
            f"sprite_to_3d failed (exit {result.returncode}): {tail}"
        )
    out_path = Path(out)
    if not out_path.exists():
        raise BlenderRunnerError(
            f"Blender exited 0 but {out_path} was not created; stdout tail: "
            f"{result.stdout[-400:]}"
        )
    return out_path


def optimize_glb(
    input_glb: str | Path,
    out: str | Path,
    *,
    decimate_ratio: float = 1.0,
    merge_distance: float = 0.0001,
    blender_bin: str | None = None,
    timeout: float = 600.0,
) -> Path:
    """Optimise a GLB (weld + recalc normals + decimate) via headless Blender.

    Args:
        input_glb: Source ``.glb`` path.
        out: Destination ``.glb`` path.
        decimate_ratio: ``<1.0`` reduces polygons.
        merge_distance: Merge-by-distance threshold (0 disables).
        blender_bin: Explicit Blender binary override.
        timeout: Seconds before the run is killed.

    Returns:
        The output ``.glb`` ``Path``.

    Raises:
        BlenderRunnerError: If Blender is missing or the run fails.
    """
    script = _bundled_script("optimize_glb.py")
    args = [
        "--in",
        os.path.abspath(input_glb),
        "--out",
        os.path.abspath(out),
        "--decimate",
        decimate_ratio,
        "--merge",
        merge_distance,
    ]
    result = BlenderRunner(blender_bin).run_script(
        script, args, timeout=timeout
    )
    if not result.ok:
        tail = (result.stderr or result.stdout)[-600:]
        raise BlenderRunnerError(
            f"optimize_glb failed (exit {result.returncode}): {tail}"
        )
    out_path = Path(out)
    if not out_path.exists():
        raise BlenderRunnerError(
            f"Blender exited 0 but {out_path} was not created; stdout tail: "
            f"{result.stdout[-400:]}"
        )
    return out_path


# ---------------------------------------------------------------------------
# GPU (NVIDIA OPTIX/CUDA) Cycles rendering
# ---------------------------------------------------------------------------

#: Marker the GPU scripts print their JSON result on.
_GPU_MARKER = "APHRODY_GPU_JSON"


def _parse_gpu_json(stdout: str) -> dict[str, Any]:
    """Extract the ``APHRODY_GPU_JSON`` payload from Blender's stdout."""
    for line in stdout.splitlines():
        if line.strip().startswith(_GPU_MARKER):
            return json.loads(line.split(_GPU_MARKER, 1)[1].strip())
    raise BlenderRunnerError(f"no {_GPU_MARKER} marker in Blender output")


def list_gpu_devices(
    *, blender_bin: str | None = None, timeout: float = 120.0
) -> dict[str, Any]:
    """Enumerate the Cycles compute devices (GPU/CPU) Blender can use.

    Returns:
        ``{compute_device_type, scene_device, devices:[{name,type,use}]}`` —
        ``compute_device_type`` is the selected back-end (e.g. ``OPTIX``).

    Raises:
        BlenderRunnerError: If Blender is missing or the probe fails.
    """
    script = _bundled_script("gpu_render.py")
    result = BlenderRunner(blender_bin).run_script(
        script, ["--probe"], timeout=timeout
    )
    if not result.ok:
        raise BlenderRunnerError(
            f"GPU probe failed (exit {result.returncode}): "
            f"{(result.stderr or result.stdout)[-400:]}"
        )
    return _parse_gpu_json(result.stdout)


def render_turntable_gpu(
    glb: str | Path,
    out_dir: str | Path,
    *,
    frames: int = 24,
    samples: int = 64,
    resolution: int = 512,
    device: str = "AUTO",
    blender_bin: str | None = None,
    timeout: float = 900.0,
) -> dict[str, Any]:
    """Render a turntable of *glb* on the GPU (Cycles OPTIX/CUDA).

    Args:
        glb: Source ``.glb`` to import and spin.
        out_dir: Destination directory for the PNG frames.
        frames: Number of frames in the revolution.
        samples: Cycles samples per frame.
        resolution: Square render resolution.
        device: ``AUTO`` / ``OPTIX`` / ``CUDA`` / ``CPU``.
        blender_bin: Explicit Blender binary override.
        timeout: Seconds before the run is killed.

    Returns:
        The device summary plus ``frames``/``out_dir`` actually rendered.

    Raises:
        BlenderRunnerError: If Blender is missing or the render fails.
    """
    script = _bundled_script("gpu_render.py")
    args = [
        "--glb",
        os.path.abspath(glb),
        "--out-dir",
        os.path.abspath(out_dir),
        "--frames",
        frames,
        "--samples",
        samples,
        "--resolution",
        resolution,
        "--device",
        device,
    ]
    result = BlenderRunner(blender_bin).run_script(
        script, args, timeout=timeout
    )
    if not result.ok:
        tail = (result.stderr or result.stdout)[-600:]
        raise BlenderRunnerError(
            f"GPU render failed (exit {result.returncode}): {tail}"
        )
    return _parse_gpu_json(result.stdout)


def showcase_sprite(
    image: str | Path,
    out: str | Path,
    *,
    frames: int = 24,
    thickness: float = 0.06,
    cross: int = 2,
    samples: int = 48,
    resolution: int = 384,
    fps: float = 12.0,
    glb_out: str | Path | None = None,
    blender_bin: str | None = None,
    timeout: float = 900.0,
) -> dict[str, Any]:
    """End-to-end GPU showcase for a sprite.

    Chains: sprite → animated textured GLB (``sprite_to_3d``) → GPU turntable
    render (Cycles OPTIX/CUDA) → animated WebP (``aphrody.anim``).

    Args:
        image: Source sprite path.
        out: Destination animated ``.webp`` path.
        frames: Spin frames (also the rendered frame count).
        thickness: Solidify depth of the standee.
        cross: Fanned planes (2=cross billboard, visible from all angles).
        samples: Cycles samples per frame.
        resolution: Square render resolution.
        fps: Output animation frame rate.
        glb_out: Optional GLB path (defaults to *out* with a ``.glb`` suffix).
        blender_bin: Explicit Blender binary override.
        timeout: Per-Blender-run timeout.

    Returns:
        A summary ``{glb, rendered_frames, animation, device, scene_device}``.

    Raises:
        BlenderRunnerError: If any stage fails.
    """
    out_path = Path(out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    glb = Path(glb_out) if glb_out else out_path.with_suffix(".glb")

    run_sprite_to_3d(
        image,
        glb,
        frames=frames,
        thickness=thickness,
        cross=cross,
        blender_bin=blender_bin,
        timeout=timeout,
    )
    frames_dir = out_path.parent / f"{out_path.stem}_frames"
    info = render_turntable_gpu(
        glb,
        frames_dir,
        frames=frames,
        samples=samples,
        resolution=resolution,
        blender_bin=blender_bin,
        timeout=timeout,
    )

    from aphrody import anim

    frame_paths = sorted(str(p) for p in frames_dir.glob("frame_*.png"))
    if not frame_paths:
        raise BlenderRunnerError("GPU render produced no frames to animate")
    anim.build_animation(frame_paths, out_path, fmt="webp", fps=fps, loop=0)

    return {
        "glb": str(glb),
        "rendered_frames": len(frame_paths),
        "animation": str(out_path),
        "device": info.get("compute_device_type"),
        "scene_device": info.get("scene_device"),
    }


def render_multiview_turntable(
    pattern: str,
    out: str | Path,
    *,
    frames: int = 24,
    samples: int = 48,
    resolution: int = 512,
    device: str = "AUTO",
    ground: bool = True,
    fps: float = 12.0,
    blender_bin: str | None = None,
    timeout: float = 900.0,
) -> dict[str, Any]:
    """Render a solid multi-view impostor turntable to an animated WebP, GPU.

    Uses the real rotation views matching *pattern* (ordered by ``_r<n>``) on a
    camera-facing billboard with a ground + shadow, GPU-rendered (OPTIX/CUDA),
    then assembled into an animated WebP.

    Args:
        pattern: Glob for the rotation views (e.g. ``assets/aphrody_body_r*.webp``).
        out: Destination animated ``.webp`` path.
        frames: Number of output frames (views are cycled across them).
        samples: Cycles samples per frame.
        resolution: Square render resolution.
        device: ``AUTO`` / ``OPTIX`` / ``CUDA`` / ``CPU``.
        ground: Render a ground plane + shadow (else transparent background).
        fps: Output animation frame rate.
        blender_bin: Explicit Blender binary override.
        timeout: Seconds before the run is killed.

    Returns:
        The device summary plus ``views``/``frames``/``animation``.

    Raises:
        BlenderRunnerError: If Blender is missing or the render fails.
    """
    out_path = Path(out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    frames_dir = out_path.parent / f"{out_path.stem}_frames"
    script = _bundled_script("multiview_turntable.py")
    args = [
        "--pattern",
        pattern,
        "--out-dir",
        os.path.abspath(frames_dir),
        "--frames",
        frames,
        "--samples",
        samples,
        "--resolution",
        resolution,
        "--device",
        device,
    ]
    if not ground:
        args.append("--no-ground")
    result = BlenderRunner(blender_bin).run_script(
        script, args, timeout=timeout
    )
    if not result.ok:
        tail = (result.stderr or result.stdout)[-600:]
        raise BlenderRunnerError(
            f"multiview render failed (exit {result.returncode}): {tail}"
        )
    info = _parse_gpu_json(result.stdout)

    from aphrody import anim

    frame_paths = sorted(str(p) for p in frames_dir.glob("frame_*.png"))
    if not frame_paths:
        raise BlenderRunnerError("multiview render produced no frames")
    anim.build_animation(
        frame_paths,
        out_path,
        fmt="webp",
        fps=fps,
        loop=0,
        transparent=not ground,
    )
    info["animation"] = str(out_path)
    return info
