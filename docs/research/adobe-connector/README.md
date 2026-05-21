<!-- SPDX-License-Identifier: Apache-2.0 -->
# Adobe "for creativity" connector — captured corpus (third-party)

This directory is a **verbatim copy** of Adobe's official *Adobe for creativity*
Claude connector plugin, captured from the local Claude Desktop install on
2026-05-21 for reverse-engineering and interoperability study. It is **not**
aphrody source — it is upstream Adobe material, reproduced here under its own
licence for reference.

## Provenance

| Field | Value |
|---|---|
| Plugin | `adobe-for-creativity` v1.0.2 |
| Author | Adobe |
| Licence | **Apache-2.0** (compatible with aphrody) |
| Upstream | `https://github.com/adobe/skills` |
| Marketplace | Anthropic `knowledge-work-plugins` |
| Captured from | `…\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Roaming\Claude\local-agent-mode-sessions\…\rpm\plugin_017FSfZwAM3GF7xTpsbDUHoA\` |

Each `skills/*/SKILL.md` retains its original Apache-2.0 frontmatter and author.
No file was modified; `.mcp.json` → `mcp.json` and `.claude-plugin/plugin.json`
→ `plugin.json` were renamed only so they are not hidden in the tree.

## What it is

A **remote** connector: `mcp.json` points at a hosted Adobe MCP server
(`https://adobe-creativity.adobe.io/mcp`, `type: http`) — no local binary, no
secrets on disk. Auth is interactive Adobe IMS OAuth handled by the connector.
The six skills are local orchestration prompts that drive the hosted tools.

## Hosted tool surface (reverse-engineered from the skills)

- **Image (Lightroom / Sensei / Photoshop-backed):** `image_apply_auto_tone`
  (`type: cameraRawFilter`), `image_auto_straighten` (`uprightMode`,
  `constrainCrop`), `image_adjust_exposure|highlights|dark_portions|`
  `light_portions|brightness_and_contrast|vibrance_and_saturation|`
  `color_temperature` (`a`/`b`/`luminance`), `image_apply_preset` (`presetName`
  — Adaptive presets: Subject Pop, Warm Pop, Whiten Teeth, Blur Background,
  Sky Blue/Dark Drama), `image_select_subject` (`bodyParts`),
  `image_apply_gaussian_blur` / `image_apply_lens_blur` (`blurRadius`,
  `blurTarget`), `image_crop_and_resize` (`output`, `fit: reframe|pad|extract`,
  `focus: subject|face|upper_body|{prompt}|{x,y}`, `quality`),
  `image_generative_expand` (`top`/`bottom`/`left`/`right` px).
- **Express (vector/design):** `search_design` (`generalQuery`, `pageSize`,
  `startIndex`), `fill_text`, `change_background_color` (hex), `animate_design`.
- **Video:** `video_resize` (`mode: letterbox|crop|stretch`),
  `video_create_quick_cut` (`assetIds`, `target_duration`, `user_prompt`),
  `quickCutPoll`, `resizeVideoPoll`, `media_enhance_speech`.
- **Asset / board / init:** `asset_add_file`, `asset_preview_file`,
  `asset_inline_preview`, `asset_search`, `asset_initialize_file_upload`,
  `asset_finalize_file_upload`, `document_render_vector`,
  `create_firefly_board`, `adobe_mandatory_init`.

## How aphrody relates

aphrody reaches the **same underlying Firefly Services REST APIs** through its
own IMS server-to-server token, independent of this hosted connector:

| Connector hosted verb | aphrody REST equivalent |
|---|---|
| `image_apply_auto_tone` | `lrService/autoTone` (`photoshop_auto_tone`) |
| `image_auto_straighten` | `lrService/autoStraighten` (`photoshop_auto_straighten`) |
| `image_adjust_*` | `lrService/edit` + `LrEdit` (`photoshop_edit`) |
| `image_apply_preset` | `lrService/presets` (`lr_apply_preset`) |
| background removal | `sensei/cutout` (`photoshop_remove_background`) |
| `image_select_subject` / mask | `sensei/mask` (`photoshop_create_mask`) |
| `image_crop_and_resize` | `psdService/productCrop` + rendition |
| `image_generative_expand` | Firefly v3 `expand-async` (`firefly_generative_expand`) |
| (generative fill) | Firefly v3 `fill-async` (`firefly_generative_fill`) |

See `../adobe-creative-integration.md` for the full decision record and the
verified REST protocol details.
