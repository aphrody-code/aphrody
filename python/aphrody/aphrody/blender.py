# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Drive a running Blender from aphrody over the blender-mcp addon socket.

The `blender-mcp <https://github.com/ahujasid/blender-mcp>`_ addon (MIT) runs a
JSON socket server inside Blender (default ``localhost:9876``). This module is a
**dependency-free** client (stdlib ``socket`` + ``json``) mirroring the addon's
wire protocol — no MCP layer, no extra packages — so aphrody can import its
generated ``.glb`` meshes, render turntables, and run arbitrary ``bpy`` code in
a live Blender.

Wire protocol: the client sends one JSON object ``{"type": str, "params":
dict}``; the addon accumulates bytes until it parses, executes on Blender's main
thread, and replies with a single JSON ``{"status": "success", "result": ...}``
or ``{"status": "error", "message": ...}``. ``execute_code`` runs Python in
Blender and returns its captured **stdout**, so structured values come back by
``print(json.dumps(...))``.

Requires a running Blender with the blender-mcp addon installed and its server
started (the addon's *Connect* button / ``BlenderMCP`` panel).

    >>> from aphrody.blender import BlenderClient
    >>> with BlenderClient() as bl:                       # doctest: +SKIP
    ...     bl.import_glb("var/imgtest/aphrody.glb")
    ...     bl.render_still("var/imgtest/blender_render.png")
"""

from __future__ import annotations

import json
import logging
import socket
import textwrap
from pathlib import Path
from typing import Any

from aphrody.errors import AphrodyError

logger = logging.getLogger(__name__)

#: Default blender-mcp addon socket host/port.
DEFAULT_HOST = "localhost"
DEFAULT_PORT = 9876
#: Match the addon's long operation timeout (renders can be slow).
DEFAULT_TIMEOUT = 180.0


class BlenderError(AphrodyError):
    """Raised on connection or command failure talking to Blender."""


class BlenderClient:
    """A stdlib socket client for the blender-mcp addon server.

    Usable as a context manager; reconnects lazily on first command.
    """

    def __init__(
        self,
        host: str = DEFAULT_HOST,
        port: int = DEFAULT_PORT,
        *,
        timeout: float = DEFAULT_TIMEOUT,
    ) -> None:
        """Initialise the client.

        Args:
            host: Addon server host.
            port: Addon server port (default 9876).
            timeout: Socket timeout in seconds for connect + each command.
        """
        self.host = host
        self.port = port
        self.timeout = timeout
        self.sock: socket.socket | None = None

    def __enter__(self) -> BlenderClient:
        self.connect()
        return self

    def __exit__(self, *_exc: object) -> None:
        self.disconnect()

    # ------------------------------------------------------------------
    # Transport
    # ------------------------------------------------------------------

    def connect(self) -> None:
        """Open the socket to the addon, raising :class:`BlenderError` on failure."""
        if self.sock is not None:
            return
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(self.timeout)
        try:
            sock.connect((self.host, self.port))
        except OSError as exc:
            sock.close()
            raise BlenderError(
                f"could not connect to Blender at {self.host}:{self.port} — is "
                "the blender-mcp addon server running? (Blender > BlenderMCP "
                f"panel > Connect). {exc}"
            ) from exc
        self.sock = sock

    def disconnect(self) -> None:
        """Close the socket if open."""
        if self.sock is not None:
            try:
                self.sock.close()
            finally:
                self.sock = None

    def _receive_full_response(self, buffer_size: int = 8192) -> bytes:
        """Read until the accumulated buffer parses as one JSON object."""
        assert self.sock is not None
        self.sock.settimeout(self.timeout)
        chunks: list[bytes] = []
        while True:
            try:
                chunk = self.sock.recv(buffer_size)
            except TimeoutError as exc:  # socket.timeout aliases TimeoutError
                raise BlenderError(
                    "timeout waiting for Blender response — try a simpler command"
                ) from exc
            if not chunk:
                if not chunks:
                    raise BlenderError("connection closed before any data")
                break
            chunks.append(chunk)
            try:
                data = b"".join(chunks)
                json.loads(data.decode("utf-8"))
                return data
            except json.JSONDecodeError:
                continue  # incomplete; keep reading
        data = b"".join(chunks)
        json.loads(data.decode("utf-8"))  # final parse (may raise)
        return data

    def send_command(
        self, cmd_type: str, params: dict[str, Any] | None = None
    ) -> Any:
        """Send a command and return the addon's ``result`` payload.

        Args:
            cmd_type: The addon command (e.g. ``"get_scene_info"``,
                ``"execute_code"``).
            params: Command parameters.

        Returns:
            The ``result`` field of a successful response.

        Raises:
            BlenderError: On transport failure or an ``error`` status.
        """
        self.connect()
        assert self.sock is not None
        command = {"type": cmd_type, "params": params or {}}
        try:
            self.sock.sendall(json.dumps(command).encode("utf-8"))
            data = self._receive_full_response()
            response = json.loads(data.decode("utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            self.disconnect()
            raise BlenderError(
                f"communication error with Blender: {exc}"
            ) from exc
        if response.get("status") == "error":
            raise BlenderError(response.get("message", "unknown Blender error"))
        return response.get("result")

    # ------------------------------------------------------------------
    # Code execution
    # ------------------------------------------------------------------

    def execute_code(self, code: str) -> str:
        """Run Python *code* inside Blender; return its captured stdout.

        Args:
            code: Python source executed with ``bpy`` in scope.

        Returns:
            The stdout the code printed.
        """
        result = self.send_command(
            "execute_code", {"code": textwrap.dedent(code)}
        )
        if isinstance(result, dict):
            return str(result.get("result", ""))
        return str(result)

    def eval_json(self, code: str) -> Any:
        """Run *code* that prints one JSON value on its last line; parse it.

        Args:
            code: Python source whose final printed line is a JSON document.

        Returns:
            The parsed JSON value.

        Raises:
            BlenderError: If no JSON line was printed.
        """
        out = self.execute_code(code)
        lines = [ln for ln in out.splitlines() if ln.strip()]
        if not lines:
            raise BlenderError(f"code printed no JSON output (stdout={out!r})")
        return json.loads(lines[-1])

    # ------------------------------------------------------------------
    # Introspection
    # ------------------------------------------------------------------

    def get_scene_info(self) -> dict[str, Any]:
        """Return the addon's scene summary (objects, materials, …)."""
        return self.send_command("get_scene_info")

    def get_object_info(self, name: str) -> dict[str, Any]:
        """Return detailed info for the object named *name*."""
        return self.send_command("get_object_info", {"name": name})

    def ping(self) -> dict[str, Any]:
        """Connectivity check: return the scene summary or raise."""
        return self.get_scene_info()

    # ------------------------------------------------------------------
    # Aphrody add-on pro commands (require the aphrody Blender add-on, not
    # plain blender-mcp — these are native handlers, not execute_code).
    # ------------------------------------------------------------------

    def scene_stats(self) -> dict[str, Any]:
        """Return aggregate poly/vert/object/material counts (aphrody add-on)."""
        return self.send_command("aphrody_scene_stats")

    def optimize_mesh(
        self,
        *,
        obj: str | None = None,
        decimate_ratio: float = 1.0,
        merge_distance: float = 0.0001,
        recalc_normals: bool = True,
        shade_smooth: bool = True,
    ) -> dict[str, Any]:
        """Weld, recalc normals, decimate and smooth meshes (aphrody add-on).

        Args:
            obj: Target object name; ``None`` = selection or all meshes.
            decimate_ratio: ``<1.0`` reduces polygon count.
            merge_distance: Merge-by-distance threshold (0 disables).
            recalc_normals: Recalculate face normals.
            shade_smooth: Set smooth shading.

        Returns:
            A per-object before/after report.
        """
        return self.send_command(
            "aphrody_optimize_mesh",
            {
                "object": obj,
                "decimate_ratio": decimate_ratio,
                "merge_distance": merge_distance,
                "recalc_normals": recalc_normals,
                "shade_smooth": shade_smooth,
            },
        )

    def auto_material(
        self,
        *,
        obj: str | None = None,
        base_color: tuple[float, float, float, float] = (0.8, 0.1, 0.1, 1.0),
        metallic: float = 0.0,
        roughness: float = 0.4,
    ) -> dict[str, Any]:
        """Assign a Principled BSDF material to meshes (aphrody add-on)."""
        return self.send_command(
            "aphrody_auto_material",
            {
                "object": obj,
                "base_color": list(base_color),
                "metallic": metallic,
                "roughness": roughness,
            },
        )

    # ------------------------------------------------------------------
    # High-level mesh / render helpers (run bpy via execute_code)
    # ------------------------------------------------------------------

    def clear_scene(self) -> None:
        """Delete every object in the current scene."""
        self.execute_code(
            "import bpy\n"
            "bpy.ops.object.select_all(action='SELECT')\n"
            "bpy.ops.object.delete()\n"
        )

    def import_glb(self, path: str | Path) -> list[str]:
        """Import a glTF/GLB file and return the new object names.

        Args:
            path: Path to a ``.glb``/``.gltf`` file.

        Returns:
            The names of the objects added by the import.
        """
        resolved = str(Path(path).resolve())
        return self.eval_json(
            f"""
            import bpy, json
            before = {{o.name for o in bpy.data.objects}}
            bpy.ops.import_scene.gltf(filepath={resolved!r})
            print(json.dumps([o.name for o in bpy.data.objects if o.name not in before]))
            """
        )

    def export_glb(
        self, path: str | Path, *, selected_only: bool = False
    ) -> Path:
        """Export the scene (or selection) to a ``.glb``.

        Args:
            path: Destination ``.glb`` path.
            selected_only: Export only selected objects.

        Returns:
            The destination ``Path``.
        """
        resolved = str(Path(path).resolve())
        self.execute_code(
            f"""
            import bpy
            bpy.ops.export_scene.gltf(filepath={resolved!r},
                export_format='GLB', use_selection={selected_only!r})
            print('exported')
            """
        )
        return Path(path)

    def render_still(
        self,
        out: str | Path,
        *,
        resolution: tuple[int, int] = (800, 800),
        engine: str | None = None,
        samples: int | None = None,
        film_transparent: bool = True,
    ) -> Path:
        """Render the current scene to a PNG.

        Assumes a camera exists (use :meth:`setup_camera_light` first if not).

        Args:
            out: Destination PNG path.
            resolution: ``(x, y)`` pixels.
            engine: Render engine id (e.g. ``"CYCLES"``); ``None`` keeps the
                scene's current engine to avoid version-specific names.
            samples: Cycles/EEVEE samples (engine-dependent); ``None`` keeps it.
            film_transparent: Transparent background.

        Returns:
            The output ``Path``.
        """
        resolved = str(Path(out).resolve())
        engine_line = f"scene.render.engine = {engine!r}\n" if engine else ""
        samples_line = (
            f"try:\n    scene.cycles.samples = {samples}\nexcept Exception:\n    pass\n"
            if samples
            else ""
        )
        self.execute_code(
            f"""
            import bpy
            scene = bpy.context.scene
            {engine_line}{samples_line}scene.render.resolution_x = {resolution[0]}
            scene.render.resolution_y = {resolution[1]}
            scene.render.film_transparent = {film_transparent!r}
            scene.render.image_settings.file_format = 'PNG'
            scene.render.filepath = {resolved!r}
            bpy.ops.render.render(write_still=True)
            print('rendered')
            """
        )
        return Path(out)

    def setup_camera_light(
        self, target: str | None = None, *, distance: float = 6.0
    ) -> None:
        """Ensure a camera + sun light exist and frame *target* (or all objects).

        Args:
            target: Object name to look at; ``None`` frames the whole scene.
            distance: Camera distance multiplier from the target.
        """
        target_expr = repr(target) if target else "None"
        self.execute_code(
            f"""
            import bpy, mathutils, math
            scene = bpy.context.scene
            # Camera
            cam = next((o for o in bpy.data.objects if o.type == 'CAMERA'), None)
            if cam is None:
                cam_data = bpy.data.cameras.new('aphrody_cam')
                cam = bpy.data.objects.new('aphrody_cam', cam_data)
                scene.collection.objects.link(cam)
            scene.camera = cam
            # Sun
            if not any(o.type == 'LIGHT' for o in bpy.data.objects):
                light_data = bpy.data.lights.new('aphrody_sun', type='SUN')
                light_data.energy = 3.0
                light = bpy.data.objects.new('aphrody_sun', light_data)
                light.rotation_euler = (math.radians(50), 0, math.radians(40))
                scene.collection.objects.link(light)
            # Frame target
            tgt_name = {target_expr}
            objs = [bpy.data.objects[tgt_name]] if tgt_name else [o for o in bpy.data.objects if o.type == 'MESH']
            if objs:
                cs = [o.matrix_world.translation for o in objs]
                center = sum(cs, mathutils.Vector()) / len(cs)
                size = max((max(o.dimensions) for o in objs), default=2.0)
                cam.location = center + mathutils.Vector((size, -size, size * 0.8)) * {distance / 6.0}
                direction = center - cam.location
                cam.rotation_euler = direction.to_track_quat('-Z', 'Y').to_euler()
            bpy.context.view_layer.update()  # transforms are lazy; flush before render
            print('camera_light_ready')
            """
        )

    def turntable(
        self,
        out_dir: str | Path,
        *,
        frames: int = 16,
        target: str | None = None,
        resolution: tuple[int, int] = (600, 600),
        engine: str | None = None,
    ) -> list[Path]:
        """Render an orbiting turntable image sequence of *target*.

        Sets up a camera + light, then spins the target on Z over *frames*
        renders. Requires a running Blender.

        Args:
            out_dir: Directory for ``frame_000.png`` … images.
            frames: Number of frames in one full revolution.
            target: Object to spin; ``None`` spins the first mesh.
            resolution: ``(x, y)`` pixels per frame.
            engine: Optional render engine id.

        Returns:
            The list of rendered frame ``Path`` objects.
        """
        out = Path(out_dir)
        out.mkdir(parents=True, exist_ok=True)
        self.setup_camera_light(target)
        resolved = str(out.resolve()).replace("\\", "/")
        engine_line = f"scene.render.engine = {engine!r}\n" if engine else ""
        names = self.eval_json(
            f"""
            import bpy, math, json, os
            scene = bpy.context.scene
            {engine_line}scene.render.resolution_x = {resolution[0]}
            scene.render.resolution_y = {resolution[1]}
            scene.render.film_transparent = True
            scene.render.image_settings.file_format = 'PNG'
            tgt = {repr(target) if target else "None"}
            obj = bpy.data.objects[tgt] if tgt else next((o for o in bpy.data.objects if o.type == 'MESH'), None)
            written = []
            for i in range({frames}):
                if obj is not None:
                    obj.rotation_euler[2] = (i / {frames}) * 2 * math.pi
                bpy.context.view_layer.update()  # flush lazy transform before render
                p = os.path.join({resolved!r}, 'frame_%03d.png' % i)
                scene.render.filepath = p
                bpy.ops.render.render(write_still=True)
                written.append(p)
            print(json.dumps(written))
            """
        )
        return [Path(p) for p in names]
