"""Build Idle Cat Forest's original editable geometry and Unity FBX library.

Run with Blender 5.2.1: blender --background --python source-art/build_forest.py
No downloads, addons, textures, paid services, or external Python packages.
"""

import argparse
import json
import math
import random
import sys
from pathlib import Path

import bpy
from mathutils import Vector


ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / "source-art"
EXPORT = ROOT / "unity/Assets/Resources/ForestArt"
RNG = random.Random(271828)
PALETTE = {
    "bark": "51362B", "wood": "A87543", "endgrain": "D7AA6D",
    "darkwood": "372C29", "earth": "977B54", "soil": "49372B",
    "stone": "78877C", "stone_light": "BCC3AD", "slate": "405660",
    "iron": "687E89", "coal": "303D45", "copper": "C98655",
    "leaf": "477B45", "leaf_light": "83AB52", "pine": "24594A",
    "moss": "9BB96B", "straw": "D2AE5D", "grain": "E1BC58",
    "teal": "306E70", "teal_light": "659A88", "terracotta": "B56443",
    "cream": "EDDFC0", "cloth": "7C86A8", "berry": "AC5068",
    "herb": "69A49B", "flower": "C1A2C9", "water": "69B3BE",
    "fur": "D89545", "fur_light": "F5DBB0", "fur_dark": "995528",
    "nose": "B37077", "eye": "283D35", "gold": "D4AA4F",
    "ember": "F29E58", "leather": "9B684A", "gem": "76C6BA",
}
MATERIALS = {}
ASSETS = {}
CURRENT = None


def material(name):
    if name not in MATERIALS:
        rgb = tuple(int(PALETTE[name][i:i + 2], 16) / 255 for i in (0, 2, 4))
        mat = bpy.data.materials.new("Forest_" + name)
        mat.diffuse_color = (*rgb, 1)
        mat.use_nodes = True
        shader = mat.node_tree.nodes.get("Principled BSDF")
        # Blender uses linear values here; keep documented palette and FBX diffuse identical.
        shader.inputs["Base Color"].default_value = (*rgb, 1)
        shader.inputs["Roughness"].default_value = 0.88
        shader.inputs["Metallic"].default_value = 0.08 if name in {"iron", "copper", "gold"} else 0
        MATERIALS[name] = mat
    return MATERIALS[name]


def register(obj, name, mat):
    obj.name = name
    for coll in list(obj.users_collection):
        coll.objects.unlink(obj)
    CURRENT.objects.link(obj)
    obj.data.materials.append(material(mat))
    return obj


def apply(obj):
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)


def bevel(obj, width=0.04, segments=1):
    mod = obj.modifiers.new("Hand softened edges", "BEVEL")
    mod.width = width
    mod.segments = segments
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.modifier_apply(modifier=mod.name)
    return obj


def box(name, pos, size, mat="wood", edge=0.035, rotation=None):
    bpy.ops.mesh.primitive_cube_add(size=1, location=pos)
    obj = register(bpy.context.object, name, mat)
    obj.scale = size
    apply(obj)
    if rotation:
        obj.rotation_euler = rotation
    if edge:
        bevel(obj, min(edge, min(size) * 0.18))
    return obj


def orb(name, pos, size, mat="leaf", subdivisions=1, smooth=False):
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=subdivisions, radius=1, location=pos)
    obj = register(bpy.context.object, name, mat)
    obj.scale = size
    apply(obj)
    if smooth:
        for face in obj.data.polygons:
            face.use_smooth = True
    return obj


def cylinder(name, pos, radius, depth, mat="wood", vertices=10, rotation=None, top=None):
    if top is None:
        bpy.ops.mesh.primitive_cylinder_add(vertices=vertices, radius=radius, depth=depth, location=pos)
    else:
        bpy.ops.mesh.primitive_cone_add(vertices=vertices, radius1=radius, radius2=top, depth=depth, location=pos)
    obj = register(bpy.context.object, name, mat)
    if rotation:
        obj.rotation_euler = rotation
    return obj


def beam(name, a, b, width=0.1, mat="wood"):
    middle = (Vector(a) + Vector(b)) * 0.5
    obj = box(name, middle, (width, width, (Vector(b) - Vector(a)).length), mat, edge=0.012)
    obj.rotation_euler = (Vector(b) - Vector(a)).to_track_quat("Z", "Y").to_euler()
    return obj


def mesh(name, verts, faces, mat):
    data = bpy.data.meshes.new(name)
    data.from_pydata(verts, [], faces)
    data.update()
    obj = bpy.data.objects.new(name, data)
    CURRENT.objects.link(obj)
    data.materials.append(material(mat))
    return obj


def fur_marking(name, center, radii, u, v0, v1, width):
    """A narrow curved color patch that follows the ellipsoid instead of protruding."""
    verts = []
    for step in range(4):
        v = v0 + (v1 - v0) * step / 3
        taper = 0.55 if step in (0, 3) else 1
        for side in (-1, 1):
            a = u + side * width * taper
            verts.append((center[0] + radii[0] * math.sin(a) * math.cos(v),
                          center[1] - radii[1] * math.cos(a) * math.cos(v),
                          center[2] + radii[2] * math.sin(v)))
    obj = mesh(name, verts, [(i * 2, i * 2 + 1, i * 2 + 3, i * 2 + 2) for i in range(3)], "fur_dark")
    for polygon in obj.data.polygons:
        polygon.use_smooth = True
    return obj


def curve(name, points, radius, mat):
    data = bpy.data.curves.new(name, "CURVE")
    data.dimensions = "3D"
    data.bevel_depth = radius
    data.bevel_resolution = 0
    data.resolution_u = 3
    spline = data.splines.new("BEZIER")
    spline.bezier_points.add(len(points) - 1)
    for p, co in zip(spline.bezier_points, points):
        p.co = co
        p.handle_left_type = "AUTO"
        p.handle_right_type = "AUTO"
    obj = bpy.data.objects.new(name, data)
    CURRENT.objects.link(obj)
    data.materials.append(material(mat))
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.convert(target="MESH")
    return bpy.context.object


def torus(name, pos, major, minor, mat, rotation=None):
    bpy.ops.mesh.primitive_torus_add(major_segments=12, minor_segments=4, location=pos,
                                   major_radius=major, minor_radius=minor)
    obj = register(bpy.context.object, name, mat)
    if rotation:
        obj.rotation_euler = rotation
    return obj


def new_asset(name, description):
    global CURRENT
    CURRENT = bpy.data.collections.new(name)
    bpy.context.scene.collection.children.link(CURRENT)
    ASSETS[name] = {"collection": CURRENT, "description": description}


def merge_group(name, objects, pivot=(0, 0, 0)):
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    bpy.ops.object.join()
    result = bpy.context.object
    result.name = name
    bpy.context.scene.cursor.location = pivot
    bpy.ops.object.origin_set(type="ORIGIN_CURSOR")
    return result


def finish_asset(name, articulated=False):
    entry = ASSETS[name]
    objs = [obj for obj in entry["collection"].objects if obj.type == "MESH"]
    if not articulated:
        objs = [merge_group(name + "_mesh", objs)]
    root = bpy.data.objects.new(name, None)
    entry["collection"].objects.link(root)
    for obj in objs:
        obj.parent = root
        # A handful of authored convex pieces overlap intentionally. Faces are outward.
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.select_all(action="DESELECT")
        obj.select_set(True)
        bpy.ops.object.mode_set(mode="EDIT")
        bpy.ops.mesh.select_all(action="SELECT")
        bpy.ops.mesh.normals_make_consistent(inside=False)
        bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.update()
    # Vertex bounds remain exact when joining preserves an active mesh's rotation.
    corners = [obj.matrix_world @ vert.co for obj in objs for vert in obj.data.vertices]
    entry["root"] = root
    entry["meshes"] = objs
    entry["vertices"] = sum(len(obj.data.vertices) for obj in objs)
    entry["triangles"] = sum(sum(len(face.vertices) - 2 for face in obj.data.polygons) for obj in objs)
    entry["bounds_blender"] = [[round(min(v[i] for v in corners), 4) for i in range(3)],
                                 [round(max(v[i] for v in corners), 4) for i in range(3)]]


def plank_floor(w=3.1, d=2.6):
    box("Stone foundation", (0, 0, 0.09), (w + 0.22, d + 0.22, 0.18), "stone", edge=0.08)
    count = round(w / 0.25)
    for i in range(count):
        box("Floor board", (-w / 2 + (i + 0.5) * w / count, 0, 0.2),
            (w / count - 0.015, d, 0.08), "wood" if i % 3 else "endgrain", edge=0.012)


def shingled_roof(w, back, front, eaves, ridge, color):
    """A real pitched rear roof; the open front exposes each station from above."""
    half = w / 2 + 0.08
    slope = math.atan2(ridge - eaves, half)
    variants = {"teal": "teal_light", "leaf": "pine", "cloth": "teal",
                "terracotta": "wood", "cream": "straw", "straw": "endgrain",
                "teal_light": "teal"}
    rows = 4
    columns = max(3, round((back - front) / 0.27))
    for side in (-1, 1):
        for row in range(rows):
            dist = (row + 0.5) * half / rows
            for col in range(columns):
                y = front + (col + 0.5) * (back - front) / columns
                z = ridge - dist * math.tan(slope) + (rows - row) * 0.012
                tint = variants.get(color, "endgrain") if (row + col * 3) % 7 == 0 else color
                box("Overlapping roof shingle", (side * dist, y, z),
                    (half / rows / math.cos(slope) + 0.055, (back - front) / columns + 0.026, 0.065),
                    tint, edge=0.018, rotation=(0, side * slope, 0))
        for y in (front + 0.035, back - 0.055):
            beam("Gable rafter", (0, y, ridge - 0.075), (side * half, y, eaves - 0.075), 0.13, "bark")
        beam("Eave fascia", (side * half, front - 0.025, eaves - 0.03),
             (side * half, back + 0.025, eaves - 0.03), 0.105, "endgrain")
    beam("Carved ridge cap", (0, front - 0.12, ridge + 0.065),
         (0, back + 0.12, ridge + 0.065), 0.15, "bark")
    for y in (front, back):
        beam("Gable tie", (-half, y, eaves - 0.11), (half, y, eaves - 0.11), 0.12, "bark")
        beam("Gable king post", (0, y, eaves - 0.11), (0, y, ridge - 0.1), 0.105, "endgrain")


def lantern(pos):
    x, y, z = pos
    box("Lantern warm glass", (x, y, z), (0.135, 0.135, 0.2), "ember", 0.014)
    for height in (-0.12, 0.12):
        box("Lantern frame", (x, y, z + height), (0.19, 0.19, 0.045), "darkwood", 0.008)
    for dx in (-0.065, 0.065):
        for dy in (-0.065, 0.065):
            beam("Lantern mullion", (x + dx, y + dy, z - 0.1),
                 (x + dx, y + dy, z + 0.1), 0.022, "darkwood")
    torus("Lantern hanger", (x, y, z + 0.2), 0.055, 0.013, "iron", (math.pi / 2, 0, 0))


def shelter(roof="teal", w=3.1, d=2.6, wall="wood", roof_height=2.48, home=False):
    plank_floor(w, d)
    for x in (-w / 2 + 0.12, w / 2 - 0.12):
        for y in (-d / 2 + 0.1, d / 2 - 0.1):
            height = 1.71 if y > 0 else 1.05
            box("Chamfered post", (x, y, 0.23 + height / 2), (0.18, 0.18, height), "bark")
            box("Stone post shoe", (x, y, 0.36), (0.24, 0.24, 0.26), "stone_light", 0.035)
            cylinder("Oak peg", (x, y - 0.1, 0.75), 0.029, 0.024, "endgrain", 6,
                     rotation=(math.pi / 2, 0, 0))
        for z in (0.36, 0.58):
            box("Cutaway side", (x, 0.18, z), (0.09, d - 0.48, 0.17), wall)
    for z in (0.35, 0.59, 0.83):
        box("Back wall", (0, d / 2 - 0.09, z), (w - 0.25, 0.12, 0.2), wall)
    beam("Lintel", (-w / 2, d / 2 - 0.1, 1.83), (w / 2, d / 2 - 0.1, 1.83), 0.18, "bark")
    shingled_roof(w, d / 2 + 0.1, 0.02 if home else 0.51, 1.93, roof_height, roof)
    for x in (-w / 2 + 0.1, w / 2 - 0.1):
        beam("Knee brace", (x, 0.5, 1.25), (x, 1.1, 1.8), 0.1, "wood")
    for y, z, width in ((-d / 2 - 0.12, 0.08, 1.2), (-d / 2 - 0.29, 0.04, 1.38)):
        box("Welcome step", (0, y, z), (width, 0.22, z * 2), "stone_light", 0.025)
    lantern((-w / 2 + 0.13, -d / 2 + 0.01, 1.32))
    for x in (-w / 2 + 0.22, w / 2 - 0.22):
        box("Foundation corner course", (x, -d / 2 - 0.01, 0.09), (0.31, 0.24, 0.16), "slate", 0.025)


def table(x=0, y=0, w=1.45, d=0.66, z=0.78):
    box("Bench top", (x, y, z), (w, d, 0.12), "endgrain")
    for dx in (-w * 0.38, w * 0.38):
        for dy in (-d * 0.33, d * 0.33):
            box("Bench leg", (x + dx, y + dy, z / 2 + 0.09), (0.11, 0.11, z - 0.15), "bark")
    box("Bench stretcher", (x, y, 0.34), (w * 0.8, 0.09, 0.1), "wood")


def log(pos=(0, 0, 0.2), length=1.5, radius=0.18, vertical=False):
    rot = None if vertical else (math.pi / 2, 0, 0)
    cylinder("Bark log", pos, radius, length, "bark", vertices=9, rotation=rot)
    for sign in (-1, 1):
        p = list(pos)
        p[2 if vertical else 1] += sign * (length / 2 + 0.004)
        cylinder("Cut growth rings", p, radius * 0.85, 0.018, "endgrain", vertices=9, rotation=rot)
        cylinder("Heartwood", p, radius * 0.4, 0.024, "wood", vertices=9, rotation=rot)


def crate(pos=(0, 0, 0.2), size=0.8, filled=False):
    x, y, z = pos
    box("Crate bottom", (x, y, z + 0.04), (size, size, 0.08), "darkwood")
    for side in (-1, 1):
        for h in range(3):
            box("Crate slat", (x + side * size / 2, y, z + 0.14 + h * size / 4),
                (0.075, size, size / 4 - 0.025), "wood")
            box("Crate slat", (x, y + side * size / 2, z + 0.14 + h * size / 4),
                (size, 0.075, size / 4 - 0.025), "wood")
        for sy in (-1, 1):
            box("Crate corner", (x + side * size * 0.45, y + sy * size * 0.45, z + size * 0.45),
                (0.075, 0.075, size * 0.88), "endgrain")
    if filled:
        for i in range(6):
            orb("Stored berries", (x + (i % 3 - 1) * size * 0.23, y + (i // 3 - 0.5) * size * 0.3,
                                   z + size * 0.62), (size * 0.2,) * 3, "berry")


def sack(pos, mat="cream", radius=0.24, height=0.5):
    x, y, z = pos
    orb("Gathered sack", (x, y, z + height * 0.42), (radius, radius * 0.82, height * 0.48), mat, 2)
    cylinder("Tied sack neck", (x, y, z + height * 0.87), radius * 0.28, height * 0.18, mat, 8)
    torus("Rope tie", (x, y, z + height * 0.8), radius * 0.28, 0.025, "bark")


def bowl(pos=(0, 0, 0), radius=0.48, content="water"):
    x, y, z = pos
    cylinder("Bowl base", (x, y, z + 0.11), radius * 0.8, 0.22, "terracotta", 12, top=radius)
    torus("Bowl rim", (x, y, z + 0.22), radius * 0.88, radius * 0.12, "terracotta")
    cylinder("Bowl contents", (x, y, z + 0.224), radius * 0.78, 0.012, content, 12)


def book(pos, cover="cloth", size=0.35, upright=False):
    x, y, z = pos
    dims = (size, size * 0.72, 0.07)
    if upright:
        dims = (0.075, size * 0.72, size)
    box("Book pages", (x, y, z), dims, "cream", edge=0.008)
    if upright:
        box("Book spine", (x, y - dims[1] / 2, z), (0.1, 0.045, size + 0.03), cover, edge=0.009)
    else:
        for zz in (-0.045, 0.045):
            box("Book cover", (x, y, z + zz), (size + 0.04, size * 0.72 + 0.035, 0.018), cover, edge=0.006)


def flower(pos, mat="flower", height=0.45):
    x, y, z = pos
    beam("Herb stem", (x, y, z), (x + 0.035, y, z + height), 0.035, "leaf")
    for side in (-1, 1):
        orb("Herb leaf", (x + side * 0.11, y, z + height * 0.5), (0.15, 0.07, 0.05), "leaf_light")
    orb("Flower cluster", (x + 0.035, y, z + height), (0.12, 0.11, 0.12), mat)


def garden(kind="herbs", width=2.4, depth=1.8):
    box("Raised earth", (0, 0, 0.1), (width, depth, 0.2), "soil", edge=0.06)
    for x in (-width / 2, width / 2):
        box("Garden edging", (x, 0, 0.14), (0.12, depth + 0.16, 0.25), "wood")
    for y in (-depth / 2, depth / 2):
        box("Garden edging", (0, y, 0.14), (width, 0.12, 0.25), "wood")
    for i in range(4):
        for j in range(3):
            x, y = (i - 1.5) * width / 4.8, (j - 1) * depth / 3.7
            if kind == "grain":
                for n in range(3):
                    xx, yy = x + (n - 1) * 0.08, y + (n % 2) * 0.07
                    h = 0.68 + RNG.random() * 0.16
                    beam("Wheat stem", (xx, yy, 0.2), (xx + 0.05, yy, h), 0.028, "straw")
                    orb("Wheat ear", (xx + 0.05, yy, h), (0.07, 0.065, 0.2), "grain")
            else:
                flower((x, y, 0.19), "flower" if kind == "catnip" else "herb", 0.34 + RNG.random() * 0.15)


def wheel(pos, radius=0.5, mat="wood", spokes=8):
    x, y, z = pos
    rot = (math.pi / 2, 0, 0)
    torus("Wheel rim", pos, radius * 0.87, radius * 0.13, mat, rotation=rot)
    cylinder("Axle hub", pos, radius * 0.19, 0.21, "iron", 10, rotation=rot)
    for i in range(spokes):
        a = i * math.tau / spokes
        beam("Wheel spoke", pos, (x + math.cos(a) * radius * 0.8, y, z + math.sin(a) * radius * 0.8),
             0.07, "endgrain")


def gear(pos, radius=0.5):
    x, y, z = pos
    verts = []
    teeth = 18
    for yy in (y - 0.028, y + 0.028):
        for i in range(teeth * 2):
            r = radius if i % 2 == 0 else radius * 0.83
            a = math.tau * i / (teeth * 2)
            verts.append((x + math.cos(a) * r, yy, z + math.sin(a) * r))
    n = teeth * 2
    faces = [tuple(range(n - 1, -1, -1)), tuple(range(n, n * 2))]
    faces += [(i, (i + 1) % n, (i + 1) % n + n, i + n) for i in range(n)]
    mesh("Toothed saw blade", verts, faces, "iron")
    cylinder("Saw spindle", pos, 0.11, 0.2, "slate", 10, rotation=(math.pi / 2, 0, 0))


def anvil(pos):
    x, y, z = pos
    cylinder("Anvil stump", (x, y, z + 0.22), 0.36, 0.44, "bark", 9)
    box("Anvil base", (x, y, z + 0.47), (0.52, 0.4, 0.11), "slate")
    box("Anvil waist", (x, y, z + 0.59), (0.3, 0.25, 0.2), "iron")
    box("Anvil striking face", (x, y, z + 0.73), (0.7, 0.42, 0.12), "iron")
    cylinder("Anvil horn", (x + 0.49, y, z + 0.73), 0.19, 0.5, "iron", 5,
             rotation=(0, math.pi / 2, 0), top=0)


def furnace(x=0.7, y=0.5, large=False):
    r, h = (0.66, 1.5) if large else (0.43, 0.76)
    cylinder("Masonry furnace", (x, y, 0.25 + h / 2), r, h, "terracotta", 10, top=r * 0.78)
    cylinder("Chimney", (x, y, h + 0.5), r * 0.35, 0.8, "stone", 8)
    cylinder("Chimney mouth", (x, y, h + 0.905), r * 0.29, 0.012, "coal", 8)
    box("Fire opening", (x, y - r + 0.005, 0.5), (r, 0.06, 0.4), "coal")
    for i in range(3):
        orb("Ember coals", (x + (i - 1) * r * 0.23, y - r - 0.035, 0.4), (r * 0.13, 0.055, 0.07), "ember")
    for hring in (0.28, 0.53):
        torus("Furnace band", (x, y, hring + 0.23), r * (1 - hring / max(h, 1) * 0.12), 0.035, "slate")


def axe(pos):
    x, y, z = pos
    beam("Axe handle", (x, y, z), (x + 0.07, y, z + 0.72), 0.075, "wood")
    mesh("Axe blade", [(x + 0.06, y - 0.035, z + 0.49), (x + 0.06, y - 0.035, z + 0.74),
                       (x + 0.37, y - 0.035, z + 0.8), (x + 0.37, y - 0.035, z + 0.43),
                       (x + 0.06, y + 0.035, z + 0.49), (x + 0.06, y + 0.035, z + 0.74),
                       (x + 0.37, y + 0.035, z + 0.8), (x + 0.37, y + 0.035, z + 0.43)],
         [(0, 1, 2, 3), (7, 6, 5, 4), (0, 4, 5, 1), (1, 5, 6, 2), (2, 6, 7, 3), (3, 7, 4, 0)], "iron")


def cat():
    new_asset("cat", "Sculpted ginger cat with head, tail and four paw pivots for procedural animation")
    orb("Torso", (0, 0.02, 0.59), (0.27, 0.47, 0.28), "fur", 3, True)
    orb("Chest bib", (0, -0.30, 0.56), (0.21, 0.18, 0.26), "fur_light", 2, True)
    for angle in (1.42, 1.82, 2.21):
        for side in (-1, 1):
            fur_marking("Tabby haunch marking", (0, 0.02, 0.59), (0.273, 0.474, 0.284),
                        side * angle, 0.06, 0.67, 0.052)
    body = merge_group("Body", list(CURRENT.objects))
    before = set(CURRENT.objects)
    orb("Cheek silhouette", (0, -0.44, 0.87), (0.32, 0.265, 0.29), "fur", 3, True)
    for side in (-1, 1):
        x = side * 0.2
        mesh("Pointed ear", [(x - 0.115, -0.41, 1.00), (x + 0.115, -0.41, 1.0), (x + side * 0.055, -0.4, 1.34),
                             (x, -0.21, 1.02)], [(0, 2, 1), (0, 3, 2), (1, 2, 3), (0, 1, 3)], "fur")
        mesh("Ear pink", [(x - 0.068, -0.417, 1.055), (x + 0.068, -0.417, 1.055),
                          (x + side * 0.035, -0.412, 1.255)], [(0, 2, 1)], "nose")
        orb("Cream muzzle", (side * 0.093, -0.665, 0.81), (0.115, 0.073, 0.085), "fur_light", 2, True)
        orb("Eye surround", (side * 0.137, -0.640, 0.965), (0.075, 0.021, 0.087), "fur_dark", 2, True)
        orb("Amber iris", (side * 0.137, -0.654, 0.965), (0.066, 0.019, 0.078), "gold", 2, True)
        orb("Cat slit pupil", (side * 0.137, -0.668, 0.965), (0.020, 0.01, 0.061), "eye", 2, True)
        orb("Eye catchlight", (side * 0.13 - 0.014, -0.680, 0.989), (0.016, 0.009, 0.022), "cream", 1)
        curve("Cat mouth", [(0, -0.719, 0.815), (side * 0.024, -0.733, 0.788),
                            (side * 0.069, -0.727, 0.796)], 0.008, "fur_dark")
        for j in range(2):
            beam("Whisker", (side * 0.15, -0.7, 0.815 - j * 0.04),
                 (side * 0.39, -0.665, 0.84 - j * 0.075), 0.014, "cream")
    orb("Nose", (0, -0.727, 0.845), (0.052, 0.028, 0.036), "nose", 1)
    for angle in (-0.39, 0, 0.39):
        fur_marking("Forehead stripe", (0, -0.44, 0.87), (0.324, 0.269, 0.294),
                    angle, 0.55, 1.01, 0.049)
    head = merge_group("Head", [o for o in CURRENT.objects if o not in before], (0, -0.31, 0.8))
    before = set(CURRENT.objects)
    curve("Upright curling tail", [(0, 0.38, 0.64), (0.03, 0.6, 0.71), (0.07, 0.72, 0.98),
                                  (0.12, 0.65, 1.2), (0.15, 0.51, 1.19)], 0.085, "fur")
    orb("Tail tip", (0.15, 0.51, 1.19), (0.089, 0.11, 0.085), "fur_dark", 2, True)
    merge_group("Tail", [o for o in CURRENT.objects if o not in before], (0, 0.36, 0.62))
    for side, label in [(-1, "Left"), (1, "Right")]:
        for y, pair in [(-0.28, "Front"), (0.3, "Back")]:
            before = set(CURRENT.objects)
            orb("Leg", (side * 0.18, y, 0.33), (0.105, 0.115, 0.26), "fur", 2, True)
            orb("Cream sock", (side * 0.18, y - 0.035, 0.105), (0.12, 0.15, 0.105), "fur_light", 2, True)
            merge_group("Paw" + pair + label, [o for o in CURRENT.objects if o not in before],
                        (side * 0.18, y, 0.48))
    finish_asset("cat", articulated=True)


def vegetation():
    new_asset("tree_oak", "Broad layered oak crown, exposed branching trunk and flared roots")
    cylinder("Tapered trunk", (0, 0, 1.3), 0.24, 2.6, "bark", 9, top=0.15)
    for i in range(6):
        a = i * math.tau / 6 + 0.13
        beam("Root flare", (0, 0, 0.35), (math.cos(a) * 0.5, math.sin(a) * 0.5, 0.03), 0.15, "bark")
        beam("Fork branch", (0, 0, 1.4), (math.cos(a) * 0.92, math.sin(a) * 0.92, 2.52), 0.15, "bark")
        crown = (math.cos(a) * (0.8 + i % 2 * 0.15), math.sin(a) * 0.83, 2.55 + (i % 3) * 0.2)
        orb("Broad leaf crown", crown, (0.86, 0.71, 0.62), "pine" if i == 4 else "leaf", 2)
        orb("Sunlit leaf spray", (crown[0] + 0.13, crown[1] - 0.09, crown[2] + 0.35),
            (0.55, 0.5, 0.32), "leaf_light" if i % 2 == 0 else "moss", 1)
    orb("Oak crown top", (0.05, 0.1, 3.26), (0.87, 0.8, 0.54), "leaf_light", 2)
    for pos in ((-0.35, -0.19, 0.06), (0.23, 0.3, 0.06)):
        orb("Root moss", pos, (0.23, 0.18, 0.07), "leaf", 1)
    finish_asset("tree_oak")
    new_asset("tree_pine", "Tiered conifer with visible trunk and irregular bough silhouette")
    cylinder("Pine trunk", (0, 0, 1.65), 0.17, 3.3, "bark", 8, top=0.06)
    for i in range(5):
        radius = 1.06 - i * 0.19
        height = 1.19 + i * 0.47
        cylinder("Pine bough core", (0.035 * (i % 2), 0, height + 0.18), radius * 0.67,
                 0.98, "pine", 9, top=0.025)
        for j in range(5):
            a = math.tau * j / 5 + i * 0.73
            pos = (math.cos(a) * radius * 0.59, math.sin(a) * radius * 0.59, height - 0.15)
            orb("Sweeping needle bough", pos, (radius * 0.53, radius * 0.45, 0.24),
                "leaf" if (i + j) % 4 == 0 else "pine", 1)
            beam("Pine branch", (0, 0, height - 0.2), (pos[0], pos[1], height - 0.15), 0.06, "bark")
    finish_asset("tree_pine")
    for name, mat in [("shrub", "leaf"), ("berry_bush", "leaf_light")]:
        new_asset(name, "Low branching shrub" if name == "shrub" else "Round berry bush with visible ripe pink fruit")
        for i in range(5):
            a = i * math.tau / 5
            pos = (math.cos(a) * 0.28, math.sin(a) * 0.28, 0.44 + 0.06 * (i % 2))
            beam("Bush stem", (0, 0, 0.02), pos, 0.06, "bark")
            orb("Bush foliage", pos, (0.4, 0.38, 0.4), mat, 2)
            if name == "berry_bush":
                for j in range(3):
                    orb("Ripe berry", (pos[0] + (j - 1) * 0.13, pos[1] - 0.27, pos[2] + 0.16),
                        (0.075,) * 3, "berry", 1)
        finish_asset(name)
    new_asset("stump", "Cut stump with exposed growth rings and roots")
    log((0, 0, 0.3), 0.6, 0.39, True)
    for i in range(4):
        a = i * math.tau / 4
        beam("Stump root", (0, 0, 0.2), (math.cos(a) * 0.6, math.sin(a) * 0.6, 0.04), 0.18, "bark")
    finish_asset("stump")
    for name, fleck in [("rock", None), ("ore_iron", "copper"), ("ore_coal", "coal")]:
        new_asset(name, "Angular stone outcrop" + (" with visible " + fleck + " inclusions" if fleck else ""))
        orb("Main outcrop", (0, 0, 0.42), (0.74, 0.61, 0.6), "stone", 1)
        orb("Split rock", (0.48, 0.22, 0.25), (0.4, 0.36, 0.35), "slate", 1)
        if fleck:
            for i in range(5):
                orb("Ore inclusion", (-0.37 + i * 0.16, -0.43, 0.42 + (i % 2) * 0.2),
                    (0.15, 0.1, 0.16), fleck, 1)
        finish_asset(name)
    for name, kind in [("grain_plot", "grain"), ("herb_plot", "herbs"), ("catnip_plot", "catnip")]:
        new_asset(name, "Raised " + kind + " bed with separate readable stems")
        garden(kind, 1.7, 1.3)
        finish_asset(name)
    new_asset("reeds", "Waterside reeds with brown seed heads")
    for i in range(9):
        x, y, h = RNG.uniform(-0.4, 0.4), RNG.uniform(-0.3, 0.3), RNG.uniform(0.55, 1.1)
        beam("Reed stem", (x, y, 0), (x + 0.1, y, h), 0.028, "leaf")
        cylinder("Cattail seed head", (x + 0.1, y, h - 0.08), 0.06, 0.22, "bark", 7)
        beam("Reed blade", (x, y, 0.12), (x - 0.15, y + 0.1, h * 0.8), 0.045, "leaf_light")
    finish_asset("reeds")


def buildings():
    names = ["den", "food_storage", "water_bowl", "beds", "herb_garden", "nursery", "elder_corner",
             "walls", "mouse_farm", "shrine", "workshop", "field", "smithy", "barracks",
             "accounting_tent", "wood_cutter", "stone_prep", "woodworking", "clothier", "tannery",
             "research_hut", "smelter", "mill", "sawmill", "school"]
    for name in names:
        new_asset(name, "Cutaway colony " + name.replace("_", " "))
        if name in {"field", "herb_garden"}:
            garden("grain" if name == "field" else "herbs", 2.9, 2.3)
            if name == "field":
                beam("Scarecrow mast", (1.25, 0.9, 0.2), (1.25, 0.9, 1.55), 0.08, "wood")
                beam("Scarecrow arms", (0.85, 0.9, 1.18), (1.65, 0.9, 1.18), 0.06, "wood")
                orb("Scarecrow head", (1.25, 0.9, 1.45), (0.15,) * 3, "straw")
        elif name == "water_bowl":
            for i in range(7):
                a = i * math.tau / 7
                orb("Spring stones", (math.cos(a) * 0.78, math.sin(a) * 0.78, 0.13), (0.26, 0.24, 0.18), "stone", 1)
            bowl((0, 0, 0), 0.72)
            beam("Water spout", (0.55, 0.4, 0.05), (0.55, 0.4, 0.8), 0.18, "wood")
            beam("Spout lip", (0.55, 0.4, 0.75), (0.22, 0.14, 0.75), 0.13, "wood")
        elif name == "walls":
            for i in range(8):
                x = (i - 3.5) * 0.3
                cylinder("Palisade stake", (x, 0, 0.73), 0.16, 1.46, "wood", 6)
                cylinder("Sharpened stake", (x, 0, 1.58), 0.16, 0.25, "endgrain", 6, top=0)
            for z in (0.36, 1.12):
                beam("Palisade rail", (-1.2, -0.18, z), (1.2, -0.18, z), 0.12, "bark")
        elif name == "shrine":
            ASSETS[name]["description"] = "Centered three-meter square sanctuary with nine paving bays, seated cat deity, halo and four open approaches"
            box("Nine tile sanctuary base", (0, 0, 0.055), (2.96, 2.96, 0.11), "slate", 0.035)
            for x in (-0.99, 0, 0.99):
                for y in (-0.99, 0, 0.99):
                    box("Sanctuary paving bay", (x, y, 0.12), (0.96, 0.96, 0.08),
                        "stone_light" if x == 0 or y == 0 else "stone", 0.035)
            for x in (-1.20, 1.20):
                for y in (-1.20, 1.20):
                    box("Corner moss bed", (x, y, 0.17), (0.3, 0.3, 0.045), "moss", 0.03)
                    flower((x, y, 0.19), "flower", 0.28)
            cylinder("Carved altar foot", (0, 0, 0.27), 0.65, 0.23, "stone_light", 8)
            cylinder("Altar pedestal", (0, 0, 0.57), 0.43, 0.48, "stone", 8, top=0.35)
            cylinder("Altar crown", (0, 0, 0.84), 0.49, 0.12, "stone_light", 8)
            orb("Seated deity haunches", (0, 0.11, 1.17), (0.4, 0.33, 0.35), "cream", 2)
            orb("Cat deity chest", (0, 0, 1.43), (0.31, 0.29, 0.47), "cream", 2)
            for x in (-0.16, 0.16):
                orb("Deity foreleg", (x, -0.22, 1.17), (0.1, 0.12, 0.34), "cream", 2)
                orb("Deity paw", (x, -0.3, 0.96), (0.13, 0.16, 0.09), "stone_light", 2)
            orb("Cat deity head", (0, -0.035, 1.89), (0.37, 0.32, 0.33), "cream", 2)
            for x in (-0.23, 0.23):
                cylinder("Deity pointed ear", (x, -0.025, 2.16), 0.17, 0.39, "cream", 3, top=0)
                curve("Deity peaceful eye", [(x * 0.38, -0.33, 1.92), (x * 0.67, -0.356, 1.88),
                                            (x, -0.314, 1.90)], 0.018, "slate")
            orb("Deity nose", (0, -0.365, 1.83), (0.065, 0.023, 0.045), "gold", 1)
            curve("Curled statue tail", [(0.3, 0.13, 1.08), (0.53, 0.0, 0.97), (0.42, -0.34, 0.96),
                                        (0.06, -0.44, 0.98)], 0.095, "cream")
            torus("Sun halo", (0, 0.31, 1.98), 0.68, 0.047, "gold", rotation=(math.pi / 2, 0, 0))
            for i in range(9):
                a = math.pi * i / 8
                beam("Halo sun ray", (math.cos(a) * 0.72, 0.31, 1.98 + math.sin(a) * 0.72),
                     (math.cos(a) * 0.82, 0.31, 1.98 + math.sin(a) * 0.82), 0.035, "gold")
            for x in (-0.85, 0.85):
                bowl((x, -0.7, 0.17), 0.22, "berry")
                lantern((x, 0.72, 0.47))
        else:
            roofs = {"den": "leaf", "nursery": "terracotta", "elder_corner": "straw", "barracks": "terracotta",
                     "clothier": "cloth", "tannery": "terracotta", "research_hut": "cloth", "school": "teal_light",
                     "mill": "straw", "smelter": "terracotta", "accounting_tent": "cream"}
            shelter(roofs.get(name, "teal"), roof_height=2.8 if name == "den" else 2.48,
                    home=name in {"den", "nursery", "elder_corner"})
            if name in {"den", "beds", "nursery", "elder_corner"}:
                positions = [(-0.92, 0.49), (0, 0.49), (0.92, 0.49), (-0.87, -0.63), (0.87, -0.63)] if name == "den" else [(-0.86, 0.26), (0, 0.26), (0.86, 0.26)]
                for i, (x, y) in enumerate(positions):
                    box("Low bed frame", (x, y, 0.34), (0.75, 0.88, 0.18), "wood")
                    box("Bed headboard", (x, y + 0.38, 0.56), (0.77, 0.065, 0.42), "bark")
                    orb("Woven nest", (x, y, 0.48), (0.33, 0.38, 0.14), "straw", 2)
                    orb("Bed quilt", (x, y - 0.08, 0.56), (0.29, 0.27, 0.06), "cloth" if i % 2 else "teal_light", 2)
                    box("Quilt stripe", (x, y - 0.12, 0.617), (0.51, 0.055, 0.012), "cream", 0.004)
                    orb("Pillow", (x, y + 0.24, 0.56), (0.24, 0.11, 0.08), "cream", 2)
                if name == "nursery":
                    for i in range(3):
                        orb("Kitten toy", (-0.4 + i * 0.4, -0.88, 0.37), (0.12,) * 3, ["berry", "gold", "cloth"][i], 2)
                    curve("Hanging mobile", [(-0.65, 0.6, 1.9), (0, 0.6, 1.73), (0.65, 0.6, 1.9)], 0.028, "gold")
                if name == "elder_corner":
                    bowl((1.1, -0.83, 0.24), 0.23, "herb")
                    book((-0.95, -0.82, 0.29), "terracotta")
                if name == "den":
                    ASSETS[name]["description"] = "Cozy timber den with five quilted beds, pitched moss-green roof, cat gable and warm entry lantern"
                    cylinder("Round cat gable", (0, -0.035, 2.30), 0.26, 0.07, "endgrain", 12,
                             rotation=(math.pi / 2, 0, 0))
                    for x in (-0.18, 0.18):
                        cylinder("Gable cat ear", (x, -0.03, 2.56), 0.1, 0.22, "endgrain", 3, top=0)
                        box("Cat gable eye", (x * 0.55, -0.078, 2.34), (0.045, 0.012, 0.075), "darkwood", 0.004)
                    orb("Gable nose", (0, -0.08, 2.22), (0.038, 0.014, 0.028), "terracotta", 1)
            elif name == "food_storage":
                for x in (-0.87, 0, 0.87):
                    crate((x, 0.3, 0.24), 0.64, True)
                for x in (-0.8, -0.25, 0.3):
                    sack((x, -0.7, 0.24), "straw", 0.23, 0.54)
            elif name == "mouse_farm":
                box("Mouse pen bedding", (0, 0, 0.27), (2.3, 1.65, 0.06), "straw")
                for x in (-1.1, 1.1):
                    for i in range(6):
                        box("Pen upright", (x, (i - 2.5) * 0.27, 0.63), (0.055, 0.055, 0.74), "wood")
                    beam("Pen rail", (x, -0.8, 0.92), (x, 0.8, 0.92), 0.08, "wood")
                for i in range(4):
                    x, y = -0.65 + (i % 2) * 0.9, -0.43 + (i // 2) * 0.8
                    orb("Mouse", (x, y, 0.38), (0.16, 0.11, 0.1), "stone", 2)
                    for yy in (-0.07, 0.07):
                        orb("Mouse ear", (x - 0.1, y + yy, 0.48), (0.05, 0.04, 0.06), "nose", 1)
                    curve("Mouse tail", [(x + 0.1, y, 0.34), (x + 0.22, y + 0.06, 0.32),
                                         (x + 0.32, y, 0.31)], 0.014, "nose")
            elif name in {"wood_cutter", "sawmill", "woodworking"}:
                table(0.25, -0.2, 1.6, 0.8)
                if name == "wood_cutter":
                    log((-0.82, -0.4, 0.55), 0.61, 0.34, True)
                    axe((-0.85, -0.4, 0.68))
                    for i in range(4):
                        log((-0.7 + i * 0.38, 0.78, 0.44), 0.7, 0.15)
                elif name == "sawmill":
                    gear((0.22, -0.17, 0.93), 0.47)
                    for y in (-0.65, 0.3):
                        box("Saw feed rail", (0, y, 0.92), (2.4, 0.09, 0.11), "iron")
                    for i in range(3):
                        log((-0.98 + i * 0.35, 0.75, 0.42), 0.6, 0.14)
                    wheel((1.35, 0.2, 0.69), 0.56)
                else:
                    for i in range(4):
                        box("Planed timber stack", (-0.5, 0.7, 0.35 + i * 0.12), (1.3, 0.5, 0.09), "endgrain")
                    box("Chair seat", (0.88, 0.2, 0.56), (0.54, 0.6, 0.1), "wood")
                    for xx in (0.67, 1.09):
                        box("Chair upright", (xx, 0.43, 0.75), (0.065, 0.065, 1.03), "wood")
                    box("Chair back", (0.88, 0.43, 1.15), (0.45, 0.065, 0.16), "endgrain")
                    box("Wood plane", (-0.2, -0.18, 0.93), (0.32, 0.18, 0.17), "slate")
            elif name == "stone_prep":
                table(0, -0.1, 1.85, 0.9, 0.65)
                for x in (-0.55, 0.05, 0.65):
                    box("Dressed stone block", (x, -0.1, 0.92), (0.48, 0.52, 0.42), "stone_light", 0.04)
                for i in range(3):
                    orb("Rough quarry stone", ((i - 1) * 0.65, 0.85, 0.48), (0.32, 0.3, 0.3), "stone", 1)
                beam("Chisel", (0.2, -0.6, 0.77), (0.55, -0.6, 0.77), 0.07, "iron")
            elif name == "mill":
                cylinder("Lower millstone", (-0.4, 0, 0.59), 0.7, 0.68, "stone", 12)
                cylinder("Rotating upper millstone", (-0.4, 0, 1), 0.66, 0.16, "stone_light", 12)
                cylinder("Grain hopper", (-0.4, 0, 1.4), 0.16, 0.56, "wood", 4, top=0.42)
                cylinder("Hopper grain", (-0.4, 0, 1.68), 0.35, 0.015, "grain", 4)
                wheel((1.35, 0.16, 0.78), 0.7)
                for y in (-0.7, 0.05, 0.73):
                    sack((0.72, y, 0.24), "cream", 0.25, 0.58)
            elif name in {"smithy", "smelter"}:
                furnace(0.73, 0.44, name == "smelter")
                if name == "smithy":
                    anvil((-0.65, -0.32, 0.24))
                    axe((-1.13, 0.6, 0.25))
                else:
                    for i in range(5):
                        box("Metal ingot", (-0.8 + (i % 2) * 0.5, -0.4 + (i // 2) * 0.38, 0.38),
                            (0.4, 0.24, 0.18), "copper", 0.04)
                    crate((-0.82, 0.78, 0.24), 0.46)
            elif name == "clothier":
                for x in (-0.87, 0.42):
                    box("Loom upright", (x, 0.08, 1.0), (0.12, 0.15, 1.5), "wood")
                for z in (0.5, 1.61):
                    beam("Loom beam", (-0.92, 0.08, z), (0.49, 0.08, z), 0.15, "endgrain")
                box("Woven cloth on loom", (-0.22, 0.05, 0.97), (1.05, 0.04, 0.62), "cloth")
                for i in range(10):
                    beam("Warp thread", (-0.75 + i * 0.115, 0.025, 0.64),
                         (-0.75 + i * 0.115, 0.025, 1.59), 0.013, "cream")
                wheel((0.97, -0.46, 0.75), 0.44)
                table(0.83, 0.62, 0.8, 0.5)
                for i in range(3):
                    cylinder("Thread spool", (0.61 + i * 0.23, 0.62, 0.98), 0.087, 0.26, "cream", 8)
            elif name == "tannery":
                for x in (-0.97, 0.38):
                    box("Drying frame post", (x, 0.5, 1.01), (0.11, 0.11, 1.54), "wood")
                beam("Drying frame rail", (-1.02, 0.5, 1.69), (0.43, 0.5, 1.69), 0.1, "wood")
                mesh("Stretched hide", [(-0.8, 0.48, 1.52), (-0.64, 0.48, 1.11), (-0.86, 0.48, 0.6),
                                        (-0.29, 0.48, 0.77), (0.2, 0.48, 0.6), (0.01, 0.48, 1.11),
                                        (0.2, 0.48, 1.52), (-0.3, 0.48, 1.4)], [tuple(range(8))], "leather")
                bowl((0.79, -0.28, 0.24), 0.43, "bark")
                table(-0.63, -0.7, 1.1, 0.42, 0.65)
            elif name in {"research_hut", "school", "accounting_tent"}:
                table(0, 0.25, 2.1, 0.8)
                for i, x in enumerate((-0.7, -0.15, 0.4)):
                    book((x, 0.22, 0.94), ["cloth", "teal", "terracotta"][i])
                if name == "research_hut":
                    orb("Celestial globe", (0.85, 0.4, 1.19), (0.24,) * 3, "teal", 2)
                    torus("Globe meridian", (0.85, 0.4, 1.19), 0.3, 0.025, "gold", rotation=(math.pi / 2, 0, 0))
                    cylinder("Globe pedestal", (0.85, 0.4, 0.94), 0.14, 0.2, "wood", 8)
                    for i in range(6):
                        book((-0.9 + i * 0.15, 0.98, 1.16), "cloth" if i % 2 else "terracotta", 0.35, True)
                elif name == "school":
                    box("Chalkboard", (0, 0.9, 1.25), (1.72, 0.11, 0.69), "pine")
                    for i in range(4):
                        box("Chalk mark", (-0.53 + i * 0.34, 0.83, 1.27), (0.18, 0.015, 0.04), "cream", 0)
                    for x in (-0.66, 0.66):
                        table(x, -0.73, 0.7, 0.45, 0.55)
                else:
                    box("Counting board", (0.7, 0.16, 0.93), (0.39, 0.4, 0.06), "bark")
                    for i in range(9):
                        orb("Counter bead", (0.57 + i % 3 * 0.13, 0.02 + i // 3 * 0.13, 1.0), (0.041,) * 3, "gold")
                    crate((-0.88, -0.74, 0.24), 0.43)
            elif name == "barracks":
                for x in (-0.9, 0.9):
                    box("Weapon rack post", (x, 0.75, 0.93), (0.1, 0.1, 1.4), "wood")
                beam("Weapon rack rail", (-0.94, 0.75, 1.27), (0.94, 0.75, 1.27), 0.11, "wood")
                for i in range(4):
                    axe((-0.85 + i * 0.45, 0.64, 0.45))
                for x in (-0.68, 0.58):
                    cylinder("Training dummy post", (x, -0.5, 0.75), 0.09, 1, "wood", 8)
                    orb("Training dummy", (x, -0.5, 1.05), (0.28, 0.23, 0.39), "straw", 2)
                    torus("Dummy target", (x, -0.715, 1.09), 0.13, 0.025, "terracotta", rotation=(math.pi / 2, 0, 0))
            elif name == "workshop":
                table(-0.25, -0.1, 1.8, 0.85)
                bowl((-0.7, 0, 0.85), 0.21, "herb")
                box("Mortar pestle", (-0.68, 0, 1.16), (0.075, 0.075, 0.35), "stone_light", rotation=(0.3, 0.25, 0))
                for i in range(3):
                    cylinder("Preserve jar", (0.25 + i * 0.33, 0.55, 0.52), 0.14, 0.48, "berry", 10)
                    cylinder("Jar lid", (0.25 + i * 0.33, 0.55, 0.77), 0.15, 0.045, "cream", 10)
                crate((0.98, -0.65, 0.24), 0.46)
        finish_asset(name)


def props():
    new_asset("stockpile", "Marked timber stockpile with open crates and stacked logs")
    for x in (-1.4, 1.4):
        box("Stockpile edge", (x, 0, 0.065), (0.12, 2.2, 0.13), "wood")
    for y in (-1.1, 1.1):
        box("Stockpile edge", (0, y, 0.065), (2.8, 0.12, 0.13), "wood")
    crate((-0.83, 0.42, 0), 0.63)
    crate((-0.05, 0.42, 0), 0.63)
    for i in range(3):
        log((0.75 + (i % 2) * 0.34, 0.2, 0.19 + i // 2 * 0.29), 1.0, 0.17)
    finish_asset("stockpile")
    new_asset("road", "One meter continuous warm gravel road with broad inset paving stones")
    box("Compacted path", (0, 0, 0.015), (1, 1, 0.03), "earth", edge=0)
    for row in range(3):
        for col in range(3):
            x, y = (col - 1) * 0.32, (row - 1) * 0.32
            box("Inset path paver", (x + RNG.uniform(-0.012, 0.012), y + RNG.uniform(-0.012, 0.012), 0.028),
                (RNG.uniform(0.26, 0.3), RNG.uniform(0.25, 0.3), 0.026),
                "stone_light" if (row + col) % 3 else "stone", edge=0.021,
                rotation=(0, 0, RNG.uniform(-0.045, 0.045)))
    finish_asset("road")
    for name in ("fence", "fence_post", "fence_rail"):
        new_asset(name, "Meter-scale split rail " + name + "; X-axis rail joins centered posts without corner gaps")
        if name != "fence_rail":
            for x in ((-0.5, 0.5) if name == "fence" else (0,)):
                box("Fence post", (x, 0, 0.4), (0.16, 0.16, 0.8), "wood", 0.018)
                cylinder("Post cap", (x, 0, 0.84), 0.114, 0.08, "endgrain", 4, top=0)
                box("Fence foot", (x, 0, 0.06), (0.18, 0.18, 0.12), "slate", 0.023)
                for z in (0.29, 0.65):
                    cylinder("Rail peg", (x, -0.087, z), 0.026, 0.018, "darkwood", 6,
                             rotation=(math.pi / 2, 0, 0))
        if name != "fence_post":
            for z in (0.29, 0.65):
                box("Continuous fence rail", (0, 0, z), (1.0, 0.085, 0.095), "wood", 0.012)
                box("Rail weathered grain", (0, -0.046, z + 0.012), (0.88, 0.009, 0.017), "endgrain", 0.003)
        finish_asset(name)
    new_asset("gate", "Open two-meter village entrance, posts at X plus/minus one meter, overhead cat crest and lanterns")
    for x in (-1, 1):
        box("Gate stone footing", (x, 0, 0.12), (0.24, 0.24, 0.24), "slate", 0.03)
        box("Gate upright", (x, 0, 0.83), (0.18, 0.18, 1.66), "bark", 0.022)
        beam("High gate knee brace", (x, 0, 1.25), (x * 0.68, 0, 1.66), 0.09, "wood")
        lantern((x, -0.15, 1.10))
    beam("Open gate lintel", (-1.09, 0, 1.72), (1.09, 0, 1.72), 0.17, "wood")
    shingled_roof(2.08, 0.24, -0.24, 1.79, 2.12, "leaf")
    cylinder("Gate crest", (0, -0.265, 1.86), 0.15, 0.04, "endgrain", 10, rotation=(math.pi / 2, 0, 0))
    for x in (-0.105, 0.105):
        cylinder("Gate crest ear", (x, -0.265, 2.01), 0.065, 0.15, "endgrain", 3, top=0)
    finish_asset("gate")
    new_asset("rail", "One meter narrow gauge track section")
    for y in (-0.4, 0, 0.4):
        box("Rail sleeper", (0, y, 0.05), (1.2, 0.15, 0.1), "wood")
    for x in (-0.38, 0.38):
        box("Steel rail", (x, 0, 0.15), (0.08, 1.0, 0.12), "iron")
    finish_asset("rail")
    new_asset("dock", "Timber pier with pilings, mooring posts and rope")
    for i in range(11):
        box("Dock plank", (0, (i - 5) * 0.28, 0.35), (1.85, 0.255, 0.13), "wood" if i % 3 else "endgrain")
    for x in (-0.79, 0.79):
        for y in (-1.25, 1.25):
            cylinder("Dock piling", (x, y, 0.36), 0.14, 0.72, "bark", 8)
            cylinder("Mooring cap", (x, y, 0.78), 0.18, 0.12, "endgrain", 8)
    torus("Coiled mooring rope", (0.4, 0.97, 0.455), 0.2, 0.025, "straw")
    finish_asset("dock")
    new_asset("cart", "Four wheel wooden haul cart with open load bed and shafts")
    crate((0, 0, 0.5), 1.1)
    for x in (-0.69, 0.69):
        for y in (-0.37, 0.37):
            # Cart axle runs left to right; rotate the standard wheel assembly as a group.
            before = set(CURRENT.objects)
            wheel((0, 0, 0), 0.34)
            obj = merge_group("Cart wheel", [o for o in CURRENT.objects if o not in before])
            obj.rotation_euler[2] = math.pi / 2
            obj.location = (x, y, 0.34)
    for x in (-0.43, 0.43):
        beam("Pull shaft", (x, -0.4, 0.59), (x, -1.75, 0.41), 0.085, "wood")
    finish_asset("cart")
    new_asset("boat", "Open clinker style river boat with benches and oars")
    outline = [(-0.61, -0.8), (-0.42, -1.28), (0, -1.6), (0.42, -1.28), (0.61, -0.8),
               (0.64, 0.9), (0.4, 1.3), (-0.4, 1.3), (-0.64, 0.9)]
    for i, p in enumerate(outline):
        q = outline[(i + 1) % len(outline)]
        for z in (0.17, 0.33, 0.49):
            beam("Clinker hull plank", (p[0] * (0.65 + z * 0.7), p[1], z),
                 (q[0] * (0.65 + z * 0.7), q[1], z), 0.17, "wood")
        beam("Gunwale", (p[0], p[1], 0.62), (q[0], q[1], 0.62), 0.09, "endgrain")
    box("Boat floor", (0, -0.04, 0.12), (0.73, 2.18, 0.12), "darkwood")
    for y in (-0.64, 0.39):
        box("Rowing bench", (0, y, 0.47), (1.09, 0.28, 0.1), "endgrain")
    for side in (-1, 1):
        beam("Oar shaft", (side * 0.15, 0.1, 0.55), (side * 1.22, -0.48, 0.31), 0.05, "wood")
        box("Oar blade", (side * 1.24, -0.49, 0.31), (0.44, 0.19, 0.055), "endgrain",
            rotation=(0, 0, side * -0.4))
    finish_asset("boat")
    new_asset("watchtower", "Raised guard platform with ladder and teal canopy")
    for x in (-0.64, 0.64):
        for y in (-0.64, 0.64):
            box("Tower leg", (x, y, 1.14), (0.2, 0.2, 2.28), "bark")
    box("Watch platform", (0, 0, 2.23), (1.58, 1.58, 0.16), "wood")
    for x in (-0.64, 0.64):
        box("Canopy post", (x, 0.64, 2.82), (0.11, 0.11, 1.2), "wood")
        beam("Tower crossbrace", (x, -0.64, 0.25), (x, 0.64, 1.95), 0.13, "wood")
    box("Tower canopy", (0, 0.42, 3.4), (1.8, 1.25, 0.14), "teal", rotation=(0.13, 0, 0))
    for x in (-0.29, 0.29):
        beam("Ladder rail", (x, -1.1, 0), (x, -0.77, 2.33), 0.07, "wood")
    for i in range(8):
        beam("Ladder rung", (-0.29, -1.08 + i * 0.04, 0.17 + i * 0.28),
             (0.29, -1.08 + i * 0.04, 0.17 + i * 0.28), 0.06, "endgrain")
    finish_asset("watchtower")
    new_asset("healing_tent", "Open herb clinic with examination nest and medicine jars")
    shelter("cream")
    table(-0.42, -0.12, 1.35, 0.9, 0.52)
    orb("Clinic cushion", (-0.42, -0.12, 0.64), (0.59, 0.37, 0.1), "teal_light", 2)
    table(0.95, 0.26, 0.62, 0.95)
    for y in (-0.05, 0.3, 0.63):
        cylinder("Medicine jar", (0.95, y, 1.02), 0.1, 0.33, "herb", 8)
        cylinder("Cork stopper", (0.95, y, 1.21), 0.065, 0.09, "straw", 8)
    for x in (-0.95, -0.55):
        flower((x, 0.88, 0.25), "flower", 0.45)
    finish_asset("healing_tent")


def cargo():
    kinds = ["food", "fish", "blessings", "materials", "stone", "refined", "logs", "lumber", "planks",
             "blocks", "tools", "water", "catnip", "grain", "flour", "preserves", "medicine", "brew",
             "herbs", "hide", "leather", "fibre", "thread", "cloth", "bone", "ore", "gem", "clay",
             "sand", "metal", "weapons", "armor"]
    for kind in kinds:
        name = "cargo_" + kind
        new_asset(name, "Distinct carried " + kind + " bundle, ground pivot for shoulder attachment")
        if kind in {"logs", "lumber", "planks", "materials"}:
            if kind in {"logs", "materials"}:
                for i in range(3):
                    log(((i % 2 - 0.5) * 0.16, 0, 0.085 + i // 2 * 0.14), 0.65, 0.079)
            else:
                for i in range(3):
                    box("Bound boards", (0, 0, 0.035 + i * 0.075), (0.33 if kind == "lumber" else 0.44, 0.63, 0.055),
                        "wood" if kind == "lumber" else "endgrain")
            for y in (-0.18, 0.18):
                box("Binding strap", (0, y, 0.17), (0.38, 0.035, 0.055), "straw", edge=0.009)
        elif kind in {"stone", "ore", "gem", "clay", "sand", "blocks", "metal"}:
            mat = {"stone": "stone", "ore": "copper", "gem": "gem", "clay": "terracotta", "sand": "straw",
                   "blocks": "stone_light", "metal": "iron"}[kind]
            for i in range(3):
                pos = ((i % 2 - 0.5) * 0.21, 0, 0.1 + i // 2 * 0.16)
                if kind in {"blocks", "metal"}:
                    box("Cut material", pos, (0.2, 0.38, 0.14), mat)
                else:
                    orb("Raw material", pos, (0.15, 0.2, 0.13), mat, 1)
        elif kind in {"water", "preserves", "medicine", "brew"}:
            mat = {"water": "water", "preserves": "berry", "medicine": "herb", "brew": "gold"}[kind]
            cylinder("Cargo jar", (0, 0, 0.22), 0.18, 0.38, mat, 10, top=0.145)
            cylinder("Jar stopper", (0, 0, 0.44), 0.1, 0.09, "cream", 8)
            torus("Jar handle", (0.17, 0, 0.29), 0.09, 0.025, "wood", rotation=(math.pi / 2, 0, 0))
        elif kind in {"grain", "flour", "food", "refined", "fibre"}:
            mat = {"grain": "grain", "flour": "cream", "food": "berry", "refined": "teal", "fibre": "straw"}[kind]
            sack((0, 0, 0), mat, 0.23, 0.45)
        elif kind in {"herbs", "catnip", "blessings"}:
            for i in range(3):
                flower(((i - 1) * 0.12, 0, 0), "herb" if kind == "herbs" else "flower" if kind == "catnip" else "gold", 0.4)
            torus("Bundle tie", (0, 0, 0.16), 0.15, 0.023, "straw")
        elif kind == "fish":
            orb("Silver fish body", (0, 0, 0.12), (0.14, 0.31, 0.11), "water", 2)
            mesh("Fish tail fin", [(-0.16, 0.45, 0.11), (0.16, 0.45, 0.11), (0, 0.24, 0.13)], [(0, 2, 1)], "teal")
            orb("Fish eye", (0.095, -0.19, 0.17), (0.021,) * 3, "eye")
        elif kind in {"hide", "leather", "cloth"}:
            mat = "cloth" if kind == "cloth" else "leather" if kind == "leather" else "fur"
            for i in range(3):
                box("Folded goods", (0, 0, 0.05 + i * 0.07), (0.4, 0.4 - i * 0.04, 0.06), mat, edge=0.035)
            box("Folded goods strap", (0, 0, 0.245), (0.035, 0.42, 0.026), "cream", edge=0.005)
        elif kind == "thread":
            for x in (-0.12, 0.12):
                cylinder("Thread spool", (x, 0, 0.18), 0.09, 0.3, "cream", 10)
                for z in (0.035, 0.325):
                    cylinder("Spool flange", (x, 0, z), 0.12, 0.03, "wood", 10)
        elif kind == "bone":
            beam("Bone shaft", (0, -0.24, 0.08), (0, 0.24, 0.08), 0.11, "cream")
            for y in (-0.24, 0.24):
                for x in (-0.055, 0.055):
                    orb("Bone joint", (x, y, 0.09), (0.08,) * 3, "cream", 1)
        elif kind in {"tools", "weapons"}:
            axe((-0.12, 0, 0))
            if kind == "weapons":
                beam("Spear shaft", (0.2, 0.05, 0), (0.2, 0.05, 0.7), 0.04, "wood")
                cylinder("Spear tip", (0.2, 0.05, 0.79), 0.065, 0.23, "iron", 4, top=0)
        elif kind == "armor":
            orb("Cat breastplate", (0, 0, 0.19), (0.28, 0.22, 0.2), "iron", 2)
            torus("Leather harness", (0, 0, 0.17), 0.24, 0.035, "leather")
        finish_asset(name)


def aim(obj, target):
    obj.rotation_euler = (Vector(target) - obj.location).to_track_quat("-Z", "Y").to_euler()


def export_and_save(skip_render=False):
    EXPORT.mkdir(parents=True, exist_ok=True)
    manifest = {"generator": "source-art/build_forest.py", "blender": bpy.app.version_string,
                "seed": 271828, "unit": "meter", "unity_forward": "+Z", "unity_up": "+Y",
                "palette_srgb_hex": PALETTE, "assets": {}}
    for name, entry in ASSETS.items():
        bpy.ops.object.select_all(action="DESELECT")
        entry["root"].select_set(True)
        for obj in entry["meshes"]:
            obj.select_set(True)
        bpy.context.view_layer.objects.active = entry["root"]
        bpy.ops.export_scene.fbx(filepath=str(EXPORT / (name + ".fbx")), use_selection=True,
                                 global_scale=1.0, apply_unit_scale=True, apply_scale_options="FBX_SCALE_UNITS",
                                 axis_forward="-Z", axis_up="Y", object_types={"EMPTY", "MESH"},
                                 use_mesh_modifiers=True, mesh_smooth_type="FACE", use_triangles=True,
                                 add_leaf_bones=False, bake_anim=False, path_mode="AUTO", embed_textures=False)
        manifest["assets"][name] = {key: entry[key] for key in ("description", "vertices", "triangles", "bounds_blender")}
        manifest["assets"][name]["mesh_count"] = len(entry["meshes"])
        manifest["assets"][name]["parts"] = [obj.name for obj in entry["meshes"]]
    (ART / "asset-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    # Arrange the editable library only after all FBXs have been exported at local origin.
    for index, entry in enumerate(ASSETS.values()):
        entry["root"].location = ((index % 10) * 4.8, (index // 10) * 4.5, 0)
    preview = bpy.data.collections.new("Preview only - not exported")
    bpy.context.scene.collection.children.link(preview)
    global CURRENT
    CURRENT = preview
    box("Gallery ground", (21.6, 18, -0.1), (51, 43, 0.15), "moss", edge=0)
    bpy.ops.object.camera_add(location=(45, -40, 59))
    camera = bpy.context.object
    camera.name = "Catalog camera"
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 57
    aim(camera, (21.6, 16.8, 0))
    bpy.context.scene.camera = camera
    bpy.ops.object.light_add(type="SUN", location=(0, 0, 12))
    sun = bpy.context.object
    sun.name = "Warm afternoon sun"
    sun.data.energy = 2.2
    sun.data.angle = math.radians(18)
    sun.rotation_euler = (0.35, -0.5, -0.35)
    bpy.ops.object.light_add(type="AREA", location=(15, -8, 25))
    bpy.context.object.data.energy = 2200
    bpy.context.object.data.shape = "DISK"
    bpy.context.object.data.size = 22
    aim(bpy.context.object, (20, 15, 0))
    scene = bpy.context.scene
    scene.world.color = (0.35, 0.4, 0.45)
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 24
    scene.cycles.use_denoising = True
    scene.render.resolution_x = 2400
    scene.render.resolution_y = 1900
    scene.render.resolution_percentage = 100
    scene.view_settings.view_transform = "AgX"
    scene.render.image_settings.file_format = "PNG"
    scene.render.film_transparent = False
    bpy.context.scene.cursor.location = (0, 0, 0)
    bpy.ops.object.select_all(action="DESELECT")
    bpy.ops.wm.save_as_mainfile(filepath=str(ART / "idle_cat_forest.blend"))
    if not skip_render:
        (ART / "previews").mkdir(exist_ok=True)
        scene.render.filepath = str(ART / "previews/catalog.png")
        bpy.ops.render.render(write_still=True)
        camera.location = (2.1, -3.4, 1.8)
        camera.data.ortho_scale = 2.3
        aim(camera, (0, 0, 0.62))
        scene.render.resolution_x = 1100
        scene.render.resolution_y = 1000
        scene.render.filepath = str(ART / "previews/cat.png")
        bpy.ops.render.render(write_still=True)
    print("FOREST_ART_COMPLETE " + json.dumps({"assets": len(ASSETS),
          "triangles": sum(e["triangles"] for e in ASSETS.values()), "export": str(EXPORT)}))


def main():
    args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-render", action="store_true")
    options = parser.parse_args(args)
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for data in list(bpy.data.collections):
        bpy.data.collections.remove(data)
    bpy.context.scene.unit_settings.system = "METRIC"
    bpy.context.scene.unit_settings.scale_length = 1.0
    cat()
    vegetation()
    buildings()
    props()
    cargo()
    export_and_save(options.skip_render)


if __name__ == "__main__":
    main()
