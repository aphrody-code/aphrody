# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Multi-view impostor turntable — render the REAL rotation views, GPU.

Instead of repeating one sprite on a billboard, this uses the character's
actual rotation frames (e.g. ``aphrody_body_r0..r7``): a camera-facing plane
whose texture is swapped to the view matching each turntable angle, set in a 3D
scene with a ground plane + key light so it casts an alpha-shaped shadow. The
result reads as a solid 3D character spinning — rendered on the NVIDIA GPU
(Cycles OPTIX/CUDA).

    blender -b --factory-startup -P multiview_turntable.py -- \
        --pattern "assets/aphrody_body_r*.webp" --out-dir frames --frames 24
"""

from __future__ import annotations

import argparse
import glob
import json
import math
import os
import re
import sys

import bpy

_DEVICE_ORDER = ("OPTIX", "CUDA", "HIP", "ONEAPI", "METAL")
_INDEX_RE = re.compile(r"_r(\d+)")


def _parse_args() -> argparse.Namespace:
    argv = sys.argv
    argv = argv[argv.index("--") + 1 :] if "--" in argv else []
    p = argparse.ArgumentParser(prog="multiview_turntable")
    p.add_argument(
        "--pattern", required=True, help="Glob for the rotation views"
    )
    p.add_argument("--out-dir", dest="out_dir", default="multiview_frames")
    p.add_argument("--frames", type=int, default=24)
    p.add_argument("--samples", type=int, default=48)
    p.add_argument("--resolution", type=int, default=512)
    p.add_argument("--device", default="AUTO")
    p.add_argument("--no-ground", dest="ground", action="store_false")
    return p.parse_args(argv)


def _enable_gpu(device_pref: str) -> dict:
    prefs = bpy.context.preferences.addons["cycles"].preferences
    candidates = (
        ("CPU",)
        if device_pref == "CPU"
        else (device_pref,)
        if device_pref in _DEVICE_ORDER
        else _DEVICE_ORDER
    )
    chosen = None
    for dt in candidates:
        try:
            prefs.compute_device_type = dt
        except TypeError:
            continue
        prefs.get_devices()
        if any(d.type == dt for d in prefs.devices):
            chosen = dt
            break
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    if chosen and device_pref != "CPU":
        for d in prefs.devices:
            d.use = d.type in (chosen, "CPU")
        scene.cycles.device = "GPU"
    else:
        scene.cycles.device = "CPU"
    return {"compute_device_type": chosen, "scene_device": scene.cycles.device}


def _sorted_views(pattern: str) -> list[str]:
    matches = glob.glob(pattern)

    def key(path: str) -> tuple[int, str]:
        m = _INDEX_RE.search(os.path.basename(path))
        return (int(m.group(1)) if m else 1_000_000, path)

    return [os.path.abspath(p) for p in sorted(matches, key=key)]


def main() -> None:
    """Build the impostor scene and render the multi-view turntable on the GPU."""
    args = _parse_args()
    views = _sorted_views(args.pattern)
    if not views:
        raise SystemExit(f"no views match {args.pattern!r}")

    info = _enable_gpu(args.device)

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()

    images = [bpy.data.images.load(v) for v in views]
    w, h = images[0].size
    aspect = (w / h) if h else 1.0

    # Camera-facing billboard standing on the ground (base at z=0).
    bpy.ops.mesh.primitive_plane_add(size=2.0)
    plane = bpy.context.active_object
    plane.rotation_euler = (math.radians(90.0), 0.0, 0.0)
    plane.scale = (aspect, 1.0, 1.0)
    bpy.context.view_layer.update()
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    plane.location = (0.0, 0.0, 1.0)  # half-height up so the base sits at z=0

    mat = bpy.data.materials.new("impostor")
    mat.use_nodes = True
    nt = mat.node_tree
    bsdf = nt.nodes.get("Principled BSDF")
    tex = nt.nodes.new("ShaderNodeTexImage")
    tex.image = images[0]
    nt.links.new(bsdf.inputs["Base Color"], tex.outputs["Color"])
    nt.links.new(bsdf.inputs["Alpha"], tex.outputs["Alpha"])
    try:
        mat.blend_method = "BLEND"
    except (AttributeError, TypeError):
        pass
    mat.use_backface_culling = False
    plane.data.materials.append(mat)

    scene = bpy.context.scene
    if args.ground:
        bpy.ops.mesh.primitive_plane_add(size=20.0, location=(0.0, 0.0, 0.0))
        ground = bpy.context.active_object
        gmat = bpy.data.materials.new("ground")
        gmat.use_nodes = True
        gmat.node_tree.nodes["Principled BSDF"].inputs[
            "Base Color"
        ].default_value = (0.85, 0.85, 0.88, 1.0)
        ground.data.materials.append(gmat)
        scene.render.film_transparent = False
    else:
        scene.render.film_transparent = True

    # Sun key light angled to throw the alpha-shaped shadow.
    light_data = bpy.data.lights.new("sun", type="SUN")
    light_data.energy = 3.5
    light = bpy.data.objects.new("sun", light_data)
    light.rotation_euler = (math.radians(55.0), 0.0, math.radians(35.0))
    scene.collection.objects.link(light)

    # Camera in front, slightly raised, looking at the character's centre.
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    scene.collection.objects.link(cam)
    scene.camera = cam
    cam.location = (0.0, -5.0, 1.2)
    import mathutils

    target = mathutils.Vector((0.0, 0.0, 1.0))
    cam.rotation_euler = (
        (target - cam.location).to_track_quat("-Z", "Y").to_euler()
    )

    scene.cycles.samples = args.samples
    # Max GPU: OptiX AI denoiser (RTX tensor cores) + keep BVH/textures resident
    # across frames so the GPU is not re-fed every frame.
    try:
        scene.cycles.use_denoising = True
        scene.cycles.denoiser = "OPTIX"
        scene.render.use_persistent_data = True
    except (AttributeError, TypeError):
        pass
    scene.render.resolution_x = scene.render.resolution_y = args.resolution
    scene.render.image_settings.file_format = "PNG"
    out_dir = os.path.abspath(args.out_dir)
    os.makedirs(out_dir, exist_ok=True)

    n = len(images)
    for i in range(args.frames):
        tex.image = images[(i * n) // args.frames % n]
        bpy.context.view_layer.update()
        scene.render.filepath = os.path.join(out_dir, f"frame_{i:03d}.png")
        bpy.ops.render.render(write_still=True)

    info.update({"views": n, "frames": args.frames, "out_dir": out_dir})
    print("APHRODY_GPU_JSON " + json.dumps(info))


if __name__ == "__main__":
    try:
        main()
    except Exception:  # `blender -b -P` does not exit non-zero on script error
        import traceback

        traceback.print_exc()
        sys.exit(1)
