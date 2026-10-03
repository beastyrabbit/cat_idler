"""Procedural construction progress assets for the forest world."""

from mathutils import Vector

from .helpers import asset, forest, math, prism, rng


def _beam(name, a, b, width, mat):
    middle = (Vector(a) + Vector(b)) * 0.5
    delta = Vector(b) - Vector(a)
    obj = forest.box(name, middle, (width, width, delta.length), mat, edge=0)
    obj.rotation_euler = delta.to_track_quat("Z", "Y").to_euler()
    return obj


def _pad(name, radius=0.82, depth=0.80, height=0.035):
    """A hand-cleared patch with a softly irregular, closed edge."""
    points = []
    for i in range(14):
        angle = math.tau * i / 14 + math.pi / 14
        radial = 1 + rng().uniform(-0.12, 0.12)
        angle += rng().uniform(-0.025, 0.025)
        points.append(
            (
                math.cos(angle) * radius * radial,
                math.sin(angle) * depth * (1 + rng().uniform(-0.10, 0.10)),
            )
        )
    prism(name, points, -0.018, -0.018 + height, "earth")


def _flag(x, y, z, height=1.15):
    forest.cylinder(
        "Flag pole", (x, y, z + height * 0.5), 0.018, height, "wood", 6, top=0.012
    )
    verts = [
        (x, y - 0.02, z + height - 0.08),
        (x, y - 0.02, z + height - 0.30),
        (x + 0.32, y - 0.02, z + height - 0.19),
        (x, y + 0.02, z + height - 0.08),
        (x, y + 0.02, z + height - 0.30),
        (x + 0.32, y + 0.02, z + height - 0.19),
    ]
    forest.mesh(
        "Berry marker pennant",
        verts,
        [(0, 1, 2), (5, 4, 3), (0, 3, 4, 1), (1, 4, 5, 2), (2, 5, 3, 0)],
        "berry",
    )


def _supply_stack(x, y, z=0.02, layers=3):
    """Neat parallel timber stack on two crosswise endgrain bearers."""
    for bx in (-0.20, 0.20):
        forest.box(
            "Endgrain stack bearer",
            (x + bx, y, z + 0.045),
            (0.085, 0.34, 0.09),
            "endgrain",
            edge=0.006,
        )
    for layer in range(layers):
        for row in (-1, 0, 1):
            forest.box(
                "Parallel supply plank",
                (x, y + row * 0.095, z + 0.105 + layer * 0.045),
                (0.55, 0.09, 0.03),
                "wood",
                edge=0.006,
            )
    # A small pyramid of six dressed stones beside the timber.
    stone_specs = (
        (0.00, 0.00, 0.070),
        (-0.07, 0.00, 0.070),
        (0.07, 0.00, 0.070),
        (-0.035, 0.055, 0.170),
        (0.035, 0.055, 0.170),
        (0.00, 0.105, 0.270),
    )
    for i, (sx, sy, sz) in enumerate(stone_specs):
        side = rng().uniform(0.10, 0.14)
        forest.box(
            "Supply stone block",
            (x + 0.37 + sx, y + 0.10 + sy, z + sz),
            (side, side * rng().uniform(0.78, 1.0), side),
            "stone_light",
            edge=0.008,
        )

def _foundation_course(edge=0.0):
    """Four courses of individually spaced, slightly irregular foundation stones."""
    centers = (-0.57, -0.19, 0.19, 0.57)
    for side in (-1, 1):
        for center in centers:
            length = rng().uniform(0.365, 0.37)
            height = rng().uniform(0.14, 0.18)
            forest.box(
                "Separated foundation block",
                (
                    center + rng().uniform(-0.012, 0.012),
                    side * (0.78 + rng().uniform(-0.012, 0.012)),
                    height * 0.5 + 0.004,
                ),
                (length, 0.16, height),
                "stone_light",
                edge=edge,
            )
        for center in centers:
            length = rng().uniform(0.365, 0.37)
            height = rng().uniform(0.14, 0.18)
            forest.box(
                "Separated foundation block",
                (
                    side * (0.78 + rng().uniform(-0.012, 0.012)),
                    center + rng().uniform(-0.012, 0.012),
                    height * 0.5 + 0.004,
                ),
                (0.16, length, height),
                "stone_light",
                edge=edge,
            )

def _contract(stage):
    return {
        "footprint_m": [1.84, 1.84],
        "pivot_unity": [0, 0, 0],
        "stage": stage,
        "naturalScale": True,
    }


def world_construction_site_0():
    asset(
        "world_construction_site_0",
        "Cleared irregular earth plot with stakes, berry marker and timber supplies",
        _contract(0),
    )
    _pad("Cleared earth pad", 0.82, 0.80, 0.035)
    for x, y in ((-0.79, -0.77), (0.79, -0.77), (-0.79, 0.77), (0.79, 0.77)):
        forest.box(
            "Chunky corner stake",
            (x, y, 0.21),
            (0.045, 0.045, 0.40),
            "wood",
            edge=0.008,
        )
    for a, b in (
        ((-0.79, -0.77, 0.27), (0.79, -0.77, 0.27)),
        ((0.79, -0.77, 0.27), (0.79, 0.77, 0.27)),
        ((0.79, 0.77, 0.27), (-0.79, 0.77, 0.27)),
        ((-0.79, 0.77, 0.27), (-0.79, -0.77, 0.27)),
    ):
        _beam("String outline", a, b, 0.012, "endgrain")
    for i in range(7):
        forest.box(
            "Raised earth clod",
            (rng().uniform(-0.56, 0.56), rng().uniform(-0.48, 0.48), rng().uniform(0.050, 0.070)),
            (rng().uniform(0.080, 0.125), rng().uniform(0.055, 0.095), rng().uniform(0.045, 0.070)),
            "earth",
            edge=0.006,
        )
    _supply_stack(-0.30, -0.46)
    _flag(-0.64, -0.47, 0.02, 1.18)


def world_construction_site_1():
    asset(
        "world_construction_site_1",
        "Continuous stone foundation and timber frame with a leaning scaffold ladder",
        _contract(1),
    )
    _pad("Earth pad inside foundation", 0.82, 0.80, 0.025)
    _foundation_course()
    for x, y in ((-0.72, -0.72), (0.72, -0.72), (-0.72, 0.72), (0.72, 0.72)):
        _beam("Tall corner post", (x, y, 0.18), (x, y, 1.53), 0.13, "wood")
    for y in (-0.78, 0.78):
        _beam("Foundation sill", (-0.70, y, 0.28), (0.70, y, 0.28), 0.08, "wood")
    for x in (-0.78, 0.78):
        _beam("Foundation sill", (x, -0.70, 0.28), (x, 0.70, 0.28), 0.08, "wood")
    for x in (-0.47, -0.17):
        _beam(
            "Scaffold ladder rail",
            (x, -0.89, 0.23),
            (x + 0.18, -0.89, 1.27),
            0.06,
            "wood",
        )
    for i in range(5):
        z = 0.37 + i * 0.19
        _beam(
            "Scaffold ladder rung",
            (-0.44 + i * 0.034, -0.89, z),
            (-0.20 + i * 0.034, -0.89, z),
            0.045,
            "endgrain",
        )
    _supply_stack(0.15, -0.42, 0.22, 1)
    _flag(-0.69, -0.69, 0.18, 1.26)


def world_construction_site_2():
    asset(
        "world_construction_site_2",
        "Framed cottage construction with rafters, scaffold and rope hoist",
        _contract(2),
    )
    _pad("Earth pad inside frame", 0.82, 0.80, 0.025)
    _foundation_course(edge=0.0)
    for x, y in ((-0.72, -0.72), (0.72, -0.72), (-0.72, 0.72), (0.72, 0.72)):
        _beam("Frame post", (x, y, 0.18), (x, y, 1.25), 0.12, "wood")
    ridge = 1.70
    for y in (-0.60, -0.20, 0.20, 0.60):
        _beam("Roof rafter", (0, y, ridge), (-0.78, y, 1.17), 0.075, "wood")
        _beam("Roof rafter", (0, y, ridge), (0.78, y, 1.17), 0.075, "wood")
    _beam("Ridge plate", (-0.82, 0, ridge), (0.82, 0, ridge), 0.11, "endgrain")
    for x in (-0.72, 0.72):
        _beam("Side wall plate", (x, -0.78, 1.22), (x, 0.78, 1.22), 0.09, "endgrain")
    for y in (-0.78, 0.78):
        _beam("Gable wall plate", (-0.72, y, 1.22), (0.72, y, 1.22), 0.09, "endgrain")
    for x, y in ((-0.42, 0.72), (0, 0.72), (0.42, 0.72)):
        _beam("Wall stud", (x, y, 0.25), (x, y, 1.20), 0.065, "wood")
    # Scaffold sits outside the right (+X) wall and remains readable from above.
    for y in (-0.58, 0.02, 0.62):
        _beam("Scaffold pole", (0.84, y, 0.18), (0.84, y, 1.38), 0.05, "wood")
    forest.box(
        "Scaffold walkway",
        (0.82, 0.02, 0.84),
        (0.16, 1.24, 0.08),
        "endgrain",
        edge=0.006,
    )
    for y in (-0.48, 0.10):
        forest.box(
            "Scaffold cross platform",
            (0.80, y, 0.84),
            (0.20, 0.16, 0.08),
            "endgrain",
            edge=0.006,
        )
    forest.box(
        "Scaffold front walkway plank",
        (0.81, -0.78, 0.84),
        (0.22, 0.14, 0.08),
        "endgrain",
        edge=0.006,
    )
    _beam("Scaffold cross brace", (0.84, -0.58, 0.27), (0.84, 0.02, 0.84), 0.035, "wood")
    _beam("Scaffold cross brace", (0.84, 0.02, 0.27), (0.84, 0.62, 0.84), 0.035, "wood")
    # The hoist is on the left (-X) gable, with a short outward jib and hanging load.
    _beam("Hoist jib", (-0.68, -0.48, 1.42), (-0.88, -0.48, 1.42), 0.055, "wood")
    _beam("Hoist rope", (-0.88, -0.48, 1.42), (-0.88, -0.48, 0.77), 0.014, "endgrain")
    for z in (0.70, 0.745, 0.79):
        forest.box(
            "Hoist plank bundle",
            (-0.88, -0.48, z),
            (0.07, 0.28, 0.032),
            "wood",
            edge=0.005,
        )
    _flag(-0.10, 0.00, 1.46, 0.28)


def _checker(name, uv, lo, hi, size, record, objects):
    contract = record.get("contract", {})
    footprint = contract.get("footprint_m", [1.84, 1.84])
    if size[0] > footprint[0] + 1e-4 or size[2] > footprint[1] + 1e-4:
        raise AssertionError((name, "footprint", size))
    if (
        max(abs(lo[0]), abs(hi[0])) > footprint[0] / 2 + 1e-4
        or max(abs(lo[2]), abs(hi[2])) > footprint[1] / 2 + 1e-4
    ):
        raise AssertionError((name, "centred footprint"))
    if not (-0.035 <= lo[1] <= 0.001):
        raise AssertionError((name, "ground contact", lo[1]))
    minima = {
        "world_construction_site_0": 0.3,
        "world_construction_site_1": 0.6,
        "world_construction_site_2": 1.2,
    }
    if not (minima.get(name, 0.3) <= size[1] <= 1.9):
        raise AssertionError((name, "height", size[1], minima.get(name, 0.3)))
    obj = objects[0]
    inv = obj.matrix_world.inverted()
    for poly in obj.data.polygons:
        n = (
            obj.matrix_world.to_3x3().inverted().transposed() @ poly.normal
        ).normalized()
        if abs(n.z) > 0.95:
            origin = obj.matrix_world @ poly.center + Vector((0, 0, 0.0005))
            direction = inv.to_3x3() @ Vector((0, 0, 1))
            hit, *_ = obj.ray_cast(inv @ origin, direction)
            if not hit and n.z < 0:
                raise AssertionError((name, "downward escaping face"))
    return ["footprint", "height", "ground contact", "upward-face rule"]


BUILDERS = {
    "world_construction_site_0": world_construction_site_0,
    "world_construction_site_1": world_construction_site_1,
    "world_construction_site_2": world_construction_site_2,
}

CONTRACT_CHECKERS = {
    "world_construction_site_": _checker,
}
