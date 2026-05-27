# Python image / icon / sprite toolchain (2026)

Recommended libraries for image manipulation, icon work, compression, and
sprite/GIF/spritesheet animation, vetted for **clean install on Windows 11 with
no system packages**. What `aphrody` actually ships is marked **[used]**;
everything else is a vetted recommendation for when the need arises.

Versions confirmed on PyPI 2026-05-21. ⚠️ = needs a system library/binary.

## Image manipulation

| pip | ver | best at | Win wheel |
|-----|-----|---------|-----------|
| `Pillow` **[used]** | 12.2 | general-purpose I/O, ICO/WebP/AVIF | ✅ zero deps |
| `pyvips` | 3.1 | highest-throughput batch resize/crop (libvips) | ✅ `pyvips[binary]` |
| `opencv-python` | 4.13 | CV ops, NumPy-native pixel work, video frames | ✅ (~90 MB) |
| `scikit-image` | 0.26 | scientific analysis (segmentation, morphology) | ✅ |
| `Wand` | 0.7 | ImageMagick breadth | ❌ needs system ImageMagick |

For high-throughput batch resize, **pyvips** wins (streaming, multithreaded);
**Pillow** is the simplest and plenty fast (`Image.Resampling.LANCZOS`).

## Icons

| pip | ver | best at | Win wheel |
|-----|-----|---------|-----------|
| `Pillow` **[used]** | 12.2 | multi-size `.ico` writer (`sizes=`/`append_images=`) | ✅ |
| `resvg-py` **[used]** | 0.3 | **SVG rasterise with NO system Cairo** (Rust wheel) | ✅ |
| `scour` **[used]** | 0.38 | SVG minify (pure Python; `svgo` is Node → banned) | ✅ |
| `pyconify` **[used]** | 0.2 | fetch Material Symbols / 275k Iconify icons by name | ✅ |
| `icnsutil` | 1.0 | macOS `.icns` writer (Pillow's is weak) | ✅ pure Python |

**SVG on Windows:** use `resvg-py` (`import resvg_py; resvg_py.svg_to_bytes(...)`).
**Avoid `CairoSVG`** (needs Cairo DLL). Pure-Python fallback: `svglib`+`reportlab`.

## Compression / optimization

| pip | ver | best at | note |
|-----|-----|---------|------|
| `pyoxipng` **[used]** (import `oxipng`) | 9.1 | PNG lossless (Rust, multithreaded) — the default | ✅ |
| `zopflipng` | 1.1 | max PNG lossless (~5% smaller, much slower) | ✅ |
| `mozjpeg-lossless-optimization` | 1.3 | JPEG lossless re-optimize | ✅ |
| Pillow native | 12.2 | AVIF (native in 12) + WebP incl. animation | ✅ |
| `pillow-jxl-plugin` | 1.3 | JPEG-XL | ⚠️ **GPL-3.0** — external tool only, never link in |

**Max-lossless pipeline:** PNG → `pyoxipng` level 6 (→ optional `zopflipng`);
JPEG → mozjpeg; prefer **AVIF** / **WebP-lossless** when the consumer supports
it (typically far smaller than PNG). `aphrody.optimize` implements PNG+WebP+AVIF.

## Animation: GIF / APNG / animated WebP

| pip | ver | best at | note |
|-----|-----|---------|------|
| `Pillow` | 12.2 | one-stop GIF/APNG/animated-WebP (`save_all`, `duration`, `disposal`, `loop`) | ✅ |
| `pygifsicle` | 1.1 | smallest GIF (wraps gifsicle) | ⚠️ needs `gifsicle` binary |
| `imageio` | 2.37 | multi-format frame I/O incl. MP4 | ✅ (video: `imageio-ffmpeg`) |
| `apngasm-python` | 1.3 | compressed APNG | ✅ |
| `numpngw` | — | APNG/PNG straight from NumPy | ✅ |
| `moviepy` | 2.x | frames/video → MP4/GIF | FFmpeg via `imageio-ffmpeg` |

- **Smallest, best-looking GIF:** Pillow frames → `pygifsicle` optimize. (The
  `gifski` encoder is best-quality but has **no clean PyPI wheel** — ship its CLI
  if needed.)
- **Animated WebP** (smallest overall, pure Pillow):
  `frames[0].save("a.webp", save_all=True, append_images=frames[1:], duration=…, loop=0, method=6)`.
- **APNG** (lossless, broad support): `apngasm-python` (compressed) or Pillow.

## Sprites & spritesheets (video games)

| pip | ver | best at | note |
|-----|-----|---------|------|
| `PyTexturePacker` | 1.2 | turn-key atlas (MaxRects, trim, pad) → JSON/Cocos plist | ✅ pure Python |
| `rectpack` | 0.2 | bin-packing algorithm only (bring your own Pillow blit) | ✅ |
| `arcade` | 3.3 | game-ready sprite animation (`load_spritesheet`, `AnimatedTimeBasedSprite`) | ✅ |
| `pygame-ce` | 2.5 | manual slicing (`subsurface`) + blit; maintained SDL2 fork | ✅ |
| `pyglet` | 2.1 | OpenGL sprite primitives (`ImageGrid`, `Animation.from_image_sequence`) | ✅ pure Python |

- **folder of frames → spritesheet + atlas:** `PyTexturePacker` (or
  `rectpack`+Pillow for a fixed grid).
- **spritesheet → frames + animate:** `pyglet.image.ImageGrid` /
  `arcade.load_spritesheet` / `pygame_ce` `subsurface`.
- Prefer **`pygame-ce`** over mainline `pygame` (actively maintained).

## Recommended stack (use X for Y)

| task | use |
|------|-----|
| batch manipulation (throughput) | `pyvips` (`[binary]`); else `Pillow` |
| general manipulation / format I/O | `Pillow` **[used]** |
| max lossless compression | `pyoxipng` + `zopflipng` + `mozjpeg-lossless-optimization` |
| ICO generation | `Pillow` **[used]** |
| ICNS generation | `icnsutil` |
| SVG rasterise on Windows | `resvg-py` **[used]** (avoid CairoSVG) |
| SVG minify | `scour` **[used]** |
| Material Symbols / Iconify fetch | `pyconify` **[used]** |
| GIF (smallest) | Pillow → `pygifsicle` ⚠️ |
| animated WebP (smallest, clean) | Pillow `save_all` |
| APNG (compressed) | `apngasm-python` |
| AVIF / WebP encode | Pillow native **[used]** |
| JPEG-XL | `pillow-jxl-plugin` ⚠️ GPL — external only |
| spritesheet packing | `PyTexturePacker` / `rectpack`+Pillow |
| sprite slicing + game animation | `arcade` / `pyglet` |

### License & policy caveats
- **`svgo` (Node.js) is banned** repo-wide → use `scour` (pure Python).
- **`pillow-jxl-plugin` is GPL-3.0** → use only as a separate CLI process, never
  statically pulled into the Apache-2.0 distribution (cf. CLAUDE.md §7 GPL ban).
- Needs a system lib/binary (avoid for zero-system-deps Windows): `Wand`
  (ImageMagick), `CairoSVG` (Cairo), `pygifsicle` (gifsicle), plain `pyvips`
  (libvips), `gifski` (no wheel).

> **Built:** `aphrody.anim` wires the Pillow path — animated WebP / GIF / APNG
> (`build_animation`, `turntable`) and uniform-grid spritesheets with a JSON
> atlas (`make_spritesheet`), exposed as `aphrody image anim {build,turntable,
> spritesheet}`. `PyTexturePacker`/`rectpack` remain the upgrade path for
> non-uniform MaxRects atlas packing.
