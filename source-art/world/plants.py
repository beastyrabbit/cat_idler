"""World asset builders."""

from .helpers import (
    asset,
    crown,
    forest,
    math,
)


def world_tree_pine():
    asset(
        "world_tree_pine",
        "Layered ragged pine canopy, visible branches and flared roots",
        {"biomes": ["pine_forest", "highland"], "pivot_unity": [0, 0, 0], "naturalScale": True},
    )
    forest.cylinder("Pine trunk", (0, 0, 2), 0.15, 4, "bark", 9, top=0.055)
    for i in range(5):
        angle = i * math.tau / 5
        forest.beam("Pine root", (math.cos(angle) * 0.34, math.sin(angle) * 0.34, 0.025), (0, 0, 0.43), 0.12, "bark")
    for layer, (z, radius, h) in enumerate(
        ((1.4, 1.03, 1.65), (2.12, 0.87, 1.6), (2.9, 0.62, 1.38), (3.56, 0.39, 1.18))
    ):
        for i in range(3):
            a = i * math.tau / 3 + layer * 0.7
            forest.beam(
                "Pine bough",
                (0, 0, z + 0.35),
                (math.cos(a) * radius * 0.83, math.sin(a) * radius * 0.83, z + 0.12),
                0.065,
                "bark",
            )
        crown("Needle tier %d" % layer, (0, 0, z), radius, h, "pine", layer * 0.42)


def world_tree_birch():
    asset(
        "world_tree_birch",
        "Pale forked birch with dark bark marks and light leaf clusters",
        {"biomes": ["birch_grove", "meadow_edge"], "pivot_unity": [0, 0, 0], "naturalScale": True},
    )
    forest.cylinder("Birch trunk", (0, 0, 1.55), 0.125, 3.1, "cream", 9, top=0.063)
    for i in range(12):
        angle, z = (i * 2.39, 0.22 + i * 0.22)
        radius = 0.125 - z * 0.02
        verts = [
            (math.cos(angle + da) * (radius + 0.003), math.sin(angle + da) * (radius + 0.003), z + dz)
            for da, dz in ((-0.45, -0.026), (0.47, -0.008), (0.3, 0.035), (-0.36, 0.015))
        ]
        forest.mesh("Birch bark dash", verts, [(0, 1, 2, 3)], "bark")
    clusters = [
        (-0.62, 0.03, 2.7, 0.65),
        (0.65, 0.12, 2.93, 0.71),
        (-0.37, 0.36, 3.4, 0.74),
        (0.38, -0.31, 3.6, 0.63),
        (-0.02, 0.08, 3.99, 0.5),
    ]
    for i, (x, y, z, r) in enumerate(clusters):
        forest.beam("Pale fork", (0, 0, 1.4 + i * 0.19), (x, y, z), 0.075, "cream")
        forest.orb("Birch leaf cluster", (x, y, z), (r, r * 0.77, r * 0.82), "leaf_light" if i % 2 else "leaf", 2)
        forest.orb("Birch sunlit tips", (x - 0.1, y - 0.1, z + 0.27), (r * 0.62, r * 0.55, r * 0.46), "moss", 1)


def world_reeds():
    asset(
        "world_reeds",
        "Loose wetland reeds with bent blades and brown seed heads",
        {"biomes": ["marsh", "riverbank"], "pivot_unity": [0, 0, 0], "naturalScale": True},
    )
    for i in range(10):
        a = i * 2.4
        x, y = (math.cos(a) * (0.08 + i * 0.025), math.sin(a) * (0.08 + i * 0.025))
        h = 0.55 + i % 4 * 0.16
        forest.cylinder("Reed stem", (x, y, h / 2), 0.014, h, "leaf", 5, top=0.009)
        if i % 2 == 0:
            forest.cylinder("Reed seed head", (x, y, h + 0.07), 0.036, 0.17, "bark", 6, top=0.025)
        for side in (-1, 1):
            dx, dy = (math.cos(a) * side, math.sin(a) * side)
            forest.mesh(
                "Bent reed blade",
                [
                    (x, y, 0.04),
                    (x - dy * 0.028, y + dx * 0.028, h * 0.5),
                    (x + dx * 0.19, y + dy * 0.19, h * 0.83),
                    (x + dx * 0.29, y + dy * 0.29, h * 0.68),
                    (x + dy * 0.028, y - dx * 0.028, h * 0.45),
                ],
                [(0, 1, 2, 3, 4), (4, 3, 2, 1, 0)],
                "leaf_light" if i % 3 else "herb",
            )


def world_mushrooms():
    asset(
        "world_mushrooms",
        "Five broad russet mushrooms with pale stems and cap spots",
        {"biomes": ["damp_forest", "dungeon"], "pivot_unity": [0, 0, 0], "naturalScale": True},
    )
    for i, (x, y, h, r) in enumerate(
        (
            (0, 0, 0.4, 0.21),
            (0.29, 0.1, 0.26, 0.14),
            (-0.24, 0.13, 0.28, 0.16),
            (-0.15, -0.22, 0.19, 0.115),
            (0.21, -0.19, 0.15, 0.1),
        )
    ):
        forest.cylinder("Mushroom stem", (x, y, h / 2), r * 0.23, h, "cream", 7, top=r * 0.17)
        forest.cylinder("Pale cap underside", (x, y, h - 0.015), r, 0.04, "cream", 10, top=r * 0.9)
        forest.cylinder(
            "Russet cap", (x, y, h + r * 0.25), r, r * 0.5, "terracotta" if i % 2 else "berry", 10, top=r * 0.2
        )
        for j in range(3):
            a = j * math.tau / 3 + i
            forest.orb(
                "Cream cap spot",
                (x + math.cos(a) * r * 0.43, y + math.sin(a) * r * 0.43, h + r * 0.37),
                (r * 0.115, r * 0.115, r * 0.038),
                "cream",
                1,
            )


def world_crystals():
    asset(
        "world_crystals",
        "Faceted teal crystals growing out of a low slate cluster",
        {"biomes": ["dungeon", "highland"], "pivot_unity": [0, 0, 0], "naturalScale": True},
    )
    forest.orb("Crystal slate base", (0, 0, 0.06), (0.44, 0.32, 0.11), "slate", 1)
    for i, (x, y, radius, h) in enumerate(
        (
            (0, 0, 0.15, 0.94),
            (0.23, 0.04, 0.11, 0.61),
            (-0.23, 0.06, 0.1, 0.51),
            (0.02, -0.22, 0.095, 0.44),
            (-0.08, 0.2, 0.09, 0.59),
        )
    ):
        verts = [
            (x + math.cos(j * math.tau / 6 + 0.2) * radius, y + math.sin(j * math.tau / 6 + 0.2) * radius, z)
            for z in (0.04, h * 0.74)
            for j in range(6)
        ]
        verts += [(x + 0.035, y - 0.035, h)]
        faces = [tuple(reversed(range(6)))]
        faces += [(j, (j + 1) % 6, (j + 1) % 6 + 6, j + 6) for j in range(6)]
        faces += [(j + 6, (j + 1) % 6 + 6, 12) for j in range(6)]
        obj = forest.mesh("Quartz point %d" % i, verts, faces, "gem")
        obj.data.materials.append(forest.material("teal"))
        for j, face in enumerate(obj.data.polygons):
            if j % 3 == 1:
                face.material_index = 1


BUILDERS = {
    "world_tree_pine": world_tree_pine,
    "world_tree_birch": world_tree_birch,
    "world_reeds": world_reeds,
    "world_mushrooms": world_mushrooms,
    "world_crystals": world_crystals,
}
