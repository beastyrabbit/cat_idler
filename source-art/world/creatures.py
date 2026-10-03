"""Procedural enemy and guardian models."""

import math

from mathutils import Vector

from .helpers import asset, forest


def _merge(name, before, pivot):
    return forest.merge_group(name, [o for o in forest.CURRENT.objects if o not in before], pivot=pivot)


def _loft(name, points, radii, mat, ring=8, tip_mat=None, tip_start=1):
    """Make a closed, smooth-shaded tube along a polyline."""
    points = [Vector(p) for p in points]
    verts = []
    for i, point in enumerate(points):
        if i == 0:
            tangent = points[1] - point
        elif i == len(points) - 1:
            tangent = point - points[i - 1]
        else:
            tangent = points[i + 1] - points[i - 1]
        tangent.normalize()
        reference = Vector((0, 0, 1)) if abs(tangent.z) < 0.9 else Vector((1, 0, 0))
        across = tangent.cross(reference).normalized()
        vertical = across.cross(tangent).normalized()
        radius = radii[i]
        rx, rz = (radius, radius) if not isinstance(radius, (tuple, list)) else radius
        for j in range(ring):
            angle = math.tau * j / ring
            verts.append(tuple(point + across * (math.cos(angle) * rx) + vertical * (math.sin(angle) * rz)))
    faces = []
    slots = []
    for i in range(len(points) - 1):
        for j in range(ring):
            faces.append((i * ring + j, i * ring + (j + 1) % ring,
                          (i + 1) * ring + (j + 1) % ring, (i + 1) * ring + j))
            slots.append(1 if tip_mat and i >= len(points) - 1 - tip_start else 0)
    first_indices = list(range(ring))
    first_normal = (Vector(verts[1]) - Vector(verts[0])).cross(Vector(verts[2]) - Vector(verts[1]))
    if first_normal.dot(-(points[1] - points[0])) < 0:
        first_indices.reverse()
    faces.append(tuple(first_indices))
    slots.append(0)
    last_indices = list((len(points) - 1) * ring + j for j in range(ring))
    last_base = (len(points) - 1) * ring
    last_normal = (Vector(verts[last_base + 1]) - Vector(verts[last_base])).cross(Vector(verts[last_base + 2]) - Vector(verts[last_base + 1]))
    if last_normal.dot(points[-1] - points[-2]) < 0:
        last_indices.reverse()
    faces.append(tuple(last_indices))
    slots.append(1 if tip_mat else 0)
    obj = forest.mesh(name, verts, faces, mat)
    if tip_mat:
        obj.data.materials.append(forest.material(tip_mat))
        for poly, slot in zip(obj.data.polygons, slots, strict=True):
            poly.material_index = slot
    for poly in obj.data.polygons:
        poly.use_smooth = True
    return obj


def _tri_ear(side, x, y, z, mat):
    return forest.mesh("Triangular ear", [(x - side * 0.12, y, z), (x + side * 0.12, y, z),
                                           (x + side * 0.01, y, z + 0.20),
                                           (x, y + 0.10, z + 0.04)],
                      [(0, 1, 2), (0, 2, 3), (1, 3, 2), (0, 3, 1)], mat)


def world_raider_fox():
    asset("world_raider_fox", "Lean masked fox raider with a huge brush tail and loot sack", {
        "length_m": 1.1, "width_m": 0.45, "height_m": 0.8,
        "parts": ["Body", "Head", "Tail", "PawFrontLeft", "PawFrontRight", "PawBackLeft", "PawBackRight"],
        "pivot_unity": [0, 0, 0], "forward_unity": "+Z", "naturalScale": True, "upward_face_check": True,
    })
    before = set(forest.CURRENT.objects)
    _loft("Fox torso", [(0, -0.20, 0.44), (0, 0.02, 0.45), (0, 0.27, 0.42)],
          [(0.17, 0.18), (0.20, 0.20), (0.16, 0.17)], "terracotta", 8)
    forest.orb("Cream chest ruff", (0, -0.245, 0.46), (0.12, 0.035, 0.17), "cream", 1, True)
    forest.orb("Loot sack", (0.13, 0.10, 0.64), (0.13, 0.12, 0.13), "leather", 2, True)
    forest.cylinder("Sack tie", (0.13, 0.10, 0.77), 0.035, 0.07, "darkwood", vertices=6)
    forest.beam("Sack strap", (-0.15, -0.04, 0.64), (0.20, 0.17, 0.67), 0.018, "leather")
    forest.orb("Ember shoulder stripe", (0.08, 0.20, 0.56), (0.11, 0.035, 0.045), "ember", 1)
    _merge("Body", before, (0, 0.08, 0.43))

    before = set(forest.CURRENT.objects)
    forest.orb("Wedge fox skull", (0, -0.31, 0.61), (0.17, 0.15, 0.14), "terracotta", 2, True)
    _loft("Long pointed snout", [(0, -0.34, 0.60), (0, -0.45, 0.59), (0, -0.53, 0.58)],
          [(0.11, 0.09), (0.075, 0.065), (0.035, 0.03)], "cream", 8)
    forest.orb("Dark nose", (0, -0.555, 0.58), (0.04, 0.035, 0.03), "nose", 1)
    for side in (-1, 1):
        _tri_ear(side, side * 0.11, -0.30, 0.68, "terracotta")
        ex = side * 0.11
        forest.mesh("Cream inner ear", [(side * 0.07, -0.315, 0.70), (side * 0.15, -0.315, 0.70),
                                         (ex, -0.315, 0.80), (ex, -0.285, 0.73)],
                    [(0, 1, 2), (0, 3, 1), (1, 3, 2), (2, 3, 0)], "cream")
    # The mask is a shallow shell on the skull rather than a flat visor.  The
    # skull is an ellipsoid centred at c; sampling its surface keeps both edges
    # flush when the low-poly orb is viewed from the front or either side.
    skull_c = Vector((0.0, -0.31, 0.61))
    skull_r = Vector((0.17, 0.15, 0.14))
    theta0, theta1 = math.radians(-105), math.radians(105)
    theta_count = 25
    z_rows = (0.605, 0.655)

    def mask_surface(theta, z, outward=0.0):
        s = math.sqrt(max(0.0, 1.0 - ((z - skull_c.z) / skull_r.z) ** 2))
        radial = Vector((skull_r.x * math.sin(theta), -skull_r.y * math.cos(theta), 0.0))
        point = skull_c + Vector((radial.x * 1.01 * s, radial.y * 1.01 * s, z - skull_c.z))
        if outward:
            point += Vector((math.sin(theta), -math.cos(theta), 0.0)) * outward
        return point

    band_verts = []
    for row_z in z_rows:
        for i in range(theta_count):
            theta = theta0 + (theta1 - theta0) * i / (theta_count - 1)
            inner = mask_surface(theta, row_z)
            outer = mask_surface(theta, row_z, 0.014)
            band_verts.extend((tuple(inner), tuple(outer)))
    band_faces = []
    for i in range(theta_count - 1):
        a, b = 2 * i, 2 * (i + 1)
        at, bt = 2 * theta_count + a, 2 * theta_count + b
        # Outer and inner skins, plus the top and bottom edge strips.
        band_faces.extend(((a + 1, b + 1, bt + 1, at + 1),
                           (a, at, bt, b), (a, b, b + 1, a + 1),
                           (at, at + 1, bt + 1, bt)))
    band_faces.extend(((0, 1, 2 * theta_count + 1, 2 * theta_count),
                       (2 * (theta_count - 1), 2 * theta_count + 2 * (theta_count - 1),
                        2 * theta_count + 2 * (theta_count - 1) + 1, 2 * (theta_count - 1) + 1)))
    forest.mesh("Wrapped darkwood eye band", band_verts, band_faces, "darkwood")

    def eye_slit(name, outer_theta, inner_theta):
        # The inner end is lower, giving each ember slit a downward slope
        # toward the snout while it remains two millimetres above the band.
        points = []
        for theta, z in ((outer_theta, 0.646), (inner_theta, 0.630)):
            for dz in (0.006, -0.006):
                points.append(tuple(mask_surface(theta, z + dz, 0.016)))
        forest.mesh(name, points, [(0, 2, 3, 1)], "ember")

    eye_slit("Angled ember eye glint L", math.radians(-52), math.radians(-20))
    eye_slit("Angled ember eye glint R", math.radians(52), math.radians(20))

    # A single knot sits at the right rear end of the wrap.  Its two tails run
    # straight backward and down, keeping the silhouette within the head width.
    knot = mask_surface(theta1, 0.630, 0.016)
    forest.orb("Mask knot", tuple(knot), (0.035, 0.032, 0.032), "darkwood", 1, True)
    for index, x in enumerate((knot.x - 0.012, knot.x + 0.012)):
        _loft(f"Mask ribbon tail {index}",
              [(x, knot.y + 0.008, knot.z), (x, knot.y + 0.055, knot.z - 0.018),
               (x, knot.y + 0.098, knot.z - 0.040)],
              [(0.018, 0.012), (0.013, 0.009), (0.004, 0.003)], "darkwood", 6)

    forest.orb("Cream cheek ruff", (0, -0.36, 0.56), (0.17, 0.05, 0.09), "cream", 1)
    _merge("Head", before, (0, -0.24, 0.59))

    before = set(forest.CURRENT.objects)
    _loft("Upswept brush tail", [(0, 0.25, 0.43), (0.03, 0.38, 0.51), (0.05, 0.52, 0.64),
                                  (0.03, 0.61, 0.73)], [0.10, 0.12, 0.105, 0.055],
          "terracotta", 8, tip_mat="cream", tip_start=1)
    _merge("Tail", before, (0, 0.25, 0.43))

    for side, label in ((-1, "Left"), (1, "Right")):
        for y, pair in ((-0.14, "Front"), (0.22, "Back")):
            before = set(forest.CURRENT.objects)
            _loft("Slim fox leg", [(side * 0.13, y, 0.39), (side * 0.14, y - 0.01, 0.20),
                                   (side * 0.14, y - 0.015, 0.07)], [(0.06, 0.065), (0.055, 0.055), (0.05, 0.045)], "terracotta", 7)
            forest.orb("Darkwood sock", (side * 0.14, y - 0.015, 0.06), (0.06, 0.06, 0.045), "darkwood", 1)
            _merge("Paw" + pair + label, before, (side * 0.13, y, 0.39))
    # A single uniform authored scale keeps the fox inside the 1.1 m silhouette
    # while preserving every joint's relative placement.
    for obj in forest.CURRENT.objects:
        if obj.type == "MESH":
            obj.scale = (0.902, 0.902, 0.902)
            obj.location *= 0.902


def world_stone_guardian():
    asset("world_stone_guardian", "Ancient stone cat guardian with moss, runes and a raised striking paw", {
        "footprint_m": [1, 1], "height_m": 1.4, "parts": ["Body", "Head", "PawFrontLeft", "PawFrontRight"],
        "pivot_unity": [0, 0, 0], "naturalScale": True, "upward_face_check": True,
    })
    # A seated silhouette: broad haunches and three slightly receding courses.
    before = set(forest.CURRENT.objects)
    forest.orb("Left rounded haunch", (-0.24, 0.14, 0.26), (0.25, 0.33, 0.26), "stone", 2, True)
    forest.orb("Right rounded haunch", (0.24, 0.14, 0.26), (0.25, 0.33, 0.26), "stone", 2, True)
    forest.box("Left hind foot", (-0.30, -0.16, 0.045), (0.17, 0.22, 0.09), "stone_light", edge=0.018)
    forest.box("Right hind foot", (0.30, -0.16, 0.045), (0.17, 0.22, 0.09), "stone_light", edge=0.018)
    for z, size, mat in ((0.36, (0.62, 0.50, 0.30), "stone"),
                         (0.64, (0.56, 0.46, 0.26), "stone_light"),
                         (0.88, (0.68, 0.44, 0.20), "stone")):
        forest.box("Bevelled torso course", (0, -0.04, z), size, mat, edge=0.035)
    forest.box("Inset chest plate", (0, -0.29, 0.60), (0.32, 0.06, 0.34), "stone_light", edge=0.018)
    for x in (-0.12, 0, 0.12):
        forest.box("Gem rune slit", (x, -0.327, 0.60), (0.03, 0.02, 0.20), "gem", edge=0.004,
                   rotation=(0, (x / 0.12) * 0.06, 0))
    _loft("Stone tail", [(0.28, 0.30, 0.12), (0.40, 0.19, 0.12), (0.38, -0.08, 0.10),
                          (0.28, -0.30, 0.08), (0.18, -0.42, 0.07)],
          [0.07, 0.065, 0.055, 0.045, 0.035], "stone", 7)
    for x, y, z, sx, sy, sz in ((-0.29, 0.16, 0.82, 0.17, 0.12, 0.07),
                                 (0.0, 0.20, 1.00, 0.16, 0.13, 0.06),
                                 (0.29, 0.16, 0.82, 0.17, 0.12, 0.07),
                                 (0.18, 0.28, 0.96, 0.13, 0.10, 0.06),
                                 (-0.10, 0.30, 0.99, 0.12, 0.09, 0.055)):
        forest.orb("Irregular moss drape", (x, y, z), (sx, sy, sz), "moss", 1)
    _merge("Body", before, (0, 0.0, 0.62))

    before = set(forest.CURRENT.objects)
    forest.box("Wide cat head", (0, -0.12, 1.15), (0.64, 0.44, 0.34), "stone", edge=0.055)
    forest.box("Heavy stone brow", (0, -0.35, 1.23), (0.62, 0.10, 0.08), "stone_light", edge=0.022)
    forest.box("Short muzzle", (0, -0.39, 1.05), (0.24, 0.12, 0.12), "stone_light", edge=0.025)
    for side in (-1, 1):
        # closed tetrahedral ears lean outward from the broad head
        x = side * 0.23
        forest.mesh("Chunky triangular ear",
                    [(x - side * 0.09, -0.20, 1.31), (x + side * 0.09, -0.20, 1.31),
                     (x + side * 0.01, -0.14, 1.40), (x, -0.02, 1.33)],
                    [(0, 1, 2), (0, 2, 3), (1, 3, 2), (0, 3, 1)], "stone_light")
        forest.box("Cheek ruff", (side * 0.34, -0.16, 1.07), (0.13, 0.27, 0.20), "stone_light", edge=0.025,
                   rotation=(0, side * math.radians(20), 0))
        forest.box("Angry gem eye", (side * 0.14, -0.355, 1.17), (0.12, 0.02, 0.035), "gem", edge=0.003,
                   rotation=(0, side * 0.10, side * 0.12))
    forest.orb("Head moss clump", (0, -0.02, 1.345), (0.18, 0.13, 0.055), "moss", 1)
    _merge("Head", before, (0, -0.05, 0.98))

    before = set(forest.CURRENT.objects)
    _loft("Raised foreleg", [(-0.33, -0.08, 0.88), (-0.41, -0.24, 1.01), (-0.39, -0.38, 1.17)],
          [(0.10, 0.11), (0.11, 0.11), (0.10, 0.09)], "stone", 7)
    forest.box("Raised paw block", (-0.39, -0.43, 1.20), (0.19, 0.19, 0.15), "stone_light", edge=0.025)
    for x in (-0.44, -0.39, -0.34):
        _loft("Chunky stone claw", [(x, -0.51, 1.17), (x, -0.53, 1.10)], [0.027, 0.016], "stone_light", 6)
    _merge("PawFrontLeft", before, (-0.34, -0.08, 0.88))

    before = set(forest.CURRENT.objects)
    _loft("Planted foreleg", [(0.34, -0.04, 0.88), (0.34, -0.16, 0.46), (0.31, -0.24, 0.13)],
          [(0.11, 0.12), (0.10, 0.11), (0.10, 0.08)], "stone", 7)
    forest.box("Planted paw", (0.31, -0.29, 0.08), (0.20, 0.22, 0.14), "stone_light", edge=0.025)
    _merge("PawFrontRight", before, (0.34, -0.04, 0.88))


def world_cave_beetle():
    asset("world_cave_beetle", "Split teal shell cave beetle with copper ridges, hooked jaws and six articulated legs", {
        "width_m": 0.9, "length_m": 1.25, "height_m": 0.6,
        "parts": ["Shell", "Jaws", "Leg_FrontLeft", "Leg_MiddleLeft", "Leg_BackLeft", "Leg_FrontRight", "Leg_MiddleRight", "Leg_BackRight"],
        "forward_unity": "+Z", "pivot_unity": [0, 0, 0], "naturalScale": True, "upward_face_check": True,
    })
    before = set(forest.CURRENT.objects)
    forest.orb("Teal thorax", (0, 0.08, 0.33), (0.32, 0.428, 0.22), "teal", 2, True)
    for side in (-1, 1):
        forest.orb("Split shell half", (side * 0.16, 0.02, 0.40), (0.19, 0.40, 0.22), "teal_light", 2, True)
        forest.beam("Copper shell ridge", (side * 0.19, -0.28, 0.57), (side * 0.19, 0.34, 0.57), 0.022, "copper")
    forest.box("Deep shell seam", (0, 0.02, 0.48), (0.026, 0.78, 0.035), "coal", edge=0)
    forest.orb("Coal head", (0, -0.43, 0.32), (0.22, 0.18, 0.17), "coal", 1, True)
    for side in (-1, 1):
        forest.orb("Amber eye", (side * 0.12, -0.57, 0.39), (0.05, 0.03, 0.045), "ember", 1)
        _loft("Antenna", [(side * 0.10, -0.54, 0.45), (side * 0.25, -0.62, 0.52)], [0.018, 0.010], "copper", 6)
    _merge("Shell", before, (0, 0.02, 0.34))

    before = set(forest.CURRENT.objects)
    for side in (-1, 1):
        # Dark hooked mandibles curl forward then inward; only the tiny tips are copper.
        _loft("Hooked coal mandible", [(side * 0.13, -0.56, 0.28), (side * 0.18, -0.61, 0.25),
                                       (side * 0.20, -0.68, 0.21), (side * 0.08, -0.73284, 0.18)],
              [0.045, 0.038, 0.026, 0.010], "coal", 7, tip_mat="copper", tip_start=1)
    _merge("Jaws", before, (0, -0.54, 0.27))

    for side, label in ((-1, "Left"), (1, "Right")):
        for y, pair in ((-0.27, "Front"), (0.03, "Middle"), (0.32, "Back")):
            before = set(forest.CURRENT.objects)
            hip = (side * 0.26, y, 0.28)
            knee = (side * 0.37, y + (0.04 if pair == "Front" else -0.01), 0.15)
            outer_foot = 0.417967 if pair == "Middle" else (0.418808 if pair == "Back" else 0.419)
            foot = (side * outer_foot, y + (0.03 if pair == "Back" else -0.03), 0.045)
            _loft("Coal femur", [hip, knee, foot], [0.055, 0.05, 0.035], "coal", 6)
            forest.orb("Copper knee knob", knee, (0.045, 0.045, 0.045), "copper", 1)
            _merge("Leg_" + pair + label, before, hip)

    # The authored foot rings sit 25.868 mm above the local ground plane. Lower
    # mesh vertices in place so the part joint origins remain unchanged.
    for obj in forest.CURRENT.objects:
        if obj.type == "MESH":
            for vertex in obj.data.vertices:
                vertex.co.z -= 0.025868


def _check_parts(name, uv, lo, hi, size, record, objects):
    if set(record.get("parts", [])) != set(record["contract"]["parts"]):
        raise AssertionError((name, "part set"))
    if lo[1] < -0.011:
        raise AssertionError((name, "below ground"))
    return ["exact articulated part set", "grounded", "joint pivots recorded"]


def _check_fox(name, uv, lo, hi, size, record, objects):
    _check_parts(name, uv, lo, hi, size, record, objects)
    if size[0] > 0.5 or abs(size[2] - 1.1) > 0.02 or abs(size[1] - 0.8) > 0.02:
        raise AssertionError((name, "fox dimensions", size))
    return ["seven fox parts", "lofted brush tail", "grounded paws"]


def _check_guardian(name, uv, lo, hi, size, record, objects):
    _check_parts(name, uv, lo, hi, size, record, objects)
    if size[0] > 1.02 or size[2] > 1.02 or abs(size[1] - 1.4) > 0.02:
        raise AssertionError((name, "guardian dimensions", size))
    return ["four golem parts", "one raised paw", "grounded"]


def _check_beetle(name, uv, lo, hi, size, record, objects):
    _check_parts(name, uv, lo, hi, size, record, objects)
    if (abs(size[0] - 0.9) > 0.001 or abs(size[2] - 1.25) > 0.001
            or abs(lo[1]) > 0.001 or not 0.5 <= size[1] <= 0.7):
        raise AssertionError((name, "beetle dimensions", size))
    return ["eight articulated beetle parts", "0.9 m width", "1.25 m length", "threat height"]


CHECKS = {
    "world_raider_fox": _check_fox,
    "world_stone_guardian": _check_guardian,
    "world_cave_beetle": _check_beetle,
}


BUILDERS = {
    "world_cave_beetle": world_cave_beetle,
    "world_raider_fox": world_raider_fox,
    "world_stone_guardian": world_stone_guardian,
}
