# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""GPU-accelerated Cycles rendering — exploit an NVIDIA GPU (OPTIX/CUDA).

Headless Blender script that enables Cycles GPU compute (OPTIX preferred on RTX,
CUDA fallback), optionally imports a ``.glb``, frames it, and renders a turntable
PNG sequence on the GPU. With ``--probe`` it just enumerates the Cycles devices
as JSON and exits.

    blender -b --factory-startup -P gpu_render.py -- --probe
    blender -b --factory-startup -P gpu_render.py -- \
        --glb model.glb --out-dir frames --frames 24 --samples 64 --device AUTO
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sys

import bpy

#: Preference order for GPU back-ends (OPTIX uses RT cores on RTX cards).
_DEVICE_ORDER = ("OPTIX", "CUDA", "HIP", "ONEAPI", "METAL")


def _parse_args() -> argparse.Namespace:
    argv = sys.argv
    argv = argv[argv.index("--") + 1 :] if "--" in argv else []
    p = argparse.ArgumentParser(prog="gpu_render")
    p.add_argument("--glb", default=None)
    p.add_argument("--out-dir", dest="out_dir", default="gpu_frames")
    p.add_argument("--frames", type=int, default=24)
    p.add_argument("--samples", type=int, default=64)
    p.add_argument("--resolution", type=int, default=512)
    p.add_argument("--device", default="AUTO", help="AUTO|OPTIX|CUDA|CPU")
    p.add_argument("--probe", action="store_true")
    return p.parse_args(argv)


def enable_gpu(device_pref: str = "AUTO") -> dict:
    """Enable Cycles GPU compute and return the chosen device summary.

    Args:
        device_pref: ``AUTO`` (best available), ``OPTIX``, ``CUDA`` or ``CPU``.

    Returns:
        A dict ``{compute_device_type, scene_device, devices:[{name,type,use}]}``.
    """
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
            continue  # back-end not compiled in
        prefs.get_devices()
        if any(d.type == dt for d in prefs.devices):
            chosen = dt
            break

    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    if chosen and device_pref != "CPU":
        for d in prefs.devices:
            d.use = d.type in (chosen, "CPU")  # GPU + CPU hybrid
        scene.cycles.device = "GPU"
    else:
        scene.cycles.device = "CPU"

    return {
        "compute_device_type": chosen,
        "scene_device": scene.cycles.device,
        "devices": [
            {"name": d.name, "type": d.type, "use": bool(d.use)}
            for d in prefs.devices
        ],
    }


def _setup_camera_light(obj) -> None:
    import mathutils

    scene = bpy.context.scene
    cam = bpy.data.objects.new(
        "aphrody_cam", bpy.data.cameras.new("aphrody_cam")
    )
    scene.collection.objects.link(cam)
    scene.camera = cam
    light_data = bpy.data.lights.new("aphrody_sun", type="SUN")
    light_data.energy = 3.0
    light = bpy.data.objects.new("aphrody_sun", light_data)
    light.rotation_euler = (math.radians(55.0), 0.0, math.radians(35.0))
    scene.collection.objects.link(light)
    size = (max(obj.dimensions) if obj else 2.0) or 2.0
    cam.location = mathutils.Vector((0.0, -size * 2.4, size * 0.4))
    cam.rotation_euler = (
        (obj.location - cam.location).to_track_quat("-Z", "Y").to_euler()
        if obj
        else (math.radians(75.0), 0.0, 0.0)
    )
    bpy.context.view_layer.update()


def main() -> None:
    """Enable the GPU and either probe devices or render a turntable."""
    args = _parse_args()
    info = enable_gpu(args.device)

    if args.probe:
        print("APHRODY_GPU_JSON " + json.dumps(info))
        return

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    obj = None
    if args.glb:
        bpy.ops.import_scene.gltf(filepath=os.path.abspath(args.glb))
        obj = next((o for o in bpy.data.objects if o.type == "MESH"), None)
    _setup_camera_light(obj)

    scene = bpy.context.scene
    scene.cycles.samples = args.samples
    # Max GPU: OptiX AI denoiser (RTX tensor cores) + persistent BVH/textures.
    try:
        scene.cycles.use_denoising = True
        scene.cycles.denoiser = "OPTIX"
        scene.render.use_persistent_data = True
    except (AttributeError, TypeError):
        pass
    scene.render.resolution_x = scene.render.resolution_y = args.resolution
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    out_dir = os.path.abspath(args.out_dir)
    os.makedirs(out_dir, exist_ok=True)

    for i in range(args.frames):
        if obj is not None:
            obj.rotation_euler[2] = (i / args.frames) * 2.0 * math.pi
            bpy.context.view_layer.update()
        scene.render.filepath = os.path.join(out_dir, f"frame_{i:03d}.png")
        bpy.ops.render.render(write_still=True)

    info["frames"] = args.frames
    info["out_dir"] = out_dir
    print("APHRODY_GPU_JSON " + json.dumps(info))


if __name__ == "__main__":
    try:
        main()
    except Exception:  # `blender -b -P` does not exit non-zero on script error
        import traceback

        traceback.print_exc()
        sys.exit(1)
