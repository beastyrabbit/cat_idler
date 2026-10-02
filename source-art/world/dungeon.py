"""World asset builders."""

from .helpers import (
    arch_stone,
    asset,
    forest,
    math,
    prism,
    voronoi_cells,
)


def world_dungeon_entrance():
    asset(
        "world_dungeon_entrance",
        "Open rock arch with warm timber posts and knee braces",
        {
            "width_m": 3,
            "height_m": 3.5,
            "opening_clear_min_width_m": 1.64,
            "opening_clear_height_m": 2.55,
            "front_unity": "+Z",
            "pivot_unity": [0, 0, 0],
            "naturalScale": True,
        },
    )
    for side in (-1, 1):
        for row in range(3):
            forest.box(
                "Arch pier stone",
                (side * 1.22, -0.02, row * 0.7 + 0.35),
                (0.56, 1.0, 0.7),
                "stone" if row % 2 else "stone_light",
                edge=0.045,
            )
        forest.box("Timber jamb", (side * 0.98, -0.565, 1.35), (0.2, 0.18, 2.7), "wood", edge=0.022)
        for z in (0.2, 1.5, 2.45):
            forest.box("Jamb iron strap", (side * 0.98, -0.662, z), (0.21, 0.024, 0.055), "iron", edge=0.005)
        forest.beam("Knee brace", (side * 1.3, -0.57, 2.03), (side * 0.89, -0.57, 2.71), 0.14, "bark")
    for i in range(8):
        arch_stone("Arch voussoir %02d" % i, i * math.pi / 8, (i + 1) * math.pi / 8)
    forest.box("Timber lintel", (0, -0.57, 2.78), (2.36, 0.24, 0.22), "wood", edge=0.025)
    for x in (-0.67, 0.15, 0.72):
        forest.orb("Arch moss", (x, -0.17, 3.26 if abs(x) > 0.5 else 3.41), (0.23, 0.29, 0.06), "pine", 1)


def world_dungeon_wall():
    asset(
        "world_dungeon_wall",
        "Five courses of rough stone with an open cutaway top",
        {
            "length_axis_unity": "X",
            "length_m": 1,
            "height_m": 2.5,
            "thickness_m": 0.32,
            "roof": False,
            "pivot_unity": [0, 0, 0],
            "naturalScale": True,
        },
    )
    forest.box("Recessed dark mortar", (0, 0, 1.25), (0.988, 0.255, 2.49), "slate", edge=0)
    for row in range(5):
        divisions = [-0.5, -0.17, 0.19, 0.5] if row % 2 == 0 else [-0.5, -0.32, 0.035, 0.37, 0.5]
        for col, (a, b) in enumerate(zip(divisions, divisions[1:], strict=False)):
            lo, hi = (a + (0.008 if col else 0), b - (0.008 if col < len(divisions) - 2 else 0))
            forest.box(
                "Wall course %d stone %d" % (row, col),
                ((lo + hi) / 2, 0, row * 0.5 + 0.25),
                (hi - lo, 0.32, 0.5 - (0.012 if row not in (0, 4) else 0)),
                "stone_light" if (row + col) % 4 == 0 else "stone",
                edge=0.025,
            )


def world_dungeon_pillar():
    asset(
        "world_dungeon_pillar",
        "Narrow stacked stone support with worn square feet",
        {"width_m": 0.35, "depth_m": 0.35, "height_m": 2.5, "pivot_unity": [0, 0, 0], "naturalScale": True},
    )
    for row in range(5):
        width = 0.35 if row in (0, 4) else 0.3 + row % 2 * 0.025
        forest.box(
            "Pillar course %d" % row,
            (0, 0, row * 0.5 + 0.25),
            (width, width, 0.5),
            "stone_light" if row in (0, 4) else "stone",
            edge=0.02,
        )


def world_dungeon_stairs():
    asset(
        "world_dungeon_stairs",
        "Sixteen solid stone treads descending four meters forward",
        {
            "width_m": 1,
            "run_m": 4,
            "descent_m": 4,
            "steps": 16,
            "tread_m": 0.25,
            "riser_m": 0.25,
            "direction_unity": "+Z",
            "top_unity": [0, 0, 0],
            "bottom_unity": [0, -4, 4],
            "pivot_unity": [0, 0, 0],
            "naturalScale": True,
        },
    )
    for i in range(16):
        a, b, top = (i * 0.25, (i + 1) * 0.25, -i * 0.25)
        ba, bb = (-0.25 - a * 0.9375, -0.25 - b * 0.9375)
        profile = [(-a, top), (-b, top), (-b, bb), (-a, ba)]
        verts = [(x, y, z) for x in (-0.5, 0.5) for y, z in profile]
        faces = [(3, 2, 1, 0), (4, 5, 6, 7), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7)]
        joints = (-0.18 + i % 2 * 0.07, 0.22 - i % 2 * 0.07)
        strips = [-0.5, joints[0] - 0.0045, joints[0] + 0.0045, joints[1] - 0.0045, joints[1] + 0.0045, 0.5]
        for left, right in zip(strips, strips[1:], strict=False):
            start = len(verts)
            verts += [(left, -a, top), (left, -b, top), (right, -b, top), (right, -a, top)]
            faces.append(tuple(range(start, start + 4)))
        obj = forest.mesh("Descending tread %02d" % (i + 1), verts, faces, "stone")
        obj.data.materials.append(forest.material("stone_light"))
        obj.data.materials.append(forest.material("slate"))
        for j, face in enumerate(obj.data.polygons[5:]):
            face.material_index = 1 if j % 2 == 0 else 2


def world_dungeon_floor():
    asset(
        "world_dungeon_floor",
        "Flat irregular stone slabs with dark recessed joints",
        {"grid_m": 1, "top_y_m": 0, "thickness_m": 0.08, "pivot_unity": [0, 0, 0], "naturalScale": True},
    )
    forest.box("Floor mortar bed", (0, 0, -0.054), (1, 1, 0.052), "slate", edge=0)
    for i, poly in enumerate(voronoi_cells()):
        prism("Floor slab %02d" % i, poly, -0.032, 0, "stone_light" if i % 4 == 0 else "stone")


BUILDERS = {
    "world_dungeon_entrance": world_dungeon_entrance,
    "world_dungeon_wall": world_dungeon_wall,
    "world_dungeon_pillar": world_dungeon_pillar,
    "world_dungeon_stairs": world_dungeon_stairs,
    "world_dungeon_floor": world_dungeon_floor,
}
