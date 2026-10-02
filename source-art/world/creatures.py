"""World asset builders."""

from .helpers import (
    asset,
    bounds,
    forest,
    points,
)


def world_cave_beetle():
    asset(
        "world_cave_beetle",
        "Six-legged cave enemy with split armored shell, amber eyes and hooked jaws",
        {
            "width_m": 0.9,
            "length_m": 1.25,
            "forward_unity": "+Z",
            "rig": "static",
            "pivot_unity": [0, 0, 0],
            "naturalScale": True,
        },
    )
    forest.orb("Dark abdomen", (0, 0.12, 0.24), (0.28, 0.37, 0.22), "coal", 2)
    for side in (-1, 1):
        forest.orb(
            "Split shell left" if side < 0 else "Split shell right",
            (side * 0.126, 0.16, 0.32),
            (0.142, 0.33, 0.2),
            "teal",
            2,
        )
        for j in range(3):
            forest.orb(
                "Shell copper ridge",
                (side * (0.13 + j * 0.022), 0.0 + j * 0.135, 0.465 - j * 0.007),
                (0.074, 0.03, 0.022),
                "copper",
                1,
            )
        for j, y in enumerate((-0.15, 0.1, 0.34)):
            hip = (side * 0.2, y, 0.25)
            knee = (side * 0.37, y + (0.12 if j == 2 else -0.04), 0.17)
            foot = (side * 0.44, y + (0.21 if j == 2 else -0.16), 0.025)
            forest.beam("Leg femur", hip, knee, 0.065, "slate")
            forest.beam("Leg tibia", knee, foot, 0.036, "coal")
        forest.orb("Amber eye", (side * 0.126, -0.403, 0.32), (0.055, 0.039, 0.055), "ember", 1)
        forest.beam("Jaw root", (side * 0.12, -0.42, 0.17), (side * 0.18, -0.57, 0.12), 0.07, "copper")
        forest.beam("Hooked jaw", (side * 0.18, -0.57, 0.12), (side * 0.06, -0.65, 0.13), 0.04, "copper")
        forest.beam("Antenna", (side * 0.1, -0.4, 0.37), (side * 0.25, -0.6, 0.42), 0.022, "bark")
    forest.orb("Armored thorax", (0, -0.195, 0.29), (0.255, 0.19, 0.17), "slate", 1)
    forest.orb("Beetle head", (0, -0.365, 0.24), (0.19, 0.18, 0.145), "coal", 2)
    objects = list(forest.CURRENT.objects)
    lo, hi = bounds(points(objects))
    for obj in objects:
        inverse = obj.matrix_world.inverted()
        for v in obj.data.vertices:
            p = obj.matrix_world @ v.co
            p.x = (p.x - (lo[0] + hi[0]) / 2) * 0.9 / (hi[0] - lo[0])
            p.y = (p.y - (lo[1] + hi[1]) / 2) * 1.25 / (hi[1] - lo[1])
            p.z -= lo[2]
            v.co = inverse @ p


BUILDERS = {
    "world_cave_beetle": world_cave_beetle,
}
