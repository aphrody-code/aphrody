# Aphrody — Blender add-on

A pro control surface for Blender: a JSON socket server (drop-in compatible with
`aphrody.blender.BlenderClient` on `localhost:9876`) plus optimised,
production-ready operations exposed both over the socket and a 3D-viewport
N-panel. This is aphrody's **own** add-on — a superset of the blender-mcp base
protocol, fully under our control, no Node/MCP layer.

## Install

**Legacy single-file (simplest, Blender 4.2–5.1):**
Edit > Preferences > Add-ons > *Install from Disk…* > pick `aphrody_addon.py` >
enable **Aphrody**.

**Extension (Blender 4.2+):**
Zip this folder (`__init__.py` + `blender_manifest.toml` + `aphrody_addon.py`)
and drag-drop it onto Blender, or *Install from Disk* the zip.

**Headless:** `blender -b -P aphrody_addon.py` (registers + can start the server
from a follow-up script).

Then open the **Aphrody** tab in the 3D viewport sidebar (press `N`) and click
*Start Server*.

## Socket protocol

Send one JSON `{"type": str, "params": {...}}`; the add-on runs it on Blender's
main thread and replies `{"status": "success", "result": ...}` /
`{"status": "error", "message": ...}`.

| Command | Purpose |
|---------|---------|
| `get_scene_info`, `get_object_info` | introspection (blender-mcp compatible) |
| `execute_code` | run arbitrary `bpy`, returns captured stdout |
| `aphrody_scene_stats` | poly/vert/object/material/triangle counts |
| `aphrody_import` | import glb/gltf/fbx/obj/stl → new object names |
| `aphrody_export_glb` | export GLB (Draco, apply modifiers, selection) |
| `aphrody_optimize_mesh` | weld + recalc normals + decimate + smooth |
| `aphrody_auto_material` | Principled BSDF (base colour, metallic, roughness) |
| `aphrody_setup_studio` | framing camera + key light |
| `aphrody_render` | still PNG (engine/samples/CPU/transparent) |
| `aphrody_turntable` | orbiting PNG sequence |

## Drive from aphrody

```bash
aphrody blender scene
aphrody blender import_glb var/imgtest/aphrody.glb
aphrody blender render --out render.png
aphrody blender turntable --frames 16 --out_dir tt
```

```python
from aphrody.blender import BlenderClient
with BlenderClient() as bl:
    print(bl.scene_stats())
    bl.optimize_mesh(decimate_ratio=0.5)
    bl.auto_material(base_color=(0.2, 0.5, 0.9, 1.0), roughness=0.3)
```

Pure stdlib socket/json — no third-party dependencies inside Blender.
