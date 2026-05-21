# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Convert an aphrody sprite into a complete, textured, animated 3D model.

A headless Blender script: it turns a single sprite (with alpha or a near-white
background) into a textured "standee" — a plane carrying the sprite as a
Principled-BSDF image texture, given real volume with a Solidify modifier — then
keyframes a 360 deg spin and exports an **animated textured GLB** (any glTF
viewer plays the rotation). Optionally renders the turntable to PNGs.

Run with a full Blender (needs ``bpy``):

    blender -b -P sprite_to_3d.py -- --image assets/aphrody.webp \
        --out var/imgtest/aphrody_model.glb --frames 48 --thickness 0.06

Drive the whole sprite set from the shell by looping over the frames; or use the
aphrody ``image to3d --texture`` path for a GPU-free textured relief without
Blender.
"""

from __future__ import annotations

import argparse
import math
import os
import sys

import bpy


def _parse_args() -> argparse.Namespace:
    """Parse args after the ``--`` separator Blender passes through."""
    argv = sys.argv
    argv = argv[argv.index("--") + 1 :] if "--" in argv else []
    parser = argparse.ArgumentParser(
        prog="sprite_to_3d", description="Sprite -> textured animated 3D model"
    )
    parser.add_argument("--image", required=True, help="Source sprite path")
    parser.add_argument("--out", default="sprite_model.glb", help="Output .glb")
    parser.add_argument("--frames", type=int, default=48, help="Spin frames")
    parser.add_argument(
        "--thickness", type=float, default=0.06, help="Solidify depth"
    )
    parser.add_argument(
        "--render", default=None, help="Optional PNG render dir"
    )
    parser.add_argument(
        "--cross",
        type=int,
        default=2,
        help="Number of fanned planes (1=flat card, 2=cross billboard — stays "
        "visible from all angles during a turntable)",
    )
    return parser.parse_args(argv)


def _clear_scene() -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()


def _build_standee(image_path: str, thickness: float, cross: int = 2):
    """Create a textured, solidified cross-billboard and return the object.

    *cross* upright image planes are fanned evenly around the vertical (Z) axis
    and joined into one mesh. With ``cross >= 2`` (a cross billboard) at least
    one textured face always points at the camera, so a 360 deg turntable never
    fully vanishes the way a single flat card does edge-on.
    """
    img = bpy.data.images.load(os.path.abspath(image_path))
    width, height = img.size
    aspect = (width / height) if height else 1.0
    n = max(1, cross)

    planes = []
    for i in range(n):
        bpy.ops.mesh.primitive_plane_add(size=2.0)
        plane = bpy.context.active_object
        # Stand upright (face -Y), fan around Z, match the image aspect, then
        # bake the transform so the spin axis is clean.
        plane.rotation_euler = (
            math.radians(90.0),
            0.0,
            math.radians(i * 180.0 / n),
        )
        plane.scale = (aspect, 1.0, 1.0)
        bpy.context.view_layer.update()
        bpy.ops.object.transform_apply(
            location=False, rotation=True, scale=True
        )
        planes.append(plane)

    if len(planes) > 1:
        for plane in planes:
            plane.select_set(True)
        bpy.context.view_layer.objects.active = planes[0]
        bpy.ops.object.join()
    obj = planes[0]
    obj.name = "aphrody_sprite"

    # Material: Principled BSDF + image texture (colour + alpha), double-sided
    # (use_backface_culling False -> glTF doubleSided) so each plane shows from
    # both sides.
    mat = bpy.data.materials.new("aphrody_sprite_mat")
    mat.use_nodes = True
    nt = mat.node_tree
    bsdf = nt.nodes.get("Principled BSDF")
    tex = nt.nodes.new("ShaderNodeTexImage")
    tex.image = img
    nt.links.new(bsdf.inputs["Base Color"], tex.outputs["Color"])
    nt.links.new(bsdf.inputs["Alpha"], tex.outputs["Alpha"])
    try:  # name/availability varies across Blender 4.x / 5.x
        mat.blend_method = "BLEND"
    except (AttributeError, TypeError):
        pass
    mat.use_backface_culling = False
    mat.show_transparent_back = False
    obj.data.materials.append(mat)

    # Give it volume so it is a real model, not a flat plane.
    if thickness > 0:
        mod = obj.modifiers.new("aphrody_solidify", "SOLIDIFY")
        mod.thickness = thickness
        mod.offset = 0.0
        with bpy.context.temp_override(object=obj):
            bpy.ops.object.modifier_apply(modifier=mod.name)
    return obj


def _animate_spin(obj, frames: int) -> None:
    """Keyframe a full 360 deg Z spin over *frames* with linear interpolation.

    Sets LINEAR as the default new-keyframe interpolation via the user pref so
    no post-hoc fcurve iteration is needed — robust across Blender 4.x/5.x where
    the Action data model (slotted actions) changed and ``Action.fcurves`` was
    removed.
    """
    scene = bpy.context.scene
    scene.frame_start = 1
    scene.frame_end = frames
    obj.rotation_mode = "XYZ"
    try:
        bpy.context.preferences.edit.keyframe_new_interpolation_type = "LINEAR"
    except (AttributeError, TypeError):
        pass
    for i in range(frames + 1):
        obj.rotation_euler[2] = (i / frames) * 2.0 * math.pi
        obj.keyframe_insert("rotation_euler", index=2, frame=1 + i)


def _setup_camera_light(obj) -> None:
    """Add a framing camera + sun key light aimed at *obj*."""
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

    size = max(obj.dimensions) or 2.0
    cam.location = mathutils.Vector((0.0, -size * 2.4, size * 0.4))
    cam.rotation_euler = (
        (obj.location - cam.location).to_track_quat("-Z", "Y").to_euler()
    )
    bpy.context.view_layer.update()


def _render_sequence(out_dir: str, frames: int) -> None:
    """Render the animation to ``frame_###.png`` in *out_dir*."""
    os.makedirs(out_dir, exist_ok=True)
    scene = bpy.context.scene
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.resolution_x = scene.render.resolution_y = 512
    for f in range(scene.frame_start, scene.frame_end + 1):
        scene.frame_set(f)
        bpy.context.view_layer.update()
        scene.render.filepath = os.path.join(out_dir, f"frame_{f:03d}.png")
        bpy.ops.render.render(write_still=True)


def main() -> None:
    """Build, animate, export (and optionally render) the sprite model."""
    args = _parse_args()
    _clear_scene()
    obj = _build_standee(args.image, args.thickness, args.cross)
    _animate_spin(obj, args.frames)

    out = os.path.abspath(args.out)
    os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=out,
        export_format="GLB",
        export_animations=True,
        export_apply=False,
    )
    print(f"[aphrody] wrote animated textured GLB -> {out}")

    if args.render:
        _setup_camera_light(obj)
        _render_sequence(args.render, args.frames)
        print(f"[aphrody] rendered {args.frames} frames -> {args.render}")


if __name__ == "__main__":
    try:
        main()
    except Exception:  # `blender -b -P` does not exit non-zero on script error
        import traceback

        traceback.print_exc()
        sys.exit(1)
