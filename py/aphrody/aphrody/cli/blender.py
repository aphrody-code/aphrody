# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Blender automation command group for the aphrody CLI."""

from __future__ import annotations

from pathlib import Path

from aphrody.cli.utils import _emit


class BlenderCommands:
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
