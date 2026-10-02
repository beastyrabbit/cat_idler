"""Build Idle Cat Forest's original editable geometry and Unity FBX library.

Run with Blender 5.2.1: blender --background --python source-art/build_forest.py
No downloads, addons, textures, paid services, or external Python packages.
"""

import json
import math
import random
import zlib
from pathlib import Path

import bpy
from mathutils import Vector


ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "source-art"
EXPORT = ROOT / "unity/Assets/Resources/ForestArt"
RNG = random.Random(271828)
PALETTE = {
    "bark": "51362B",
    "wood": "A87543",
    "endgrain": "D7AA6D",
    "darkwood": "372C29",
    "earth": "977B54",
    "soil": "49372B",
    "stone": "78877C",
    "stone_light": "BCC3AD",
    "slate": "405660",
    "iron": "687E89",
    "coal": "303D45",
    "copper": "C98655",
    "leaf": "477B45",
    "leaf_light": "83AB52",
    "pine": "24594A",
    "moss": "9BB96B",
    "straw": "D2AE5D",
    "grain": "E1BC58",
    "teal": "306E70",
    "teal_light": "659A88",
    "terracotta": "B56443",
    "cream": "EDDFC0",
    "cloth": "7C86A8",
    "berry": "AC5068",
    "herb": "69A49B",
    "flower": "C1A2C9",
    "water": "69B3BE",
    "fur": "D89545",
    "fur_light": "F5DBB0",
    "fur_dark": "995528",
    "nose": "B37077",
    "eye": "283D35",
    "gold": "D4AA4F",
    "ember": "F29E58",
    "leather": "9B684A",
    "gem": "76C6BA",
}
MATERIALS = {}
ASSETS = {}
CURRENT = None


def material(name):
    if name not in MATERIALS:
        rgb = tuple(int(PALETTE[name][i : i + 2], 16) / 255 for i in (0, 2, 4))
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
            verts.append(
                (
                    center[0] + radii[0] * math.sin(a) * math.cos(v),
                    center[1] - radii[1] * math.cos(a) * math.cos(v),
                    center[2] + radii[2] * math.sin(v),
                )
            )
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
    for p, co in zip(spline.bezier_points, points, strict=True):
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
    bpy.ops.mesh.primitive_torus_add(
        major_segments=12, minor_segments=4, location=pos, major_radius=major, minor_radius=minor
    )
    obj = register(bpy.context.object, name, mat)
    if rotation:
        obj.rotation_euler = rotation
    return obj


RNG_OFFSETS = json.loads((Path(__file__).with_name("rng_offsets.json")).read_text())


def seeded_stream(seed, name, offsets):
    """Return the deterministic per-asset stream positioned at its legacy offset."""
    if name not in offsets:
        return random.Random((seed ^ zlib.crc32(name.encode("utf-8"))) & 0xFFFFFFFF)
    stream = random.Random(seed)
    for _ in range(int(offsets[name])):
        stream.getrandbits(32)
    return stream


def rng():
    """Return the current asset stream; never import the rebound RNG directly."""
    return RNG


def _asset_rng(name):
    return seeded_stream(271828, name, RNG_OFFSETS)


def new_asset(name, description):
    global CURRENT, RNG
    RNG = _asset_rng(name)
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
    entry["bounds_blender"] = [
        [round(min(v[i] for v in corners), 4) for i in range(3)],
        [round(max(v[i] for v in corners), 4) for i in range(3)],
    ]


def plank_floor(w=3.1, d=2.6):
    box("Stone foundation", (0, 0, 0.09), (w + 0.22, d + 0.22, 0.18), "stone", edge=0.08)
    count = round(w / 0.25)
    for i in range(count):
        box(
            "Floor board",
            (-w / 2 + (i + 0.5) * w / count, 0, 0.2),
            (w / count - 0.015, d, 0.08),
            "wood" if i % 3 else "endgrain",
            edge=0.012,
        )


def shingled_roof(w, back, front, eaves, ridge, color):
    """A real pitched rear roof; the open front exposes each station from above."""
    half = w / 2 + 0.08
    slope = math.atan2(ridge - eaves, half)
    variants = {
        "teal": "teal_light",
        "leaf": "pine",
        "cloth": "teal",
        "terracotta": "wood",
        "cream": "straw",
        "straw": "endgrain",
        "teal_light": "teal",
    }
    rows = 4
    columns = max(3, round((back - front) / 0.27))
    for side in (-1, 1):
        for row in range(rows):
            dist = (row + 0.5) * half / rows
            for col in range(columns):
                y = front + (col + 0.5) * (back - front) / columns
                z = ridge - dist * math.tan(slope) + (rows - row) * 0.012
                tint = variants.get(color, "endgrain") if (row + col * 3) % 7 == 0 else color
                box(
                    "Overlapping roof shingle",
                    (side * dist, y, z),
                    (half / rows / math.cos(slope) + 0.055, (back - front) / columns + 0.026, 0.065),
                    tint,
                    edge=0.018,
                    rotation=(0, side * slope, 0),
                )
        for y in (front + 0.035, back - 0.055):
            beam("Gable rafter", (0, y, ridge - 0.075), (side * half, y, eaves - 0.075), 0.13, "bark")
        beam(
            "Eave fascia",
            (side * half, front - 0.025, eaves - 0.03),
            (side * half, back + 0.025, eaves - 0.03),
            0.105,
            "endgrain",
        )
    beam("Carved ridge cap", (0, front - 0.12, ridge + 0.065), (0, back + 0.12, ridge + 0.065), 0.15, "bark")
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
            beam("Lantern mullion", (x + dx, y + dy, z - 0.1), (x + dx, y + dy, z + 0.1), 0.022, "darkwood")
    torus("Lantern hanger", (x, y, z + 0.2), 0.055, 0.013, "iron", (math.pi / 2, 0, 0))


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
            box(
                "Crate slat", (x + side * size / 2, y, z + 0.14 + h * size / 4), (0.075, size, size / 4 - 0.025), "wood"
            )
            box(
                "Crate slat", (x, y + side * size / 2, z + 0.14 + h * size / 4), (size, 0.075, size / 4 - 0.025), "wood"
            )
        for sy in (-1, 1):
            box(
                "Crate corner",
                (x + side * size * 0.45, y + sy * size * 0.45, z + size * 0.45),
                (0.075, 0.075, size * 0.88),
                "endgrain",
            )
    if filled:
        for i in range(6):
            orb(
                "Stored berries",
                (x + (i % 3 - 1) * size * 0.23, y + (i // 3 - 0.5) * size * 0.3, z + size * 0.62),
                (size * 0.2,) * 3,
                "berry",
            )


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
        beam("Wheel spoke", pos, (x + math.cos(a) * radius * 0.8, y, z + math.sin(a) * radius * 0.8), 0.07, "endgrain")


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
    cylinder("Anvil horn", (x + 0.49, y, z + 0.73), 0.19, 0.5, "iron", 5, rotation=(0, math.pi / 2, 0), top=0)


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
    mesh(
        "Axe blade",
        [
            (x + 0.06, y - 0.035, z + 0.49),
            (x + 0.06, y - 0.035, z + 0.74),
            (x + 0.37, y - 0.035, z + 0.8),
            (x + 0.37, y - 0.035, z + 0.43),
            (x + 0.06, y + 0.035, z + 0.49),
            (x + 0.06, y + 0.035, z + 0.74),
            (x + 0.37, y + 0.035, z + 0.8),
            (x + 0.37, y + 0.035, z + 0.43),
        ],
        [(0, 1, 2, 3), (7, 6, 5, 4), (0, 4, 5, 1), (1, 5, 6, 2), (2, 6, 7, 3), (3, 7, 4, 0)],
        "iron",
    )


def aim(obj, target):
    obj.rotation_euler = (Vector(target) - obj.location).to_track_quat("-Z", "Y").to_euler()


CANONICAL_ORDER = [
    "cat",
    "tree_oak",
    "tree_pine",
    "shrub",
    "berry_bush",
    "stump",
    "rock",
    "ore_iron",
    "ore_coal",
    "grain_plot",
    "herb_plot",
    "catnip_plot",
    "reeds",
    "den",
    "food_storage",
    "water_bowl",
    "beds",
    "herb_garden",
    "nursery",
    "elder_corner",
    "walls",
    "mouse_farm",
    "shrine",
    "workshop",
    "field",
    "smithy",
    "barracks",
    "accounting_tent",
    "wood_cutter",
    "stone_prep",
    "woodworking",
    "clothier",
    "tannery",
    "research_hut",
    "smelter",
    "mill",
    "sawmill",
    "school",
    "stockpile",
    "road",
    "fence",
    "fence_post",
    "fence_rail",
    "gate",
    "rail",
    "dock",
    "cart",
    "boat",
    "watchtower",
    "healing_tent",
    "cargo_food",
    "cargo_fish",
    "cargo_blessings",
    "cargo_materials",
    "cargo_stone",
    "cargo_refined",
    "cargo_logs",
    "cargo_lumber",
    "cargo_planks",
    "cargo_blocks",
    "cargo_tools",
    "cargo_water",
    "cargo_catnip",
    "cargo_grain",
    "cargo_flour",
    "cargo_preserves",
    "cargo_medicine",
    "cargo_brew",
    "cargo_herbs",
    "cargo_hide",
    "cargo_leather",
    "cargo_fibre",
    "cargo_thread",
    "cargo_cloth",
    "cargo_bone",
    "cargo_ore",
    "cargo_gem",
    "cargo_clay",
    "cargo_sand",
    "cargo_metal",
    "cargo_weapons",
    "cargo_armor",
]


def export_one(name, entry):
    """Export one selected asset with the canonical FBX parameters."""
    bpy.ops.object.select_all(action="DESELECT")
    entry["root"].select_set(True)
    for obj in entry["meshes"]:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = entry["root"]
    bpy.ops.export_scene.fbx(
        filepath=str(EXPORT / f"{name}.fbx"),
        use_selection=True,
        global_scale=1.0,
        apply_unit_scale=True,
        apply_scale_options="FBX_SCALE_UNITS",
        axis_forward="-Z",
        axis_up="Y",
        object_types={"EMPTY", "MESH"},
        use_mesh_modifiers=True,
        mesh_smooth_type="FACE",
        use_triangles=True,
        add_leaf_bones=False,
        bake_anim=False,
        path_mode="AUTO",
        embed_textures=False,
    )


def export_and_save(skip_render=False, export=True):
    EXPORT.mkdir(parents=True, exist_ok=True)
    manifest = {
        "generator": "source-art/build_forest.py",
        "blender": bpy.app.version_string,
        "seed": 271828,
        "unit": "meter",
        "unity_forward": "+Z",
        "unity_up": "+Y",
        "palette_srgb_hex": PALETTE,
        "assets": {},
    }
    for name, entry in ASSETS.items():
        if export:
            export_one(name, entry)
        manifest["assets"][name] = {
            key: entry[key] for key in ("description", "vertices", "triangles", "bounds_blender")
        }
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
    print(
        "FOREST_ART_COMPLETE "
        + json.dumps(
            {"assets": len(ASSETS), "triangles": sum(e["triangles"] for e in ASSETS.values()), "export": str(EXPORT)}
        )
    )


# Public geometry vocabulary imported by asset builder modules.
__all__ = [
    "ASSETS",
    "CURRENT",
    "rng",
    "PALETTE",
    "seeded_stream",
    "Vector",
    "math",
    "new_asset",
    "finish_asset",
    "merge_group",
    "register",
    "apply",
    "bevel",
    "material",
    "box",
    "orb",
    "cylinder",
    "beam",
    "mesh",
    "fur_marking",
    "curve",
    "torus",
    "plank_floor",
    "shingled_roof",
    "lantern",
    "table",
    "log",
    "crate",
    "sack",
    "bowl",
    "book",
    "flower",
    "garden",
    "wheel",
    "gear",
    "anvil",
    "furnace",
    "axe",
    "aim",
    "CANONICAL_ORDER",
    "random",
]
