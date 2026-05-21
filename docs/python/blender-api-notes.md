# Blender Python API notes + the aphrody ↔ Blender bridge

Distilled from the **Blender 5.1** Python API docs (`docs.blender.org/api/current`)
for headless `.glb` automation, plus how `aphrody.blender` drives a live Blender.

## Module map (what to use headless)

| Module | Purpose | Headless? |
|--------|---------|-----------|
| `bpy.data` | create/remove/link data-blocks (`objects`, `meshes`, `materials`) | ✅ core |
| `bpy.context` | read-only active state (active object, selection, scene) | both |
| `bpy.ops` | invoke operators/tools; **context-dependent, fragile in scripts** | UI-ish |
| `bpy.types` | RNA class defs (subclass for add-ons; type refs) — not for instancing | add-on |
| `bpy.props` | typed add-on properties | add-on |
| `bpy.app` | `background`, `version`, `binary_path`, `handlers`, `timers` | ✅ |
| `bmesh` | editable mesh (verts/edges/faces, `ops` like remesh/normals) | ✅ geometry |
| `mathutils` | `Vector` / `Matrix` / `Euler` / `Quaternion` (+ geometry, kdtree) | ✅ math |
| `imbuf` | image buffer load/save/resize independent of `bpy.data.images` | ✅ images |
| `gpu`, `blf`, `bpy_extras`, `freestyle` | viewport/draw/add-on helpers | UI |

Rule of thumb: headless mesh/render work uses `bpy.data` + `bmesh` + `mathutils`
+ a *small* set of `bpy.ops` (import/export/render). Prefer the **data API over
`bpy.ops`** (ops can't take data args, only return status, and a failed `poll()`
raises `RuntimeError`).

## Headless / background

- CLI: `blender -b --background --python script.py -- <args>`; the script reads
  args after a bare `--` via `sys.argv[sys.argv.index("--")+1:]`.
- **pip `bpy`**: `pip install bpy` → wheel `bpy 5.1.2` **pins CPython 3.13**
  (each wheel matches exactly one Blender + one Python minor). `import bpy` runs
  implicitly in background, no UI.
- **No-GPU rendering**: use **Cycles CPU** (`scene.render.engine='CYCLES'`,
  `scene.cycles.device='CPU'`). EEVEE needs a real GL context (unreliable on
  truly headless boxes).

## Load-bearing gotchas

- **Transforms are lazy** → after setting `obj.location`/`rotation_euler` call
  **`bpy.context.view_layer.update()`** before reading `matrix_world` or
  rendering. (aphrody's turntable does this between frames.)
- **Don't persist data wrappers / reference by name** across ops — Blender may
  free internal data, and new data can be renamed (`.001`). Keep your own
  `name → datablock` dict.
- **Edit-Mode desync**: `obj.data` is stale in Edit-Mode — use
  `bmesh.from_edit_mesh()`.
- **Threading not supported** — parallelise across *processes* (multiple
  background Blender invocations), never Python threads.
- **Z-up vs glTF Y-up**: let `import_scene.gltf` / `export_scene.gltf`
  (`export_yup=True`) handle the swap; don't re-axis geometry by hand.

## Key operator signatures

```python
bpy.ops.import_scene.gltf(filepath="in.glb")        # imported objs = selected after
bpy.ops.export_scene.gltf(filepath="out.glb", export_format='GLB',
                          use_selection=False, export_apply=True)
bpy.ops.render.render(write_still=True)             # writes scene.render.filepath
```

## The aphrody ↔ Blender bridge (`aphrody.blender`)

`var/blender-mcp/` is the `ahujasid/blender-mcp` addon (MIT): it runs a JSON
socket server inside Blender on **`localhost:9876`**. `aphrody.blender` is a
**dependency-free** client (stdlib `socket` + `json`) mirroring that protocol —
no MCP layer needed.

aphrody also ships its **own** add-on at `python/aphrody/blender_addon/`
(`aphrody_addon.py` + `blender_manifest.toml`): a superset of the same protocol
plus pro commands (`aphrody_optimize_mesh`, `aphrody_auto_material`,
`aphrody_setup_studio`, `aphrody_scene_stats`, `aphrody_render`,
`aphrody_turntable`) and an N-panel — fully under our control, no Node/MCP. The
client exposes `scene_stats()`, `optimize_mesh()`, `auto_material()` for these.
And `blender_addon/scripts/sprite_to_3d.py` (headless `blender -b -P`) converts a
sprite into a textured standee with volume + a 360° spin and exports an animated
textured GLB. The base protocol:

- send `{"type": str, "params": dict}`; the addon accumulates bytes until the
  JSON parses, runs on Blender's main thread, replies
  `{"status": "success", "result": ...}` / `{"status": "error", "message": ...}`.
- `execute_code` runs `bpy` and returns its **captured stdout**, so structured
  values come back via `print(json.dumps(...))` (`BlenderClient.eval_json`).

```bash
# Blender must be open with the blender-mcp addon server started (BlenderMCP panel > Connect)
aphrody blender scene                              # scene summary
aphrody blender import_glb var/imgtest/aphrody.glb # import an aphrody-generated mesh
aphrody blender render --out render.png            # camera+light+render
aphrody blender turntable --frames 16 --out_dir tt # orbiting PNG sequence
aphrody blender exec "import bpy; print(len(bpy.data.objects))"
```

```python
from aphrody.blender import BlenderClient
with BlenderClient() as bl:
    names = bl.import_glb("var/imgtest/aphrody.glb")
    bl.setup_camera_light(names[0])
    bl.render_still("render.png", resolution=(1024, 1024))
```

**Tradeoff:** pip `bpy` is the simplest embed (one `pip install`) but is rigidly
locked to Blender 5.1 ↔ Python 3.13 and has no UI/add-ons; a full Blender install
ships the blender-mcp addon and runs the same `--background` script across Linux
(target #1) and Windows. The bridge here targets a *running, interactive* Blender
(the addon's socket), so EEVEE works and no Python-ABI matching is required.
