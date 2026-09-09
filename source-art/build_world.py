"""Build and reimport the independent, meter-scale forest world kit with Blender.

blender --background --python-exit-code 1 --python source-art/build_world.py
Pass -- --verify-only to check existing exports without rebuilding them.
Pass -- --preview-dir /tmp/world-kit-preview to render inspection images.
Only world_* exports, world_kit.blend and world-asset-manifest.json are written.
"""

import argparse
import importlib.util
import json
import math
import random
import re
import sys
import uuid
from pathlib import Path

import bpy
from mathutils import Vector


ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / "source-art"
EXPORT = ROOT / "unity/Assets/Resources/ForestArt"
MANIFEST = ART / "world-asset-manifest.json"
SEED = 161803
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("forest_geometry", ART / "build_forest.py")
forest = importlib.util.module_from_spec(spec)
spec.loader.exec_module(forest)  # Its __main__ guard prevents rebuilding the old kit.
RNG = random.Random(SEED)
ASSETS = {}


def asset(name, description, contract):
    assert name.startswith("world_")
    forest.new_asset(name, description)
    ASSETS[name] = {"collection": forest.CURRENT, "description": description, "contract": contract}


def points(objects):
    bpy.context.view_layer.update()
    return [obj.matrix_world @ v.co for obj in objects for v in obj.data.vertices]


def bounds(verts):
    return [[round(fn(v[i] for v in verts), 6) for i in range(3)] for fn in (min, max)]


def unity(v):
    return (v[0], v[2], -v[1])


def clip(poly, axis, value, keep_less):
    """Clip a convex polygon to a half plane without external geometry packages."""
    result = []
    if not poly:
        return result
    for a, b in zip(poly, poly[1:] + poly[:1]):
        da = (a[axis] - value) * (1 if keep_less else -1)
        db = (b[axis] - value) * (1 if keep_less else -1)
        if da <= 1e-9:
            result.append(a)
        if (da < 0 < db) or (db < 0 < da):
            t = da / (da - db)
            result.append(tuple(a[i] + t * (b[i] - a[i]) for i in range(2)))
    return result


def clip_rect(poly, rect):
    x0, y0, x1, y1 = rect
    for axis, value, less in ((0, x0, False), (0, x1, True), (1, y0, False), (1, y1, True)):
        poly = clip(poly, axis, value, less)
    return poly


def area(poly):
    return abs(sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(poly, poly[1:] + poly[:1]))) / 2


def prism(name, poly, bottom, top, mat):
    n = len(poly)
    verts = [(x, y, z) for z in (bottom, top) for x, y in poly]
    faces = [tuple(reversed(range(n))), tuple(range(n, 2 * n))]
    faces += [(i, (i + 1) % n, (i + 1) % n + n, i + n) for i in range(n)]
    return forest.mesh(name, verts, faces, mat)


def voronoi_cells():
    sites = [(x * .22 + RNG.uniform(-.048, .048), y * .22 + RNG.uniform(-.048, .048))
             for y in range(-2, 3) for x in range(-2, 3)]
    for sx, sy in sites:
        poly = [(-.5, -.5), (.5, -.5), (.5, .5), (-.5, .5)]
        for tx, ty in sites:
            if (sx, sy) == (tx, ty):
                continue
            nx, ny = tx - sx, ty - sy
            c = (tx * tx + ty * ty - sx * sx - sy * sy) / 2
            result = []
            for a, b in zip(poly, poly[1:] + poly[:1]):
                da, db = nx * a[0] + ny * a[1] - c, nx * b[0] + ny * b[1] - c
                if da <= 1e-9:
                    result.append(a)
                if (da < 0 < db) or (db < 0 < da):
                    t = da / (da - db)
                    result.append((a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])))
            poly = result
        # Keep exact tile boundaries; recess joints only within the tile.
        yield [(x if abs(abs(x) - .5) < 1e-7 else sx + (x - sx) * .93,
                y if abs(abs(y) - .5) < 1e-7 else sy + (y - sy) * .93) for x, y in poly]


def grass(name, pos, height=.07, mat="leaf"):
    x, y, z = pos
    for i in range(3):
        angle = i * 2.094 + .3
        dx, dy = math.cos(angle), math.sin(angle)
        forest.mesh(name, [(x - dy * .016, y + dx * .016, z),
                           (x + dy * .016, y - dx * .016, z),
                           (x + dx * height * .48, y + dy * height * .48, z + height)],
                    [(0, 1, 2)], mat)


PORTS = {"straight": ["+Z", "-Z"], "corner": ["+Z", "+X"],
         "junction": ["-X", "+X", "+Z"], "cross": ["-X", "+X", "+Z", "-Z"], "end": ["+Z"]}
REGIONS = {"+Z": (-.34, -.5, .34, -.34), "-Z": (-.34, .34, .34, .5),
           "+X": (.34, -.34, .5, .34), "-X": (-.5, -.34, -.34, .34)}


def roads():
    for kind, ports in PORTS.items():
        asset("world_road_" + kind, "Irregular light paving with low soil and moss shoulders",
              {"grid_m": 1, "ports_unity": ports, "paving_width_m": .68,
               "paving_y_m": .045, "pivot_unity": [0, 0, 0], "naturalScale": True})
        forest.box("Low soil bed", (0, 0, -.0145), (1, 1, .041), "earth", edge=0)
        regions = [(-.34, -.34, .34, .34)] + [REGIONS[p] for p in ports]
        interior = (-.40 if "-X" in ports else -.5, -.40 if "+Z" in ports else -.5,
                    .40 if "+X" in ports else .5, .40 if "-Z" in ports else .5)
        for index, poly in enumerate(voronoi_cells()):
            tint = ("stone_light", "stone_light", "stone", "stone_light", "earth")[index % 5]
            for region in regions:
                piece = clip_rect(clip_rect(poly, region), interior)
                if len(piece) >= 3 and area(piece) > .00015:
                    prism("Paving stone %02d" % index, piece, .006, .045, tint)
        # Three irregular edge stones give every connected port the same width and joint pattern.
        for port in ports:
            for j, (left, right) in enumerate(((-.34, -.118), (-.11, .11), (.118, .34))):
                poly = [(left, -.5), (right, -.5), (right, -.43),
                        (right - .016, -.406), (left + .013, -.408), (left, -.429)]
                if port == "-Z":
                    poly = [(-x, -y) for x, y in poly]
                elif port == "+X":
                    poly = [(-y, x) for x, y in poly]
                elif port == "-X":
                    poly = [(y, -x) for x, y in poly]
                prism("Connection stone " + port + str(j), poly, .006, .045,
                      "stone_light" if j != 1 else "stone")
        # Patches and sparse tufts sit inside the grid and outside the walking strip.
        for i in range(16):
            x, y = RNG.uniform(-.455, .455), RNG.uniform(-.455, .455)
            if any(a - .045 < x < c + .045 and b - .045 < y < d + .045 for a, b, c, d in regions):
                continue
            poly = [(x + math.cos(t * math.tau / 5) * .028,
                     y + math.sin(t * math.tau / 5) * .026) for t in range(5)]
            prism("Flat moss patch", poly, .005, .009, "leaf" if i % 2 else "moss")
            if i % 2:
                grass("Shoulder blades", (x, y, .007), .042, "leaf")


def arch_stone(name, t0, t1):
    # Flat radial joins keep the arch watertight; faceted outer corners give it weight.
    profile = [(math.cos(t0) * .94, 2.1 + math.sin(t0) * 1.1),
               (math.cos(t0) * 1.5, 2.1 + math.sin(t0) * 1.4),
               (math.cos((t0 + t1) / 2) * 1.5, 2.1 + math.sin((t0 + t1) / 2) * 1.4),
               (math.cos(t1) * 1.5, 2.1 + math.sin(t1) * 1.4),
               (math.cos(t1) * .94, 2.1 + math.sin(t1) * 1.1)]
    n = len(profile)
    verts = [(x, depth, z) for depth in (-.52, .48) for x, z in profile]
    faces = [tuple(reversed(range(n))), tuple(range(n, 2 * n))]
    faces += [(i, (i + 1) % n, (i + 1) % n + n, i + n) for i in range(n)]
    return forest.mesh(name, verts, faces, "stone" if int(t0 * 12) % 3 else "stone_light")


def dungeons():
    asset("world_dungeon_entrance", "Open rock arch with warm timber posts and knee braces",
          {"width_m": 3, "height_m": 3.5, "opening_clear_min_width_m": 1.64,
           "opening_clear_height_m": 2.55, "front_unity": "+Z", "pivot_unity": [0, 0, 0], "naturalScale": True})
    for side in (-1, 1):
        for row in range(3):
            forest.box("Arch pier stone", (side * 1.22, -.02, row * .7 + .35),
                       (.56, 1.0, .7), "stone" if row % 2 else "stone_light", edge=.045)
        forest.box("Timber jamb", (side * .98, -.565, 1.35), (.2, .18, 2.7), "wood", edge=.022)
        for z in (.2, 1.5, 2.45):
            forest.box("Jamb iron strap", (side * .98, -.662, z), (.21, .024, .055), "iron", edge=.005)
        forest.beam("Knee brace", (side * 1.30, -.57, 2.03), (side * .89, -.57, 2.71), .14, "bark")
    for i in range(8):
        arch_stone("Arch voussoir %02d" % i, i * math.pi / 8, (i + 1) * math.pi / 8)
    forest.box("Timber lintel", (0, -.57, 2.78), (2.36, .24, .22), "wood", edge=.025)
    for x in (-.67, .15, .72):
        forest.orb("Arch moss", (x, -.17, 3.26 if abs(x) > .5 else 3.41), (.23, .29, .06), "pine", 1)

    asset("world_dungeon_wall", "Five courses of rough stone with an open cutaway top",
          {"length_axis_unity": "X", "length_m": 1, "height_m": 2.5, "thickness_m": .32,
           "roof": False, "pivot_unity": [0, 0, 0], "naturalScale": True})
    forest.box("Recessed dark mortar", (0, 0, 1.25), (.988, .255, 2.49), "slate", edge=0)
    for row in range(5):
        divisions = [-.5, -.17, .19, .5] if row % 2 == 0 else [-.5, -.32, .035, .37, .5]
        for col, (a, b) in enumerate(zip(divisions, divisions[1:])):
            lo, hi = a + (.008 if col else 0), b - (.008 if col < len(divisions) - 2 else 0)
            forest.box("Wall course %d stone %d" % (row, col), ((lo + hi) / 2, 0, row * .5 + .25),
                       (hi - lo, .32, .5 - (.012 if row not in (0, 4) else 0)),
                       "stone_light" if (row + col) % 4 == 0 else "stone", edge=.025)

    asset("world_dungeon_pillar", "Narrow stacked stone support with worn square feet",
          {"width_m": .35, "depth_m": .35, "height_m": 2.5, "pivot_unity": [0, 0, 0], "naturalScale": True})
    for row in range(5):
        width = .35 if row in (0, 4) else .30 + (row % 2) * .025
        forest.box("Pillar course %d" % row, (0, 0, row * .5 + .25), (width, width, .5),
                   "stone_light" if row in (0, 4) else "stone", edge=.02)

    asset("world_dungeon_stairs", "Sixteen solid stone treads descending four meters forward",
          {"width_m": 1, "run_m": 4, "descent_m": 4, "steps": 16,
           "tread_m": .25, "riser_m": .25, "direction_unity": "+Z",
           "top_unity": [0, 0, 0], "bottom_unity": [0, -4, 4],
           "pivot_unity": [0, 0, 0], "naturalScale": True})
    for i in range(16):
        a, b, top = i * .25, (i + 1) * .25, -i * .25
        # The underside slopes within the exact 4 x 4 envelope. The final riser meets the lower floor.
        ba, bb = -.25 - a * .9375, -.25 - b * .9375
        profile = [(-a, top), (-b, top), (-b, bb), (-a, ba)]
        verts = [(x, y, z) for x in (-.5, .5) for y, z in profile]
        faces = [(3, 2, 1, 0), (4, 5, 6, 7), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7)]
        joints = (-.18 + (i % 2) * .07, .22 - (i % 2) * .07)
        strips = [-.5, joints[0] - .0045, joints[0] + .0045, joints[1] - .0045, joints[1] + .0045, .5]
        for left, right in zip(strips, strips[1:]):
            start = len(verts)
            verts += [(left, -a, top), (left, -b, top), (right, -b, top), (right, -a, top)]
            faces.append(tuple(range(start, start + 4)))
        obj = forest.mesh("Descending tread %02d" % (i + 1), verts, faces, "stone")
        obj.data.materials.append(forest.material("stone_light"))
        obj.data.materials.append(forest.material("slate"))
        for j, face in enumerate(obj.data.polygons[5:]):
            face.material_index = 1 if j % 2 == 0 else 2

    asset("world_dungeon_floor", "Flat irregular stone slabs with dark recessed joints",
          {"grid_m": 1, "top_y_m": 0, "thickness_m": .08, "pivot_unity": [0, 0, 0], "naturalScale": True})
    forest.box("Floor mortar bed", (0, 0, -.054), (1, 1, .052), "slate", edge=0)
    for i, poly in enumerate(voronoi_cells()):
        prism("Floor slab %02d" % i, poly, -.032, 0, "stone_light" if i % 4 == 0 else "stone")


def crown(name, center, radius, height, mat, phase):
    x, y, z = center
    count = 11
    verts = []
    for layer, (scale, lift) in enumerate(((1, 0), (.68, height * .38))):
        for i in range(count):
            angle = phase + math.tau * i / count
            r = radius * scale * (1 + .11 * math.sin(i * 5.3 + phase))
            verts.append((x + math.cos(angle) * r, y + math.sin(angle) * r,
                          z + lift + .10 * math.sin(i * 4.1 + phase)))
    verts += [(x + .07, y + .03, z + height), (x, y, z + .13)]
    faces = []
    for i in range(count):
        j = (i + 1) % count
        faces += [(i, j, count + j, count + i), (count + i, count + j, 2 * count),
                  (j, i, 2 * count + 1)]
    obj = forest.mesh(name, verts, faces, mat)
    obj.data.materials.append(forest.material("leaf"))
    for i, face in enumerate(obj.data.polygons):
        if i % 13 == 1:
            face.material_index = 1


def vegetation():
    asset("world_tree_pine", "Layered ragged pine canopy, visible branches and flared roots",
          {"biomes": ["pine_forest", "highland"], "pivot_unity": [0, 0, 0], "naturalScale": True})
    forest.cylinder("Pine trunk", (0, 0, 2), .15, 4, "bark", 9, top=.055)
    for i in range(5):
        angle = i * math.tau / 5
        forest.beam("Pine root", (math.cos(angle) * .34, math.sin(angle) * .34, .025),
                    (0, 0, .43), .12, "bark")
    for layer, (z, radius, h) in enumerate(((1.40, 1.03, 1.65), (2.12, .87, 1.6),
                                          (2.9, .62, 1.38), (3.56, .39, 1.18))):
        for i in range(3):
            a = i * math.tau / 3 + layer * .7
            forest.beam("Pine bough", (0, 0, z + .35), (math.cos(a) * radius * .83,
                         math.sin(a) * radius * .83, z + .12), .065, "bark")
        crown("Needle tier %d" % layer, (0, 0, z), radius, h, "pine", layer * .42)

    asset("world_tree_birch", "Pale forked birch with dark bark marks and light leaf clusters",
          {"biomes": ["birch_grove", "meadow_edge"], "pivot_unity": [0, 0, 0], "naturalScale": True})
    forest.cylinder("Birch trunk", (0, 0, 1.55), .125, 3.1, "cream", 9, top=.063)
    for i in range(12):
        angle, z = i * 2.39, .22 + i * .22
        radius = .125 - z * .020
        verts = [(math.cos(angle + da) * (radius + .003), math.sin(angle + da) * (radius + .003), z + dz)
                 for da, dz in ((-.45, -.026), (.47, -.008), (.3, .035), (-.36, .015))]
        forest.mesh("Birch bark dash", verts, [(0, 1, 2, 3)], "bark")
    clusters = [(-.62, .03, 2.7, .65), (.65, .12, 2.93, .71), (-.37, .36, 3.40, .74),
                (.38, -.31, 3.60, .63), (-.02, .08, 3.99, .5)]
    for i, (x, y, z, r) in enumerate(clusters):
        forest.beam("Pale fork", (0, 0, 1.4 + i * .19), (x, y, z), .075, "cream")
        forest.orb("Birch leaf cluster", (x, y, z), (r, r * .77, r * .82),
                   "leaf_light" if i % 2 else "leaf", 2)
        forest.orb("Birch sunlit tips", (x - .1, y - .10, z + .27), (r * .62, r * .55, r * .46), "moss", 1)

    asset("world_reeds", "Loose wetland reeds with bent blades and brown seed heads",
          {"biomes": ["marsh", "riverbank"], "pivot_unity": [0, 0, 0], "naturalScale": True})
    for i in range(10):
        a = i * 2.4
        x, y = math.cos(a) * (.08 + i * .025), math.sin(a) * (.08 + i * .025)
        h = .55 + (i % 4) * .16
        forest.cylinder("Reed stem", (x, y, h / 2), .014, h, "leaf", 5, top=.009)
        if i % 2 == 0:
            forest.cylinder("Reed seed head", (x, y, h + .07), .036, .17, "bark", 6, top=.025)
        for side in (-1, 1):
            dx, dy = math.cos(a) * side, math.sin(a) * side
            forest.mesh("Bent reed blade", [(x, y, .04), (x - dy * .028, y + dx * .028, h * .50),
                        (x + dx * .19, y + dy * .19, h * .83),
                        (x + dx * .29, y + dy * .29, h * .68),
                        (x + dy * .028, y - dx * .028, h * .45)],
                        [(0, 1, 2, 3, 4), (4, 3, 2, 1, 0)], "leaf_light" if i % 3 else "herb")

    asset("world_mushrooms", "Five broad russet mushrooms with pale stems and cap spots",
          {"biomes": ["damp_forest", "dungeon"], "pivot_unity": [0, 0, 0], "naturalScale": True})
    for i, (x, y, h, r) in enumerate(((0, 0, .40, .21), (.29, .10, .26, .14),
                                    (-.24, .13, .28, .16), (-.15, -.22, .19, .115), (.21, -.19, .15, .10))):
        forest.cylinder("Mushroom stem", (x, y, h / 2), r * .23, h, "cream", 7, top=r * .17)
        forest.cylinder("Pale cap underside", (x, y, h - .015), r, .04, "cream", 10, top=r * .9)
        forest.cylinder("Russet cap", (x, y, h + r * .25), r, r * .50,
                        "terracotta" if i % 2 else "berry", 10, top=r * .20)
        for j in range(3):
            a = j * math.tau / 3 + i
            forest.orb("Cream cap spot", (x + math.cos(a) * r * .43, y + math.sin(a) * r * .43,
                        h + r * .37), (r * .115, r * .115, r * .038), "cream", 1)

    asset("world_crystals", "Faceted teal crystals growing out of a low slate cluster",
          {"biomes": ["dungeon", "highland"], "pivot_unity": [0, 0, 0], "naturalScale": True})
    forest.orb("Crystal slate base", (0, 0, .06), (.44, .32, .11), "slate", 1)
    for i, (x, y, radius, h) in enumerate(((0, 0, .15, .94), (.23, .04, .11, .61),
                                         (-.23, .06, .10, .51), (.02, -.22, .095, .44), (-.08, .20, .09, .59))):
        verts = [(x + math.cos(j * math.tau / 6 + .2) * radius,
                  y + math.sin(j * math.tau / 6 + .2) * radius, z) for z in (.04, h * .74) for j in range(6)]
        verts += [(x + .035, y - .035, h)]
        faces = [tuple(reversed(range(6)))]
        faces += [(j, (j + 1) % 6, (j + 1) % 6 + 6, j + 6) for j in range(6)]
        faces += [(j + 6, (j + 1) % 6 + 6, 12) for j in range(6)]
        obj = forest.mesh("Quartz point %d" % i, verts, faces, "gem")
        obj.data.materials.append(forest.material("teal"))
        for j, face in enumerate(obj.data.polygons):
            if j % 3 == 1:
                face.material_index = 1


def beetle():
    asset("world_cave_beetle", "Six-legged cave enemy with split armored shell, amber eyes and hooked jaws",
          {"width_m": .9, "length_m": 1.25, "forward_unity": "+Z",
           "rig": "static", "pivot_unity": [0, 0, 0], "naturalScale": True})
    forest.orb("Dark abdomen", (0, .12, .24), (.28, .37, .22), "coal", 2)
    for side in (-1, 1):
        forest.orb("Split shell left" if side < 0 else "Split shell right", (side * .126, .16, .32),
                   (.142, .33, .20), "teal", 2)
        for j in range(3):
            forest.orb("Shell copper ridge", (side * (.13 + j * .022), .0 + j * .135, .465 - j * .007),
                       (.074, .03, .022), "copper", 1)
        for j, y in enumerate((-.15, .10, .34)):
            hip = (side * .20, y, .25)
            knee = (side * .37, y + (.12 if j == 2 else -.04), .17)
            foot = (side * .44, y + (.21 if j == 2 else -.16), .025)
            forest.beam("Leg femur", hip, knee, .065, "slate")
            forest.beam("Leg tibia", knee, foot, .036, "coal")
        forest.orb("Amber eye", (side * .126, -.403, .32), (.055, .039, .055), "ember", 1)
        forest.beam("Jaw root", (side * .12, -.42, .17), (side * .18, -.57, .12), .07, "copper")
        forest.beam("Hooked jaw", (side * .18, -.57, .12), (side * .06, -.65, .13), .04, "copper")
        forest.beam("Antenna", (side * .1, -.4, .37), (side * .25, -.60, .42), .022, "bark")
    forest.orb("Armored thorax", (0, -.195, .29), (.255, .19, .17), "slate", 1)
    forest.orb("Beetle head", (0, -.365, .24), (.19, .18, .145), "coal", 2)
    # Fit the authored silhouette once; consumers must preserve natural meter scale.
    objects = list(forest.CURRENT.objects)
    lo, hi = bounds(points(objects))
    for obj in objects:
        inverse = obj.matrix_world.inverted()
        for v in obj.data.vertices:
            p = obj.matrix_world @ v.co
            p.x = (p.x - (lo[0] + hi[0]) / 2) * .9 / (hi[0] - lo[0])
            p.y = (p.y - (lo[1] + hi[1]) / 2) * 1.25 / (hi[1] - lo[1])
            p.z -= lo[2]
            v.co = inverse @ p


def exporter(name, entry):
    source = [o for o in entry["collection"].objects if o.type == "MESH"]
    root = bpy.data.objects.new(name, None)
    entry["collection"].objects.link(root)
    root["contract"] = json.dumps(entry["contract"], sort_keys=True)
    for obj in source:
        obj.parent = root
    entry["root"] = root
    copies = []
    for obj in source:
        duplicate = obj.copy()
        duplicate.data = obj.data.copy()
        duplicate.parent = None
        duplicate.matrix_world = obj.matrix_world.copy()
        bpy.context.scene.collection.objects.link(duplicate)
        copies.append(duplicate)
    joined = forest.merge_group(name + "_mesh", copies)
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.mesh.normals_make_consistent(inside=False)
    bpy.ops.mesh.quads_convert_to_tris(quad_method="BEAUTY", ngon_method="BEAUTY")
    bpy.ops.object.mode_set(mode="OBJECT")
    joined.parent = root
    root.select_set(True)
    bpy.context.view_layer.objects.active = root
    verts = points([joined])
    record = {"description": entry["description"], "contract": entry["contract"],
              "vertices": len(joined.data.vertices), "triangles": len(joined.data.polygons),
              "mesh_count": 1, "editable_part_count": len(source),
              "bounds_blender": bounds(verts), "bounds_unity": bounds([unity(p) for p in verts]),
              "pivot_unity": [0, 0, 0], "materials": sorted({m.name for m in joined.data.materials})}
    record["dimensions_unity_m"] = [round(b - a, 6) for a, b in zip(*record["bounds_unity"])]
    bpy.ops.export_scene.fbx(filepath=str(EXPORT / (name + ".fbx")), use_selection=True,
                            global_scale=1, apply_unit_scale=True, apply_scale_options="FBX_SCALE_UNITS",
                            axis_forward="-Z", axis_up="Y", object_types={"EMPTY", "MESH"},
                            use_mesh_modifiers=True, mesh_smooth_type="FACE", use_triangles=True,
                            add_leaf_bones=False, bake_anim=False, path_mode="AUTO", embed_textures=False)
    bpy.data.objects.remove(joined, do_unlink=True)
    # Keep the project's complete importer schema. Stable names produce stable GUIDs.
    meta_path = EXPORT / (name + ".fbx.meta")
    guid = uuid.uuid5(uuid.NAMESPACE_URL, "idle-cat-forest/art/" + name + ".fbx").hex
    if meta_path.exists():
        existing = re.search(r"^guid: ([0-9a-f]{32})$", meta_path.read_text(), re.M)
        assert existing and existing.group(1) == guid, (name, "refusing to replace an unrelated GUID")
    template = (EXPORT / "road.fbx.meta").read_text()
    meta_path.write_text(re.sub(r"^guid: [0-9a-f]{32}$", "guid: " + guid, template, flags=re.M))
    record["guid"] = guid
    return record


def assert_close(actual, expected, label, tolerance=.001):
    assert len(actual) == len(expected) and all(abs(a - b) < tolerance for a, b in zip(actual, expected)), (label, actual, expected)


def check_contract(name, verts, record, objects):
    uv = [unity(v) for v in verts]
    lo, hi = bounds(uv)
    size = [b - a for a, b in zip(lo, hi)]
    checks = []
    if name.startswith("world_road_"):
        assert_close([lo[0], hi[0], lo[2], hi[2]], [-.5, .5, -.5, .5], name + " grid")
        assert hi[1] < .065, (name, "tall shoulder")
        ports = record["contract"]["ports_unity"]
        for port, axis, edge in (("+X", 0, .5), ("-X", 0, -.5), ("+Z", 2, .5), ("-Z", 2, -.5)):
            paved = [v for v in uv if abs(v[axis] - edge) < .0001 and abs(v[1] - .045) < .0001]
            assert bool(paved) == (port in ports), (name, "incorrect paved port", port)
            if paved:
                across = 2 if axis == 0 else 0
                assert_close([min(v[across] for v in paved), max(v[across] for v in paved)], [-.34, .34], name + port)
        checks += ["exact 1 m grid bounds", "all requested ports and no extra ports", "matching 0.68 m paving width and 0.045 m top", "low shoulders"]
    elif name == "world_dungeon_stairs":
        assert_close(lo, [-.5, -4, 0], "stair lower bounds")
        assert_close(hi, [.5, 0, 4], "stair upper bounds")
        obj = objects[0]
        inverse = obj.matrix_world.inverted()
        for i in range(16):
            hit, local, _, _ = obj.ray_cast(inverse @ Vector((0, -(i + .5) * .25, 1)), Vector((0, 0, -1)))
            assert hit, (name, "missing tread", i)
            assert abs((obj.matrix_world @ local).z + i * .25) < .001, (name, "wrong tread height", i)
        checks += ["16 actual horizontal treads", "4 m run along Unity +Z", "4 m descent", "top stair origin", "1 m width"]
    elif name == "world_dungeon_entrance":
        assert_close([size[0], hi[1], lo[1]], [3, 3.5, 0], "entrance envelope")
        obj = objects[0]
        inv = obj.matrix_world.inverted()
        for x in (-.82, 0, .82):
            for z in (.05, 1.2, 2.55):
                hit, _, _, _ = obj.ray_cast(inv @ Vector((x, -2, z)), Vector((0, 1, 0)), distance=4)
                assert not hit, (name, "blocked entrance opening", x, z)
        checks += ["3 m width", "3.5 m height", "nine rays pass through the real opening"]
    elif name in ("world_dungeon_wall", "world_dungeon_pillar"):
        expected = [1, 2.5, .32] if name.endswith("wall") else [.35, 2.5, .35]
        assert_close(size, expected, name + " dimensions")
        assert abs(lo[1]) < .001
        checks += ["exact modular dimensions", "ground pivot"]
    elif name == "world_dungeon_floor":
        assert_close(lo, [-.5, -.08, -.5], name + " lower")
        assert_close(hi, [.5, 0, .5], name + " upper")
        checks += ["exact 1 m grid", "flat y=0 slab tops"]
    elif name == "world_cave_beetle":
        assert_close([size[0], size[2], lo[1]], [.9, 1.25, 0], name + " silhouette")
        checks += ["0.9 m width", "1.25 m length", "ground pivot"]
    return checks


def verify(manifest):
    from io_scene_fbx import parse_fbx
    original_scene = bpy.context.scene
    scene = bpy.data.scenes.new("FBX verification only")
    bpy.context.window.scene = scene
    checks = []
    for name, record in manifest["assets"].items():
        for obj in list(scene.objects):
            bpy.data.objects.remove(obj, do_unlink=True)
        path = EXPORT / (name + ".fbx")
        document, _ = parse_fbx.parse(str(path))
        settings = next(e for e in document.elems if e.id == b"GlobalSettings")
        properties = next(e for e in settings.elems if e.id == b"Properties70")
        axes = {e.props[0].decode(): e.props[-1] for e in properties.elems}
        assert axes["UpAxis"] == 1 and axes["UpAxisSign"] == 1, (name, "FBX Y up")
        assert axes["FrontAxis"] == 2 and axes["FrontAxisSign"] == 1, (name, "FBX -Z forward metadata", axes)
        assert abs(axes["UnitScaleFactor"] - 100) < .001, (name, "FBX meter units")
        bpy.ops.import_scene.fbx(filepath=str(path), use_anim=False)
        bpy.context.view_layer.update()
        objects = [o for o in scene.objects if o.type == "MESH"]
        assert len(objects) == 1, (name, "mesh count")
        obj = objects[0]
        verts = points(objects)
        assert all(math.isfinite(c) for p in verts for c in p), (name, "nonfinite vertex")
        triangles = sum(len(p.vertices) - 2 for p in obj.data.polygons)
        assert triangles == record["triangles"], (name, "triangles", triangles, record["triangles"])
        actual_bounds = bounds(verts)
        for a, b in zip(actual_bounds, record["bounds_blender"]):
            assert_close(a, b, name + " bounds")
        assert_close(list(obj.matrix_world.translation), [0, 0, 0], name + " mesh pivot")
        assert_close(list(obj.matrix_world.to_scale()), [1, 1, 1], name + " scale")
        roots = [o for o in scene.objects if o.parent is None]
        assert len(roots) == 1 and roots[0].type == "EMPTY", (name, "root hierarchy")
        assert_close(list(roots[0].matrix_world.translation), [0, 0, 0], name + " root pivot")
        actual_materials = {re.sub(r"\.\d{3}$", "", m.name) for m in obj.data.materials if m}
        assert actual_materials == set(record["materials"]), (name, "materials", actual_materials)
        assert all(p.material_index < len(obj.data.materials) for p in obj.data.polygons)
        for mat in obj.data.materials:
            key = re.sub(r"\.\d{3}$", "", mat.name).removeprefix("Forest_")
            color = forest.PALETTE[key]
            expected = [int(color[i:i + 2], 16) / 255 for i in (0, 2, 4)]
            assert_close(list(mat.diffuse_color[:3]), expected, name + " material color")
        contract_checks = check_contract(name, verts, record, objects)
        result = {"passed": True, "triangles": triangles, "mesh_count": 1,
                  "bounds_blender": actual_bounds, "bounds_unity": bounds([unity(p) for p in verts]),
                  "dimensions_unity_m": [round(b - a, 6) for a, b in zip(*bounds([unity(p) for p in verts]))],
                  "pivot_unity": list(unity(obj.matrix_world.translation)),
                  "scale": [round(c, 6) for c in obj.matrix_world.to_scale()],
                  "fbx_axes": {k: axes[k] for k in ("UpAxis", "UpAxisSign", "FrontAxis", "FrontAxisSign", "UnitScaleFactor")},
                  "contract_checks": contract_checks}
        record["verification"] = result
        checks.append(result)
    for obj in list(scene.objects):
        bpy.data.objects.remove(obj, do_unlink=True)
    bpy.context.window.scene = original_scene
    bpy.data.scenes.remove(scene)
    manifest["verification"] = {"passed": True, "asset_count": len(checks),
                                "method": "Actual FBX reimport in Blender; no Unity run",
                                "checks": ["FBX axes and units", "finite geometry", "triangle preservation",
                                           "one mesh per export", "source/reimport bounds", "origin pivots",
                                           "unit transforms", "palette materials", "asset geometry contracts"]}


def gallery():
    for i, (name, entry) in enumerate(ASSETS.items()):
        entry["root"].location = ((i % 5) * 4.6, (i // 5) * 6.1, 4 if name.endswith("stairs") else 0)
    coll = bpy.data.collections.new("Inspection only - never exported")
    bpy.context.scene.collection.children.link(coll)
    forest.CURRENT = coll
    forest.box("Gallery floor", (9.1, 9.2, -.16), (24, 26, .2), "soil", edge=0)
    bpy.ops.object.camera_add(location=(25, -30, 33))
    camera = bpy.context.object
    camera.name = "World kit management inspection"
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 31
    forest.aim(camera, (9, 8.7, 1))
    bpy.context.scene.camera = camera
    bpy.ops.object.light_add(type="SUN", location=(0, -5, 12))
    bpy.context.object.rotation_euler = (.3, -.5, -.4)
    bpy.context.object.data.energy = 2.3
    bpy.context.object.data.angle = .3
    bpy.ops.object.light_add(type="AREA", location=(6, -6, 15))
    bpy.context.object.data.energy = 1800
    bpy.context.object.data.shape = "DISK"
    bpy.context.object.data.size = 14
    forest.aim(bpy.context.object, (8, 8, 0))
    scene = bpy.context.scene
    scene.world.color = (.35, .4, .45)
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 24
    scene.cycles.use_denoising = True
    scene.render.resolution_x = 1800
    scene.render.resolution_y = 1500
    scene.render.resolution_percentage = 100
    scene.view_settings.view_transform = "AgX"
    bpy.ops.object.select_all(action="DESELECT")


def previews(directory):
    directory = Path(directory).resolve()
    assert directory.is_relative_to(Path("/tmp").resolve()), "Inspection renders belong in /tmp"
    directory.mkdir(parents=True, exist_ok=True)
    scene = bpy.context.scene
    scene.render.filepath = str(directory / "world-kit.png")
    bpy.ops.render.render(write_still=True)
    scene.camera.location = (8, -9, 11)
    scene.camera.data.ortho_scale = 7
    forest.aim(scene.camera, (4.6, 0, 0))
    scene.render.filepath = str(directory / "roads.png")
    bpy.ops.render.render(write_still=True)
    scene.camera.location = (3.7, .8, 3)
    scene.camera.data.ortho_scale = 4.8
    forest.aim(scene.camera, (0, 6.1, 1.7))
    scene.render.filepath = str(directory / "entrance.png")
    bpy.ops.render.render(write_still=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument("--preview-dir")
    options = parser.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.world = bpy.data.worlds.new("World kit sky")
    bpy.context.scene.unit_settings.system = "METRIC"
    bpy.context.scene.unit_settings.scale_length = 1
    if options.verify_only:
        manifest = json.loads(MANIFEST.read_text())
        verify(manifest)
    else:
        roads()
        dungeons()
        vegetation()
        beetle()
        manifest = {"generator": "source-art/build_world.py", "source": "source-art/world_kit.blend",
                    "blender": bpy.app.version_string, "seed": SEED, "unit": "meter",
                    "source_up": "+Z", "source_forward": "-Y", "fbx_forward": "-Z", "fbx_up": "+Y",
                    "unity_up": "+Y", "unity_forward": "+Z", "unity_import_scale": 1,
                    "consumer_naturalScale": True, "textures": 0,
                    "provenance": "Original project geometry; no downloaded assets, images, paid services or additional asset licenses",
                    "assets": {name: exporter(name, entry) for name, entry in ASSETS.items()}}
        verify(manifest)
        used = {m.removeprefix("Forest_") for e in manifest["assets"].values() for m in e["materials"]}
        manifest["palette_srgb_hex"] = {k: forest.PALETTE[k] for k in sorted(used)}
        manifest["totals"] = {"assets": len(ASSETS), "meshes": len(ASSETS),
                              "triangles": sum(e["triangles"] for e in manifest["assets"].values()),
                              "vertices": sum(e["vertices"] for e in manifest["assets"].values()),
                              "shared_materials": len(used),
                              "fbx_bytes": sum((EXPORT / (name + ".fbx")).stat().st_size for name in ASSETS)}
        gallery()
        bpy.context.preferences.filepaths.save_version = 0
        bpy.ops.wm.save_as_mainfile(filepath=str(ART / "world_kit.blend"), compress=True)
        MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n")
        if options.preview_dir:
            previews(options.preview_dir)
    print("WORLD_ART_VERIFIED " + json.dumps(manifest["totals"], sort_keys=True))


if __name__ == "__main__":
    main()
