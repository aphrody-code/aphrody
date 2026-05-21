# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
#
# Provenance: the JSON socket-server scaffolding (thread accept loop + running
# commands on Blender's main thread via bpy.app.timers) follows the pattern of
# ahujasid/blender-mcp (MIT, © 2025 Siddharth Ahuja). The "pro" command set
# (optimised mesh cleanup, studio setup, auto-material, GLB/turntable presets,
# scene stats) is original aphrody work.
"""Aphrody — a pro control surface add-on for Blender.

Single-file Blender add-on that gives aphrody (and you) more control over
Blender, exposing optimised, production-ready operations both through a JSON
socket server (drop-in compatible with ``aphrody.blender.BlenderClient`` on
``localhost:9876``) and a 3D-viewport N-panel.

Install (Blender 4.2 - 5.1):
    Edit > Preferences > Add-ons > Install from Disk… > pick this file, enable
    "Aphrody". Then open the **Aphrody** tab in the 3D viewport N-panel and
    click *Start Server* (or it can auto-start). Headless:
    ``blender -b -P aphrody_addon.py`` then drive it via ``aphrody blender``.

Socket protocol: send one JSON ``{"type": str, "params": {...}}``; the add-on
runs it on the main thread and replies ``{"status": "success", "result": ...}``
or ``{"status": "error", "message": ...}``. ``execute_code`` returns captured
stdout.
"""

bl_info = {
    "name": "Aphrody",
    "author": "aphrody contributors",
    "version": (1, 0, 0),
    "blender": (4, 2, 0),
    "location": "View3D > Sidebar (N) > Aphrody",
    "description": "Pro control surface: JSON socket server + optimised mesh/render/export ops",
    "warning": "",
    "doc_url": "https://github.com/aphrody-code/aphrody",
    "support": "COMMUNITY",
    "category": "System",
}

import io
import json
import math
import socket
import threading
import traceback
from contextlib import redirect_stdout

import bpy

DEFAULT_PORT = 9876

# ---------------------------------------------------------------------------
# Pro operation helpers — pure bpy, reused by both the socket server and the
# UI operators. Each takes a params dict and returns a JSON-able result.
# ---------------------------------------------------------------------------


def _resolve_objects(params):
    """Return the target mesh objects: a named one, the selection, or all meshes."""
    name = params.get("object")
    if name:
        obj = bpy.data.objects.get(name)
        return [obj] if obj else []
    sel = [o for o in bpy.context.selected_objects if o.type == "MESH"]
    if sel:
        return sel
    return [o for o in bpy.data.objects if o.type == "MESH"]


def op_scene_info(params=None):
    """Summarise the scene (objects, materials, frame range)."""
    scene = bpy.context.scene
    return {
        "name": scene.name,
        "blender_version": bpy.app.version_string,
        "object_count": len(scene.objects),
        "objects": [
            {"name": o.name, "type": o.type} for o in scene.objects[:200]
        ],
        "materials": len(bpy.data.materials),
        "frame_start": scene.frame_start,
        "frame_end": scene.frame_end,
    }


def op_object_info(params):
    """Detailed info for one object."""
    obj = bpy.data.objects.get(params["name"])
    if obj is None:
        raise ValueError(f"object not found: {params['name']!r}")
    info = {
        "name": obj.name,
        "type": obj.type,
        "location": list(obj.location),
        "dimensions": list(obj.dimensions),
        "materials": [m.name for m in obj.data.materials]
        if obj.data and hasattr(obj.data, "materials")
        else [],
    }
    if obj.type == "MESH":
        info["vertices"] = len(obj.data.vertices)
        info["polygons"] = len(obj.data.polygons)
    return info


def op_scene_stats(params=None):
    """Aggregate poly/vert/object/material counts across the scene."""
    verts = tris = polys = 0
    meshes = 0
    for o in bpy.data.objects:
        if o.type == "MESH" and o.data:
            meshes += 1
            verts += len(o.data.vertices)
            polys += len(o.data.polygons)
            o.data.calc_loop_triangles()
            tris += len(o.data.loop_triangles)
    return {
        "objects": len(bpy.data.objects),
        "meshes": meshes,
        "vertices": verts,
        "polygons": polys,
        "triangles": tris,
        "materials": len(bpy.data.materials),
        "images": len(bpy.data.images),
    }


def op_execute_code(params):
    """Run arbitrary Python; return captured stdout."""
    buf = io.StringIO()
    with redirect_stdout(buf):
        exec(params["code"], {"bpy": bpy, "math": math})
    return {"executed": True, "result": buf.getvalue()}


def op_clear(params=None):
    """Delete every object in the scene."""
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    return {"cleared": True}


def op_import(params):
    """Import a mesh file (glb/gltf/fbx/obj/stl) and return new object names."""
    path = params["path"]
    before = {o.name for o in bpy.data.objects}
    ext = path.lower().rsplit(".", 1)[-1]
    if ext in ("glb", "gltf"):
        bpy.ops.import_scene.gltf(filepath=path)
    elif ext == "fbx":
        bpy.ops.import_scene.fbx(filepath=path)
    elif ext == "obj":
        bpy.ops.wm.obj_import(filepath=path)
    elif ext == "stl":
        bpy.ops.wm.stl_import(filepath=path)
    else:
        raise ValueError(f"unsupported import extension: .{ext}")
    return {
        "imported": [o.name for o in bpy.data.objects if o.name not in before]
    }


def op_export_glb(params):
    """Export to GLB with optional Draco compression and modifier apply."""
    bpy.ops.export_scene.gltf(
        filepath=params["path"],
        export_format="GLB",
        use_selection=bool(params.get("selected_only", False)),
        export_apply=bool(params.get("apply_modifiers", True)),
        export_draco_mesh_compression_enable=bool(params.get("draco", False)),
    )
    return {"exported": params["path"]}


def op_optimize_mesh(params):
    """Optimised pro mesh cleanup: weld, recalc normals, decimate, smooth.

    params: object, merge_distance (float), decimate_ratio (0-1, <1 reduces),
            recalc_normals (bool), shade_smooth (bool).
    """
    import bmesh

    objs = _resolve_objects(params)
    if not objs:
        raise ValueError("no mesh objects to optimise")
    merge = float(params.get("merge_distance", 0.0001))
    ratio = float(params.get("decimate_ratio", 1.0))
    recalc = bool(params.get("recalc_normals", True))
    smooth = bool(params.get("shade_smooth", True))

    report = []
    for obj in objs:
        me = obj.data
        before_v, before_p = len(me.vertices), len(me.polygons)
        bm = bmesh.new()
        bm.from_mesh(me)
        if merge > 0:
            bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=merge)
        if recalc:
            bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        bm.to_mesh(me)
        bm.free()
        me.update()
        if ratio < 1.0:
            mod = obj.modifiers.new("aphrody_decimate", "DECIMATE")
            mod.ratio = ratio
            with bpy.context.temp_override(object=obj):
                bpy.ops.object.modifier_apply(modifier=mod.name)
        if smooth:
            for poly in me.polygons:
                poly.use_smooth = True
        report.append(
            {
                "object": obj.name,
                "vertices_before": before_v,
                "vertices_after": len(me.vertices),
                "polygons_before": before_p,
                "polygons_after": len(me.polygons),
            }
        )
    return {"optimized": report}


def op_auto_material(params):
    """Assign a Principled BSDF material with given base colour / PBR values."""
    objs = _resolve_objects(params)
    if not objs:
        raise ValueError("no mesh objects for material")
    color = params.get("base_color", [0.8, 0.1, 0.1, 1.0])
    metallic = float(params.get("metallic", 0.0))
    roughness = float(params.get("roughness", 0.4))
    mat = bpy.data.materials.new(params.get("name", "aphrody_mat"))
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf:
        bsdf.inputs["Base Color"].default_value = color
        bsdf.inputs["Metallic"].default_value = metallic
        bsdf.inputs["Roughness"].default_value = roughness
    for obj in objs:
        obj.data.materials.clear()
        obj.data.materials.append(mat)
    return {"material": mat.name, "objects": [o.name for o in objs]}


def op_setup_studio(params=None):
    """Pro studio setup: framing camera + sun key light, aimed at the meshes."""
    import mathutils

    params = params or {}
    scene = bpy.context.scene
    cam = next((o for o in bpy.data.objects if o.type == "CAMERA"), None)
    if cam is None:
        cam = bpy.data.objects.new(
            "aphrody_cam", bpy.data.cameras.new("aphrody_cam")
        )
        scene.collection.objects.link(cam)
    scene.camera = cam
    if not any(o.type == "LIGHT" for o in bpy.data.objects):
        ld = bpy.data.lights.new("aphrody_sun", type="SUN")
        ld.energy = float(params.get("light_energy", 3.0))
        light = bpy.data.objects.new("aphrody_sun", ld)
        light.rotation_euler = (math.radians(50), 0, math.radians(40))
        scene.collection.objects.link(light)
    meshes = [o for o in bpy.data.objects if o.type == "MESH"]
    if meshes:
        cs = [o.matrix_world.translation for o in meshes]
        center = sum(cs, mathutils.Vector()) / len(cs)
        size = max((max(o.dimensions) for o in meshes), default=2.0)
        cam.location = center + mathutils.Vector((size, -size, size * 0.8))
        cam.rotation_euler = (
            (center - cam.location).to_track_quat("-Z", "Y").to_euler()
        )
    bpy.context.view_layer.update()
    return {"camera": cam.name}


def _apply_render_settings(scene, params):
    """Apply common render settings from params (engine, samples, resolution)."""
    engine = params.get("engine")
    if engine:
        scene.render.engine = engine
    if params.get("cpu") and hasattr(scene, "cycles"):
        scene.cycles.device = "CPU"
    samples = params.get("samples")
    if samples and hasattr(scene, "cycles"):
        scene.cycles.samples = int(samples)
    res = params.get("resolution", [800, 800])
    scene.render.resolution_x, scene.render.resolution_y = (
        int(res[0]),
        int(res[1]),
    )
    scene.render.film_transparent = bool(params.get("transparent", True))
    scene.render.image_settings.file_format = "PNG"


def op_render(params):
    """Render a still to PNG with sane pro defaults."""
    scene = bpy.context.scene
    _apply_render_settings(scene, params)
    scene.render.filepath = params["out"]
    bpy.context.view_layer.update()
    bpy.ops.render.render(write_still=True)
    return {"rendered": params["out"]}


def op_turntable(params):
    """Render an orbiting turntable PNG sequence of the target mesh."""
    import os

    scene = bpy.context.scene
    _apply_render_settings(
        scene, {**params, "resolution": params.get("resolution", [600, 600])}
    )
    frames = int(params.get("frames", 16))
    out_dir = params["out_dir"]
    os.makedirs(out_dir, exist_ok=True)
    name = params.get("target")
    obj = (
        bpy.data.objects.get(name)
        if name
        else next((o for o in bpy.data.objects if o.type == "MESH"), None)
    )
    written = []
    for i in range(frames):
        if obj is not None:
            obj.rotation_euler[2] = (i / frames) * 2 * math.pi
        bpy.context.view_layer.update()
        p = os.path.join(out_dir, f"frame_{i:03d}.png")
        scene.render.filepath = p
        bpy.ops.render.render(write_still=True)
        written.append(p)
    return {"frames": written}


#: Socket command dispatch table.
HANDLERS = {
    "get_scene_info": op_scene_info,
    "get_object_info": op_object_info,
    "execute_code": op_execute_code,
    "aphrody_scene_stats": op_scene_stats,
    "aphrody_clear": op_clear,
    "aphrody_import": op_import,
    "aphrody_export_glb": op_export_glb,
    "aphrody_optimize_mesh": op_optimize_mesh,
    "aphrody_auto_material": op_auto_material,
    "aphrody_setup_studio": op_setup_studio,
    "aphrody_render": op_render,
    "aphrody_turntable": op_turntable,
}


# ---------------------------------------------------------------------------
# Socket server (commands executed on Blender's main thread via timers)
# ---------------------------------------------------------------------------


class AphrodyServer:
    """A JSON command server living inside Blender."""

    def __init__(self, host="localhost", port=DEFAULT_PORT):
        self.host = host
        self.port = port
        self.running = False
        self._sock = None
        self._thread = None

    def start(self):
        """Bind, listen, and spawn the accept thread."""
        if self.running:
            return
        self._sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self._sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self._sock.bind((self.host, self.port))
        self._sock.listen(1)
        self.running = True
        self._thread = threading.Thread(target=self._loop, daemon=True)
        self._thread.start()
        print(f"[aphrody] server started on {self.host}:{self.port}")

    def stop(self):
        """Stop accepting and close the socket."""
        self.running = False
        if self._sock is not None:
            try:
                self._sock.close()
            except OSError:
                pass
            self._sock = None
        print("[aphrody] server stopped")

    def _loop(self):
        self._sock.settimeout(1.0)
        while self.running:
            try:
                client, _ = self._sock.accept()
            except TimeoutError:
                continue
            except OSError:
                break
            self._handle_client(client)

    def _handle_client(self, client):
        client.settimeout(None)
        buffer = b""
        try:
            while self.running:
                data = client.recv(8192)
                if not data:
                    break
                buffer += data
                try:
                    command = json.loads(buffer.decode("utf-8"))
                    buffer = b""
                except json.JSONDecodeError:
                    continue

                def run(cmd=command, conn=client):
                    try:
                        resp = self._dispatch(cmd)
                    except Exception as exc:
                        traceback.print_exc()
                        resp = {"status": "error", "message": str(exc)}
                    try:
                        conn.sendall(json.dumps(resp).encode("utf-8"))
                    except OSError:
                        pass
                    # implicit return None unregisters this one-shot timer

                bpy.app.timers.register(run, first_interval=0.0)
        except OSError:
            pass
        finally:
            try:
                client.close()
            except OSError:
                pass

    @staticmethod
    def _dispatch(command):
        handler = HANDLERS.get(command.get("type"))
        if handler is None:
            return {
                "status": "error",
                "message": f"unknown command: {command.get('type')!r}",
            }
        result = handler(command.get("params", {}))
        return {"status": "success", "result": result}


_server = None


def _get_server(port=DEFAULT_PORT):
    global _server
    if _server is None:
        _server = AphrodyServer(port=port)
    return _server


# ---------------------------------------------------------------------------
# Operators (UI + headless)
# ---------------------------------------------------------------------------


class APHRODY_OT_start_server(bpy.types.Operator):
    """Start the Aphrody JSON socket server."""

    bl_idname = "aphrody.start_server"
    bl_label = "Start Aphrody Server"

    def execute(self, context):
        srv = _get_server(context.scene.aphrody_port)
        srv.port = context.scene.aphrody_port
        srv.start()
        context.scene.aphrody_running = True
        self.report({"INFO"}, f"Aphrody server on :{srv.port}")
        return {"FINISHED"}


class APHRODY_OT_stop_server(bpy.types.Operator):
    """Stop the Aphrody JSON socket server."""

    bl_idname = "aphrody.stop_server"
    bl_label = "Stop Aphrody Server"

    def execute(self, context):
        if _server is not None:
            _server.stop()
        context.scene.aphrody_running = False
        return {"FINISHED"}


class APHRODY_OT_optimize_mesh(bpy.types.Operator):
    """Weld, recalc normals, decimate and smooth the selected meshes."""

    bl_idname = "aphrody.optimize_mesh"
    bl_label = "Optimize Mesh"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        res = op_optimize_mesh(
            {
                "decimate_ratio": context.scene.aphrody_decimate,
                "merge_distance": context.scene.aphrody_merge,
            }
        )
        self.report({"INFO"}, f"Optimised {len(res['optimized'])} mesh(es)")
        return {"FINISHED"}


class APHRODY_OT_setup_studio(bpy.types.Operator):
    """Add a framing camera and key light."""

    bl_idname = "aphrody.setup_studio"
    bl_label = "Setup Studio"

    def execute(self, context):
        op_setup_studio({})
        return {"FINISHED"}


class APHRODY_OT_turntable(bpy.types.Operator):
    """Render a turntable sequence to //aphrody_turntable."""

    bl_idname = "aphrody.turntable_render"
    bl_label = "Turntable Render"

    def execute(self, context):
        op_setup_studio({})
        res = op_turntable(
            {
                "out_dir": bpy.path.abspath("//aphrody_turntable"),
                "frames": context.scene.aphrody_frames,
            }
        )
        self.report({"INFO"}, f"Rendered {len(res['frames'])} frames")
        return {"FINISHED"}


# ---------------------------------------------------------------------------
# UI panel
# ---------------------------------------------------------------------------


class APHRODY_PT_panel(bpy.types.Panel):
    """Aphrody control panel in the 3D viewport sidebar."""

    bl_label = "Aphrody"
    bl_idname = "APHRODY_PT_panel"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Aphrody"

    def draw(self, context):
        layout = self.layout
        scene = context.scene

        box = layout.box()
        box.label(text="Server", icon="PLUGIN")
        box.prop(scene, "aphrody_port")
        if scene.aphrody_running:
            box.operator("aphrody.stop_server", icon="PAUSE")
            box.label(
                text=f"Running on :{scene.aphrody_port}", icon="CHECKMARK"
            )
        else:
            box.operator("aphrody.start_server", icon="PLAY")

        box = layout.box()
        box.label(text="Pro ops", icon="MODIFIER")
        box.prop(scene, "aphrody_decimate")
        box.prop(scene, "aphrody_merge")
        box.operator("aphrody.optimize_mesh", icon="MOD_DECIM")
        box.operator("aphrody.setup_studio", icon="CAMERA_DATA")
        box.prop(scene, "aphrody_frames")
        box.operator("aphrody.turntable_render", icon="RENDER_ANIMATION")


_CLASSES = (
    APHRODY_OT_start_server,
    APHRODY_OT_stop_server,
    APHRODY_OT_optimize_mesh,
    APHRODY_OT_setup_studio,
    APHRODY_OT_turntable,
    APHRODY_PT_panel,
)


def register():
    """Register classes, scene properties; the server is started from the panel."""
    bpy.types.Scene.aphrody_port = bpy.props.IntProperty(
        name="Port", default=DEFAULT_PORT, min=1024, max=65535
    )
    bpy.types.Scene.aphrody_running = bpy.props.BoolProperty(default=False)
    bpy.types.Scene.aphrody_decimate = bpy.props.FloatProperty(
        name="Decimate",
        default=1.0,
        min=0.01,
        max=1.0,
        description="Decimate ratio (<1 reduces polygons)",
    )
    bpy.types.Scene.aphrody_merge = bpy.props.FloatProperty(
        name="Merge Dist",
        default=0.0001,
        min=0.0,
        max=1.0,
        precision=5,
        description="Merge-by-distance threshold",
    )
    bpy.types.Scene.aphrody_frames = bpy.props.IntProperty(
        name="Frames", default=16, min=2, max=180
    )
    for cls in _CLASSES:
        bpy.utils.register_class(cls)


def unregister():
    """Stop the server, unregister classes and scene properties."""
    if _server is not None:
        _server.stop()
    for cls in reversed(_CLASSES):
        bpy.utils.unregister_class(cls)
    del bpy.types.Scene.aphrody_port
    del bpy.types.Scene.aphrody_running
    del bpy.types.Scene.aphrody_decimate
    del bpy.types.Scene.aphrody_merge
    del bpy.types.Scene.aphrody_frames


if __name__ == "__main__":
    register()
