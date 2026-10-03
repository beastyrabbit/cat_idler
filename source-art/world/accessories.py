"""Small world-kit accessories that attach to articulated creature pivots."""

import math

from mathutils import Vector

from .helpers import asset, forest

# Measured by artifacts/demo/W2-26/measure_scarf_neck.py from the rest-pose
# cat.fbx: 16 rays in the plane at the upper-chest/head junction, with the
# eight evenly spaced measured points below retained for the deterministic mesh.
# The ray origin is chest=(0,-0.22,0.62), the head bbox centre is (0,-0.471,0.88),
# and the 44-degree axis points up and toward the cat's -Y nose.  Each point is
# the outer fur hit plus 0.6 * the 0.05 m tube radius, expressed at Head pivot.
SCARF_CENTERLINE = (
    (0.298829, -0.022950, -0.063000),
    (0.216294, 0.132663, 0.087226),
    (0.000000, 0.177099, 0.130124),
    (-0.216294, 0.132663, 0.087226),
    (-0.298829, -0.022950, -0.063000),
    (-0.195672, -0.163726, -0.198903),
    (0.000000, -0.199099, -0.233051),
    (0.195672, -0.163726, -0.198903),
)
SCARF_AXIS = Vector((0.0, -0.694545, 0.719449))
SCARF_TUBE_RADIUS = 0.05


def _tube(name, centreline, axis, radius):
    """Build a closed eight-sided knit tube around the measured neck path."""
    points = [Vector(point) for point in centreline]
    axis = Vector(axis).normalized()
    tube_segments = 8
    verts = []
    for index, point in enumerate(points):
        tangent = (points[(index + 1) % len(points)] - points[index - 1]).normalized()
        radial = tangent.cross(axis).normalized()
        for side in range(tube_segments):
            angle = math.tau * side / tube_segments
            offset = axis * math.cos(angle) + radial * math.sin(angle)
            verts.append(tuple(point + radius * offset))
    faces = []
    for index in range(len(points)):
        next_index = (index + 1) % len(points)
        for side in range(tube_segments):
            next_side = (side + 1) % tube_segments
            faces.append(
                (
                    index * tube_segments + side,
                    next_index * tube_segments + side,
                    next_index * tube_segments + next_side,
                    index * tube_segments + next_side,
                )
            )
    obj = forest.mesh(name, verts, faces, "berry")
    obj.data.materials.append(forest.material("cream"))
    # One narrow flush knit row keeps the berry wrap dominant from every side.
    for face_index, polygon in enumerate(obj.data.polygons):
        if face_index // tube_segments == 2 and face_index % tube_segments in (2, 3, 4, 5):
            polygon.material_index = 1
        polygon.use_smooth = True
    return obj


def _triangular_tail(name, a, b, width, depth, material):
    """Closed triangular-prism tail segment with no open fringe cards."""
    ax, ay, az = a
    bx, by, bz = b
    cross = ((-width, -depth * 0.5), (width, -depth * 0.5), (0.0, depth * 0.5))
    verts = [(ax + x, ay + y, az) for x, y in cross] + [(bx + x, by + y, bz) for x, y in cross]
    faces = [(0, 2, 1), (3, 4, 5), (0, 1, 4, 3), (1, 2, 5, 4), (2, 0, 3, 5)]
    return forest.mesh(name, verts, faces, material)


def _fringe(name, centre, material):
    """Small closed tetrahedral fringe end."""
    x, y, z = centre
    verts = [(x - 0.024, y - 0.018, z), (x + 0.024, y - 0.018, z), (x, y + 0.024, z), (x, y, z - 0.042)]
    faces = [(0, 2, 1), (0, 1, 3), (1, 2, 3), (2, 0, 3)]
    return forest.mesh(name, verts, faces, material)


def _knot(name, centre):
    """Chunky closed octahedral knot tucked under the chin."""
    x, y, z = centre
    verts = [
        (x, y, z + 0.085),
        (x, y, z - 0.085),
        (x - 0.09, y, z),
        (x + 0.09, y, z),
        (x, y - 0.07, z),
        (x, y + 0.07, z),
    ]
    faces = [
        (0, 2, 4), (0, 4, 3), (0, 3, 5), (0, 5, 2),
        (1, 4, 2), (1, 3, 4), (1, 5, 3), (1, 2, 5),
    ]
    return forest.mesh(name, verts, faces, "berry")


def world_elder_scarf():
    """Berry knit scarf authored at the adult cat Head pivot in source units."""
    description = "Berry elder scarf measured snug to the neck, with cream band, knot and two tails"
    asset(
        "world_elder_scarf",
        description,
        {
            "attach_to": "cat/Body/Head",
            "local_pose": "identity",
            "units": "adult cat source units",
            "naturalScale": True,
            "pivot": [0, 0, 0],
        },
    )
    _tube("Knit neck wrap", SCARF_CENTERLINE, SCARF_AXIS, SCARF_TUBE_RADIUS)

    # Front is negative local Y.  The knot is offset slightly to the cat's
    # left, below the measured front of the collar and clear of both forelegs.
    _knot("Scarf knot", (-0.045, -0.355, -0.245))
    tails = (
        ("Left", (-0.055, -0.355, -0.300), (-0.045, -0.380, -0.555)),
        ("Right", (0.025, -0.355, -0.295), (0.070, -0.375, -0.485)),
    )
    for label, start, end in tails:
        band = tuple(start[index] * 0.48 + end[index] * 0.52 for index in range(3))
        _triangular_tail(f"{label} scarf tail berry", start, band, 0.042, 0.052, "berry")
        _triangular_tail(f"{label} scarf tail cream stripe", band, end, 0.042, 0.052, "cream")
        _fringe(f"{label} fringe", end, "cream")


BUILDERS = {"world_elder_scarf": world_elder_scarf}
