"""World-only geometry helpers and asset registration."""

import json
import math
from pathlib import Path

import bpy
from kit import helpers as forest

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "source-art"
SEED = 161803
ASSETS = {}
RNG = None
WORLD_OFFSETS = json.loads((Path(__file__).with_name("rng_offsets.json")).read_text())
CANONICAL_ORDER = [
    "world_road_straight",
    "world_road_corner",
    "world_road_junction",
    "world_road_cross",
    "world_road_end",
    "world_dungeon_entrance",
    "world_dungeon_wall",
    "world_dungeon_pillar",
    "world_dungeon_stairs",
    "world_dungeon_floor",
    "world_tree_pine",
    "world_tree_birch",
    "world_reeds",
    "world_mushrooms",
    "world_crystals",
    "world_cave_beetle",
]
PORTS = {
    "straight": ["+Z", "-Z"],
    "corner": ["+Z", "+X"],
    "junction": ["-X", "+X", "+Z"],
    "cross": ["-X", "+X", "+Z", "-Z"],
    "end": ["+Z"],
}
REGIONS = {
    "+Z": (-0.34, -0.5, 0.34, -0.34),
    "-Z": (-0.34, 0.34, 0.34, 0.5),
    "+X": (0.34, -0.34, 0.5, 0.34),
    "-X": (-0.5, -0.34, -0.34, 0.34),
}


def rng():
    """Return the current world asset stream."""
    return RNG


def asset(name, description, contract):
    """Register an asset and reset its isolated deterministic random stream."""
    if not name.startswith("world_"):
        raise ValueError(f"world asset must start with world_: {name}")
    global RNG
    RNG = forest.seeded_stream(SEED, name, WORLD_OFFSETS)
    forest.new_asset(name, description)
    ASSETS[name] = {"collection": forest.CURRENT, "description": description, "contract": contract}


def points(objects):
    bpy.context.view_layer.update()
    return [obj.matrix_world @ v.co for obj in objects for v in obj.data.vertices]


def bounds(verts):
    return [[round(fn((v[i] for v in verts)), 6) for i in range(3)] for fn in (min, max)]


def unity(v):
    return (v[0], v[2], -v[1])


def clip(poly, axis, value, keep_less):
    """Clip a convex polygon to a half plane without external geometry packages."""
    result = []
    if not poly:
        return result
    for a, b in zip(poly, poly[1:] + poly[:1], strict=True):
        da = (a[axis] - value) * (1 if keep_less else -1)
        db = (b[axis] - value) * (1 if keep_less else -1)
        if da <= 1e-09:
            result.append(a)
        if da < 0 < db or db < 0 < da:
            t = da / (da - db)
            result.append(tuple((a[i] + t * (b[i] - a[i]) for i in range(2))))
    return result


def clip_rect(poly, rect):
    x0, y0, x1, y1 = rect
    for axis, value, less in ((0, x0, False), (0, x1, True), (1, y0, False), (1, y1, True)):
        poly = clip(poly, axis, value, less)
    return poly


def area(poly):
    return abs(sum((a[0] * b[1] - b[0] * a[1] for a, b in zip(poly, poly[1:] + poly[:1], strict=True)))) / 2


def prism(name, poly, bottom, top, mat):
    n = len(poly)
    verts = [(x, y, z) for z in (bottom, top) for x, y in poly]
    faces = [tuple(reversed(range(n))), tuple(range(n, 2 * n))]
    faces += [(i, (i + 1) % n, (i + 1) % n + n, i + n) for i in range(n)]
    return forest.mesh(name, verts, faces, mat)


def voronoi_cells():
    sites = [
        (x * 0.22 + rng().uniform(-0.048, 0.048), y * 0.22 + rng().uniform(-0.048, 0.048))
        for y in range(-2, 3)
        for x in range(-2, 3)
    ]
    for sx, sy in sites:
        poly = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)]
        for tx, ty in sites:
            if (sx, sy) == (tx, ty):
                continue
            nx, ny = (tx - sx, ty - sy)
            c = (tx * tx + ty * ty - sx * sx - sy * sy) / 2
            result = []
            for a, b in zip(poly, poly[1:] + poly[:1], strict=True):
                da, db = (nx * a[0] + ny * a[1] - c, nx * b[0] + ny * b[1] - c)
                if da <= 1e-09:
                    result.append(a)
                if da < 0 < db or db < 0 < da:
                    t = da / (da - db)
                    result.append((a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])))
            poly = result
        yield [
            (
                x if abs(abs(x) - 0.5) < 1e-07 else sx + (x - sx) * 0.93,
                y if abs(abs(y) - 0.5) < 1e-07 else sy + (y - sy) * 0.93,
            )
            for x, y in poly
        ]


def grass(name, pos, height=0.07, mat="leaf"):
    x, y, z = pos
    for i in range(3):
        angle = i * 2.094 + 0.3
        dx, dy = (math.cos(angle), math.sin(angle))
        forest.mesh(
            name,
            [
                (x - dy * 0.016, y + dx * 0.016, z),
                (x + dy * 0.016, y - dx * 0.016, z),
                (x + dx * height * 0.48, y + dy * height * 0.48, z + height),
            ],
            [(0, 1, 2)],
            mat,
        )


def arch_stone(name, t0, t1):
    profile = [
        (math.cos(t0) * 0.94, 2.1 + math.sin(t0) * 1.1),
        (math.cos(t0) * 1.5, 2.1 + math.sin(t0) * 1.4),
        (math.cos((t0 + t1) / 2) * 1.5, 2.1 + math.sin((t0 + t1) / 2) * 1.4),
        (math.cos(t1) * 1.5, 2.1 + math.sin(t1) * 1.4),
        (math.cos(t1) * 0.94, 2.1 + math.sin(t1) * 1.1),
    ]
    n = len(profile)
    verts = [(x, depth, z) for depth in (-0.52, 0.48) for x, z in profile]
    faces = [tuple(reversed(range(n))), tuple(range(n, 2 * n))]
    faces += [(i, (i + 1) % n, (i + 1) % n + n, i + n) for i in range(n)]
    return forest.mesh(name, verts, faces, "stone" if int(t0 * 12) % 3 else "stone_light")


def crown(name, center, radius, height, mat, phase):
    x, y, z = center
    count = 11
    verts = []
    for _layer, (scale, lift) in enumerate(((1, 0), (0.68, height * 0.38))):
        for i in range(count):
            angle = phase + math.tau * i / count
            r = radius * scale * (1 + 0.11 * math.sin(i * 5.3 + phase))
            verts.append((x + math.cos(angle) * r, y + math.sin(angle) * r, z + lift + 0.1 * math.sin(i * 4.1 + phase)))
    verts += [(x + 0.07, y + 0.03, z + height), (x, y, z + 0.13)]
    faces = []
    for i in range(count):
        j = (i + 1) % count
        faces += [(i, j, count + j, count + i), (count + i, count + j, 2 * count), (j, i, 2 * count + 1)]
    obj = forest.mesh(name, verts, faces, mat)
    obj.data.materials.append(forest.material("leaf"))
    for i, face in enumerate(obj.data.polygons):
        if i % 13 == 1:
            face.material_index = 1


__all__ = [
    "ASSETS",
    "CANONICAL_ORDER",
    "REGIONS",
    "PORTS",
    "rng",
    "area",
    "arch_stone",
    "asset",
    "bounds",
    "clip",
    "clip_rect",
    "crown",
    "forest",
    "grass",
    "points",
    "prism",
    "unity",
    "voronoi_cells",
]
