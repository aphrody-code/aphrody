# Nano-banana extract — keyless Vertex AI image generation

## What the upstream repo does

**Repo**: `https://github.com/zhongweili/nanobanana-mcp-server` (352 stars, MIT)

The repo is a FastMCP server that wraps Google Gemini image models and exposes
them as MCP tools (`generate_image`, `edit_image`, `upload_file`).

Key capabilities extracted:
- **Text-to-image** (`generate_images`): assembles a `contents` list (optional
  system instruction + full prompt with optional negative suffix), calls
  `client.models.generate_content` with
  `GenerateContentConfig(response_modalities=["TEXT","IMAGE"])`, iterates `n`
  times for multiple images.
- **Image editing** (`edit_image`): prepends `gx.Part.from_bytes(data, mime)`
  parts to the contents list, then appends the instruction string; same
  `response_modalities` config.
- **Multi-image compose**: the `create_image_parts` helper converts a list of
  `(base64, mime)` tuples to `Part` objects, which are prepended to the
  contents.  Our `compose_images` method mirrors this.
- **Response decoding**: `response.candidates[0].content.parts` is iterated;
  parts where `part.inline_data.data` is non-null carry raw image bytes (not
  base64 — the google-genai SDK already decodes them).
- **Aspect ratio**: passed as `ImageConfig(aspect_ratio=...)` inside
  `GenerateContentConfig`.
- **Model ids** used (from `config/settings.py`):
  - `gemini-2.5-flash-image` — Flash-speed image model (**our primary target**)
  - `gemini-3-pro-image-preview` — 4K quality model
  - `gemini-3.1-flash-image-preview` — Nano Banana 2 (thinking support)

## Licence and attribution

- **Upstream licence**: MIT License © 2025 Zhongwei Li
- **Risk**: MIT is permissive; adaptation + attribution is fully compliant.
- **Attribution**: preserved in `python/aphrody/aphrody/images.py` header comment
  (provenance line referencing the upstream URL and licence).

No code was copied verbatim — the logic was re-implemented using the same
`google-genai` patterns (response_modalities, inline_data extraction, Part
construction) that are documented behaviour of the SDK.

## API key → keyless Vertex mapping

| Upstream (nanobanana)              | aphrody keyless path                         |
|------------------------------------|----------------------------------------------|
| `genai.Client(api_key=...)`        | `genai.Client(vertexai=True, project=..., location=..., credentials=load_google_credentials())` |
| `GEMINI_API_KEY` / `GOOGLE_API_KEY` env var | Antigravity token from Windows Credential Manager (no env secret) |
| `auth_method = API_KEY` (default)  | `GeminiVertex(...).client` — always Vertex    |
| `gcp_project_id` + `gcp_region`    | `vertex.DEFAULT_VERTEX_PROJECT` + `DEFAULT_VERTEX_LOCATION` (resolved via `resolve_project()`) |

The `NanoBanana` class instantiates a `GeminiVertex` and reads its `.client`
property — a fully authenticated `google.genai.Client` with `vertexai=True`.

## Model ID retained

**`gemini-2.5-flash-image`**

Source: `nanobanana_mcp_server/config/settings.py` lines 139 and 228 (both
`FlashImageConfig.model_name` and `GeminiConfig.model_name`).  This is the
"nano-banana" model referenced in the repo name and README.  Overridable at
runtime via `APHRODY_IMAGE_MODEL`.

## Python dependencies to add to pyproject

Add these to `python/aphrody/pyproject.toml` (under `[project] dependencies`):

```
google-genai>=1.0.0        # already present for vertex.py; confirm >=1.x for ImageConfig
pillow>=10.4.0             # only needed if image manipulation is added later (PNG save is pure bytes in our impl — not strictly required now)
```

Strictly speaking, our `images.py` only calls `google-genai` (already a dep)
and the stdlib (`pathlib`, `mimetypes`, `base64`).  **Pillow is NOT required**
for the current implementation because we write the raw bytes returned by the
model directly (Gemini returns already-encoded PNG bytes).  Pillow would only
be needed for dimension detection or format conversion.

**Concrete minimum addition**:
```
# No new dep required beyond existing google-genai.
# If Pillow is not yet listed, add:
pillow>=10.4.0
```

## Usage example

```python
from aphrody.images import generate_image, edit_image

# Text-to-image — writes out/banana.png, returns [Path("out/banana.png")]
paths = generate_image("a red banana, studio photo", out="out/banana.png")

# Multiple images into a directory
paths = generate_image("sunset over mountains", out="out/", n=3, aspect_ratio="16:9")

# Edit an existing image
result = edit_image("out/banana.png", "make the background deep blue", out="out/banana_blue.png")

# One-shot in-memory (no file written)
img_bytes = generate_image("a glowing neon orb")[0]  # returns list[bytes]
```

CLI wrapper (to be wired in `cli.py`):
```
aphrody image "a red banana, studio photo" --out out/banana.png
aphrody image edit out/banana.png "make it neon green" --out out/banana_neon.png
```

## Notes

- `n` triggers **n independent `generate_content` calls** (not a batch
  parameter); the model returns one image per call for `gemini-2.5-flash-image`.
- `response_modalities=["TEXT","IMAGE"]` is required; omitting it causes the
  model to return text only.
- `inline_data.data` on a `Part` is already raw bytes in google-genai ≥1.x
  (the SDK decodes the base64 internally).
- For Vertex AI the location must be `"us-central1"` (or another supported
  region) — not `"global"` which nanobanana uses for its Pro model via the
  Developer API path.
