# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Optimise a GLB headlessly with Blender's native bmesh.

Imports a ``.glb``, runs a pro mesh cleanup on every mesh (merge-by-distance,
recalc normals, optional decimate), and re-exports a ``.glb``. Run via the
aphrody ``BlenderRunner`` or directly:

    blender -b --factory-startup -P optimize_glb.py -- \
        --in in.glb --out out.glb --decimate 0.5 --merge 0.0001
"""

from __future__ import annotations

import argparse
import sys

import bmesh
import bpy


def _parse_args() -> argparse.Namespace:
    argv = sys.argv
    argv = argv[argv.index("--") + 1 :] if "--" in argv else []
    parser = argparse.ArgumentParser(prog="optimize_glb")
    parser.add_argument("--in", dest="input", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--decimate", type=float, default=1.0)
    parser.add_argument("--merge", type=float, default=0.0001)
    return parser.parse_args(argv)


def main() -> None:
    """Import, optimise every mesh, and re-export the GLB."""
    args = _parse_args()
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    bpy.ops.import_scene.gltf(filepath=args.input)

    before_v = before_p = after_v = after_p = 0
    for obj in [o for o in bpy.data.objects if o.type == "MESH"]:
        me = obj.data
        before_v += len(me.vertices)
        before_p += len(me.polygons)
        bm = bmesh.new()
        bm.from_mesh(me)
        if args.merge > 0:
            bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=args.merge)
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        bm.to_mesh(me)
        bm.free()
        me.update()
        if args.decimate < 1.0:
            mod = obj.modifiers.new("aphrody_decimate", "DECIMATE")
            mod.ratio = args.decimate
            with bpy.context.temp_override(object=obj):
                bpy.ops.object.modifier_apply(modifier=mod.name)
        after_v += len(me.vertices)
        after_p += len(me.polygons)

    bpy.ops.export_scene.gltf(
        filepath=args.out, export_format="GLB", export_apply=True
    )
    print(
        f"[aphrody] optimized GLB: verts {before_v}->{after_v} "
        f"polys {before_p}->{after_p} -> {args.out}"
    )


if __name__ == "__main__":
    try:
        main()
    except Exception:  # `blender -b -P` does not exit non-zero on script error
        import traceback

        traceback.print_exc()
        sys.exit(1)
