# Nano Banana Pro image & Material 3 icon suite

`aphrody` drives **Nano Banana Pro** (Gemini 3 Pro Image,
`gemini-3-pro-image-preview`) and the Material 3 icon workflow **keyless** — only
the on-device Antigravity OAuth credentials, never an API key.

Modules: `aphrody.images` (generate/edit/compose), `aphrody.prompts` (template
library + enhancer), `aphrody.optimize` (PNG/WebP/AVIF), `aphrody.batch`
(declarative bulk runs), `aphrody.icons` (M3 icon generation + SVG→ICO).

Install the optional encoders: `uv pip install 'aphrody[images]'` (PNG/WebP/AVIF)
and `'aphrody[icons]'` (resvg, scour, pyconify for the icon pipeline).

## Key gotcha — Nano Banana Pro lives at `location=global`

`gemini-3-pro-image-preview` is **only served from the `global` Vertex
location**; regional endpoints (`us-central1`) return HTTP 404. `aphrody.images`
defaults the location to `global` automatically for the Pro family, and falls
back down `FALLBACK_CHAIN` (`gemini-3-pro-image-preview` →
`gemini-3.1-flash-image-preview` → `gemini-2.5-flash-image`) if a model is not
entitled. The chosen model is reported as `model` in the CLI output.

## Generation — `aphrody image gen`

```bash
# Max quality: Nano Banana Pro at 4K, optimise PNG + WebP afterwards
aphrody image gen "a friendly robot mascot holding a banana, studio render" \
    --out robot.png --size 4K --aspect 16:9 --optimize png,webp

# Apply a style preset to the prompt, ground factual images on Google Search
aphrody image gen "the Eiffel tower at dawn" --enhance cinematic --grounding
```

- `--size` ∈ `1K|2K|4K`; `--aspect` ∈ `1:1 2:3 3:2 3:4 4:3 4:5 5:4 9:16 16:9 21:9`
  (see `aphrody image models`).
- `--enhance` ∈ `photoreal|cinematic|product|studio|illustration|render`.
- `--optimize` ∈ `True` (png+webp) or a list `png,webp,avif`.
- `--n` for multiple images, `--negative` for constraints, `--model` to override.

## Edit & compose

```bash
aphrody image edit photo.png "change the tie to green, remove the background car" --out edited.png
aphrody image compose "use image 1 as the pose and image 2 as the outfit" pose.png outfit.png --out merged.png
```

`compose` accepts up to **14** reference images.

## Prompt library — `aphrody image prompts` / `template` / `enhance`

20 production templates (portrait, product, typography, logo, cinematic,
infographic, sticker, 3d-render, architecture, food, fashion, character-sheet,
ui-mockup, data-viz, …).

```bash
aphrody image prompts                       # list templates + placeholders
aphrody image template logo --brand_name Aphrody --industry "developer tools" \
    --out logo.png --size 2K                # render + generate
aphrody image enhance "a cat on a sofa" --preset photoreal   # preview enhanced prompt
```

## Batch — `aphrody image batch spec.json`

```json
{
  "defaults": {"image_size": "2K", "aspect_ratio": "1:1", "optimize": ["png", "webp"]},
  "items": [
    {"id": "hero", "prompt": "a glowing banana", "image_size": "4K", "aspect_ratio": "16:9"},
    {"id": "logo", "template": "logo", "vars": {"brand_name": "Aphrody"}},
    {"id": "cat", "prompt": "a cat on a sofa", "enhance": "photoreal"}
  ]
}
```

```bash
aphrody image batch spec.json --out_dir out --workers 3
```

Generation runs concurrently (per-thread client, credentials read once per
worker); a `manifest.json` summarises the run.

## Material 3 icons — `aphrody image icon`

Built on the M3 spec (24dp grid, 20dp live area, 2dp stroke, outlined/rounded/
sharp, single colour, no 3D/shadow/gradient).

```bash
# Generate a custom M3 icon with Nano Banana Pro, also emit a Windows .ico
aphrody image icon gen "rocket launch" --style rounded --color "#1F1F1F" --out rocket.png --ico

# Turn any image into an M3 glyph
aphrody image icon from_image logo.png --subject "company mark" --style outlined --out mark.png --ico

# Convert Material Symbols SVGs from a checkout into Windows .ico (bulk)
aphrody image icon convert var/material-design-icons --out_dir out/icons --style outlined --names home,settings,search

# Fetch a Material Symbol by name (no checkout, via Iconify) and package it
aphrody image icon fetch home --style rounded --out home.svg --ico

# Convert a single SVG
aphrody image icon svg2ico glyph.svg glyph.ico --color "#0078D7"

# Index a local checkout / print the self-host CSS
aphrody image icon catalogue var/material-design-icons
aphrody image icon css outlined
```

**Pipeline:** SVG → `resvg-py` raster (no system Cairo) → `oxipng` lossless PNG →
multi-resolution `.ico` (16/24/32/48/64/128/256). Custom icons go PNG → `.ico`.

### Replacing Windows folder icons (safe & reversible)

```bash
aphrody image icon apply_folder "C:\\path\\to\\folder" out\\icons\\folder.ico
```

This writes a `desktop.ini` pointing the folder at the `.ico` (reversible —
delete the file to restore). aphrody **does not** modify system icons
(`shell32.dll` / registry): that is destructive and out of scope; do it manually
with a resource editor if you really want a system-wide swap.

### Our material-design-icons fork

Forked to `aphrody-code/material-design-icons`. Clone leanly into the gitignored
`var/` (only the SVGs you need):

```bash
git clone --depth 1 --filter=blob:none --sparse \
    https://github.com/aphrody-code/material-design-icons var/material-design-icons
cd var/material-design-icons
git sparse-checkout set symbols/web/home symbols/web/settings symbols/web/search
```

`symbols/web/<name>/{materialsymbolsoutlined,materialsymbolsrounded,materialsymbolssharp}/<name>_24px.svg`.

## Image analysis — `aphrody image analyze`

Deep technical fingerprint (Pillow + NumPy): format/mode/size/animation/alpha,
the subject's tight bounding box and coverage, the dominant-colour palette, the
mean colour, and an optional palette swatch — enough to recreate and vary an
image precisely.

```bash
aphrody image analyze assets/aphrody-body.webp --palette palette.png --palette_size 10
```

## Animation & sprites — `aphrody image anim`

Turn frame sequences (e.g. a model viewer's rotation frames) into a looping
**animated WebP** (smallest, full alpha), **GIF** (most compatible) or **APNG**
(lossless), or pack them into a spritesheet + JSON atlas.

```bash
# Turntable loop from rotation frames (ordered by the _r<n> index), ping-pong
aphrody image anim turntable "assets/aphrody_body_r*.webp" --out turntable.webp --fps 10 --pingpong
aphrody image anim turntable "assets/aphrody_r*.webp" --out portrait.gif --fmt gif --fps 12

# Explicit frames -> animation / spritesheet (+ atlas .json)
aphrody image anim build f0.png f1.png f2.png --out anim.webp --fps 12 --loop 0
aphrody image anim spritesheet frame0.webp frame1.webp ... --out sheet.png --columns 4
```

### Recipe — extracting sprite frames from a web model viewer

The Inazuma Eleven model viewer loads 8 rotation frames per view (portrait +
full-body = 16) from a CDN as `…/<token>_r<i>[_fullbody].webp`. Discover the
token + count by parsing the page's inline JS (look for `imageUrls`/`_r${i}`),
then download with **httpx** (a browser `User-Agent` + `Referer:
https://zukan.inazuma.jp/` are required, else the CDN 403s):

```python
import httpx
base = "https://dxi4wb638ujep.cloudfront.net/1/k/0/r/<token>"
headers = {"User-Agent": "Mozilla/5.0 … Chrome/137", "Referer": "https://zukan.inazuma.jp/"}
with httpx.Client(headers=headers, follow_redirects=True, timeout=30) as c:
    for i in range(8):
        for suffix in ("", "_fullbody"):
            data = c.get(f"{base}_r{i}{suffix}.webp").content  # verify RIFF…WEBP
```

Then `aphrody image analyze` each frame and `aphrody image anim turntable` the
set. (HTTP-client choice: **httpx** for download — sync+async+HTTP/2; **lxml** /
**selectolax** for parsing — see [python-image-toolchain.md](python-image-toolchain.md).)

## 2D → 3D — `aphrody image to3d`

Turn a 2D image into a vertex-coloured **`.glb`** mesh (viewable in Blender,
three.js, or the Windows 3D Viewer). Two backends:

```bash
# relief (default): NO GPU, NO ML — silhouette inflation + luminance displacement
aphrody image to3d assets/aphrody_body_r0.webp --out aphrody.glb --max_dim 220 --depth_scale 0.18

# depth: Depth Anything V2 monocular depth (CPU-capable, heavy) — needs aphrody[depth]
aphrody image to3d assets/aphrody_r0.webp --out aphrody_depth.glb --method depth

# --texture: UV-map the sprite as a full-image texture (a "complete" textured model)
aphrody image to3d assets/aphrody_body_r0.webp --out aphrody_textured.glb --texture
```

### Sprite → textured, animated 3D model (Blender)

For a richer textured **and animated** model, the headless Blender script
`blender_addon/scripts/sprite_to_3d.py` turns a sprite into a textured standee
(image texture + alpha + Solidify volume), keyframes a 360° spin, and exports an
**animated textured GLB** that plays in any glTF viewer:

```bash
# Wired: aphrody drives an INSTALLED Blender headlessly (no running Blender,
# no add-on, no socket) — resolves blender.exe automatically.
aphrody blender sprite3d assets/aphrody.webp --out var/imgtest/aphrody_model.glb --frames 48 --thickness 0.06
aphrody blender bin            # show the resolved Blender binary
aphrody blender optimize_glb in.glb out.glb --decimate 0.5   # bmesh weld+normals+decimate

# NVIDIA GPU (Cycles OPTIX/CUDA) — list devices, then GPU-render a turntable
aphrody blender gpu                                          # lists OPTIX/CUDA devices
aphrody blender render_gpu var/imgtest/aphrody_model.glb --out_dir gpu_tt --frames 24 --samples 64

# End-to-end on GPU: sprite -> animated textured GLB -> OPTIX render -> animated WebP
aphrody blender showcase assets/aphrody.webp --out var/imgtest/aphrody_showcase.webp --frames 24 --samples 48

# Solid multi-view turntable: the REAL 8 rotation views on a billboard + ground/shadow, GPU
aphrody blender multiview "assets/aphrody_body_r*.webp" --out var/imgtest/aphrody_multiview.webp --frames 16

# Or invoke the script directly:
blender -b -P apps/aphrody/blender_addon/scripts/sprite_to_3d.py -- \
    --image assets/aphrody.webp --out model.glb --frames 48 --render frames/
```

The result is an **animated, textured GLB** (verified: embeds an `animations`
track + a `baseColorTexture`). The `aphrody image to3d --texture` path needs no
Blender/GPU; this Blender path adds volume + an embedded spin animation.
`aphrody.bpy_runner` is the headless route (`blender -b -P`); the
`aphrody.blender` socket bridge is for a *running* Blender + the aphrody add-on.

- **relief** (`aphrody[to3d]` = trimesh + numpy + scipy): the subject silhouette
  (alpha or near-white-bg) is "inflated" via a distance transform and modulated
  by luminance, then a grid is displaced. Great for stickers / flat characters;
  runs anywhere, in a second.
- **depth** (`aphrody[depth]` = + transformers + torch): real monocular depth
  from Depth Anything V2 — more volumetric, needs the model download.
- For a true 360° model from the 8 rotation frames, feed them to a multi-view
  pipeline (Gaussian Splatting / nerfstudio) or a single frame to an AI
  image-to-3D model (TRELLIS.2 / Hunyuan3D / TripoSR) — see the research notes;
  those need a CUDA GPU.

## Blender bridge — `aphrody blender`

Drive a **live Blender** (via the `ahujasid/blender-mcp` addon socket on
`localhost:9876`) to import an aphrody-generated `.glb`, render it, or run any
`bpy` code — a stdlib-only client, no MCP layer. Blender must be open with the
addon server started (BlenderMCP panel > Connect).

```bash
aphrody blender scene                                  # scene summary
aphrody blender import_glb var/imgtest/aphrody.glb     # import a generated mesh
aphrody blender render --out render.png --width 1024 --height 1024
aphrody blender turntable --frames 16 --out_dir tt     # orbiting PNG sequence
aphrody blender exec "import bpy; print(len(bpy.data.objects))"
```

Full Blender API map, headless recipes and gotchas:
[blender-api-notes.md](blender-api-notes.md).

## Library use

```python
from aphrody.images import generate_image, NanoBanana
from aphrody.prompts import render_template, enhance_prompt
from aphrody.optimize import optimize_all
from aphrody.icons import generate_icon, make_windows_ico, convert_symbols

paths = generate_image("a neon banana", out="b.png", image_size="4K", aspect_ratio="1:1")
png   = generate_icon("settings gear", style="rounded")          # bytes
make_windows_ico(png, "settings.ico")
```

See also: [python-image-toolchain.md](python-image-toolchain.md) for the broader
recommended library stack (manipulation, compression, GIF/animation, sprites).
