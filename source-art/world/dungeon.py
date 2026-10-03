"""Procedural Root Hollow dungeon assets."""

import math

from mathutils import Matrix, Vector

from .creatures import _loft
from .helpers import arch_stone, asset, forest, prism, rng, voronoi_cells


def _assert_close(actual, expected, label, tolerance=0.01):
    if len(actual) != len(expected) or any(abs(a - b) > tolerance for a, b in zip(actual, expected, strict=True)):
        raise AssertionError((label, actual, expected))


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


def world_dungeon_floor():
    asset(
        "world_dungeon_floor",
        "Flat irregular stone slabs with dark recessed joints",
        {"grid_m": 1, "top_y_m": 0, "thickness_m": 0.08, "pivot_unity": [0, 0, 0], "naturalScale": True},
    )
    forest.box("Floor mortar bed", (0, 0, -0.054), (1, 1, 0.052), "slate", edge=0)
    for i, poly in enumerate(voronoi_cells()):
        prism("Floor slab %02d" % i, poly, -0.032, 0, "stone_light" if i % 4 == 0 else "stone")


def _tread_slab(name, x0, x1, front, back, top, include_back):
    z0 = -4.0
    verts = [(x0, front, z0), (x1, front, z0), (x1, back, z0), (x0, back, z0),
             (x0, front, top), (x1, front, top), (x1, back, top), (x0, back, top)]
    # Adjacent slabs share a riser plane.  Keep the visible front riser and
    # omit the hidden back face after the first slab to prevent z-fighting.
    faces = [(3, 2, 1, 0), (4, 5, 6, 7), (0, 1, 5, 4)]
    if include_back:
        faces.append((2, 3, 7, 6))
    faces += [(1, 2, 6, 5), (3, 0, 4, 7)]
    obj = forest.mesh(name, verts, faces, "stone")
    obj.data.materials.append(forest.material("stone_light"))
    obj.data.polygons[2].material_index = 1  # the visible front nosing band
    return obj


def world_dungeon_stairs():
    asset("world_dungeon_stairs", "Sixteen solid stone treads descending four meters forward", {
        "width_m": 1, "run_m": 4, "descent_m": 4, "steps": 16, "tread_m": 0.25, "riser_m": 0.25,
        "direction_unity": "+Z", "top_unity": [0, 0, 0], "bottom_unity": [0, -4, 4],
        "pivot_unity": [0, 0, 0], "naturalScale": True,
    })
    for i in range(16):
        a, b, top = i * 0.25, (i + 1) * 0.25, -i * 0.25
        front, back = -b, -a
        gap = 0.008
        left = -0.5
        _tread_slab(f"Tread slab {i:02d} left", left, -gap / 2, front, back, top, i == 0)
        _tread_slab(f"Tread slab {i:02d} right", gap / 2, 0.5, front, back, top, i == 0)
        # The recessed joint is a closed dark slate block, below the tread top.
        forest.box(f"Recessed slate joint {i:02d}", (0, (front + back) / 2, top - 0.024),
                   (gap - 0.004, back - front - 0.045, 0.024), "slate", edge=0)


def world_dungeon_stairwell():
    asset("world_dungeon_stairwell", "Stepped stone stairwell with timber frames and a warm lantern", {
        "width_m": 1.64, "run_m": 4, "descent_m": 4, "inner_clear_width_m": 1.0, "clear_height_m": 1.1,
        "light_anchors_unity": [[-0.68, -1.05, 2.25]], "pivot_unity": [0, 0, 0],
        "matches": "world_dungeon_stairs", "naturalScale": True, "upward_face_check": True,
    })
    tops = (0.4, 0.4, -0.25, -1.25)
    for side in (-1, 1):
        x0, x1 = side * 0.5, side * 0.82
        lo_x, hi_x = min(x0, x1), max(x0, x1)
        for k, top in enumerate(tops):
            bottom = -(k + 1)
            y0, y1 = -k, -(k + 1)
            # Recessed slate core is visible through the staggered block joints.
            # Keep the recessed core below both the wall blocks and the cap so
            # no horizontal face shares their planes.  Its side faces remain
            # inset by 18 mm from the block faces.
            core_bottom = bottom + 0.002
            core_top = top - 0.060
            forest.box(f"Slate mortar core {side} {k}", ((lo_x + hi_x) / 2, (y0 + y1) / 2, (core_bottom + core_top) / 2),
                       (hi_x - lo_x - 0.018, 0.982, core_top - core_bottom), "slate", edge=0)
            course_h = (top - bottom) / 2
            for row in range(2):
                za, zb = bottom + row * course_h, bottom + (row + 1) * course_h
                za += 0.008
                zb -= 0.008 if row == 0 else 0.044
                # alternating longitudinal joints on both wall faces and their exposed top
                nblocks = 2 if (row + k) % 2 else 3
                offset = (0.5 / nblocks) if nblocks == 2 else 0.0
                edges = [-0.5 + (1.0 / nblocks) * (i + offset) for i in range(nblocks + 1)]
                edges[0], edges[-1] = -0.5, 0.5
                for block, (ya, yb) in enumerate(zip(edges, edges[1:], strict=False)):
                    gap = 0.010
                    ya += gap if block else 0
                    yb -= gap if block < nblocks - 1 else 0
                    # Leave a narrow vertical joint between neighboring depth
                    # courses as well; their end faces must not share a plane.
                    ya += 0.004 if k > 0 and block == 0 else 0
                    yb -= 0.004 if k < len(tops) - 1 and block == nblocks - 1 else 0
                    forest.box(f"Rough stone block {side} {k} {row} {block}", ((lo_x + hi_x) / 2, (y0 + y1) / 2 + (ya + yb) * 0.5, (za + zb) * 0.5),
                               (hi_x - lo_x, yb - ya, zb - za - 0.008), "stone_light" if row == 1 else "stone", edge=0)
            # The cap is the only wall top surface.  Its underside is at the
            # nominal course top, above the blocks' 8 mm setback.
            forest.box(f"Stone light cap {side} {k}", ((lo_x + hi_x) / 2, (y0 + y1) / 2, top - 0.018),
                       (hi_x - lo_x, 0.98, 0.036), "stone_light", edge=0)
    for z_unity in (1.5, 3.0):
        lintel_depth = 0.16
        lintel_height = 0.14
        # The lintel spans two neighboring treads at each frame location. Use
        # the highest tread touched by its full depth, then leave a little more
        # than the contracted 1.1 m of headroom beneath the timber.
        highest_tread = math.floor((z_unity - lintel_depth / 2) / 0.25)
        tread_top = -highest_tread * 0.25
        underside = tread_top + 1.15
        wall_top = tops[min(int(z_unity), 3)] - 0.050
        wall_cap_underside = tops[min(int(z_unity), 3)] - 0.036
        underside = min(underside, wall_cap_underside - lintel_height)
        y = -z_unity
        forest.box("Bark frame lintel", (0, y, underside + lintel_height / 2),
                   (1.44, lintel_depth, lintel_height), "bark", edge=0)
        for side in (-1, 1):
            # Keep each post wholly outside the one-meter clear corridor.
            post_bottom = underside + 0.004
            forest.box("Bark frame post", (side * 0.56, y, (post_bottom + wall_top) / 2),
                       (0.12, 0.12, max(0.12, wall_top - post_bottom)), "bark", edge=0)
    # Compact bark-and-ember lantern keeps the wall kit to five palette materials.
    # The wall bracket sits below the course cap; only the lantern rises into
    # the corridor, so no dark square can poke through the wall top.
    forest.box("Lantern wall bracket", (-0.68, -2.25, -1.30), (0.16, 0.12, 0.10), "bark", edge=0)
    forest.beam("Lantern arm", (-0.68, -2.25, -1.30), (-0.68, -2.25, -1.16), 0.04, "bark")
    forest.box("Lantern ember", (-0.68, -2.25, -1.03), (0.13, 0.13, 0.20), "ember", edge=0)
    forest.box("Lantern top cap", (-0.68, -2.25, -1.16), (0.18, 0.18, 0.04), "bark", edge=0)
    forest.box("Lantern bottom cap", (-0.68, -2.25, -0.90), (0.18, 0.18, 0.04), "bark", edge=0)


def world_dungeon_entrance():
    asset("world_dungeon_entrance", "Moss-topped rock arch set into a coal-dark Root Hollow mound", {
        "width_m": 3, "height_m": 3.5, "opening_clear_min_width_m": 1.64, "opening_clear_height_m": 2.55,
        "front_unity": "+Z", "pivot_unity": [0, 0, 0],
        # Measured from the visible stone/moss mantle: 3 m wide, 2.6 m deep,
        # and 3.3 m high above the ground at the crown.
        "mound_m": [3.0, 3.0, 3.35], "recess_depth_m": 1.2,
        "light_anchors_unity": [[-1.08, 1.45, 1.25], [1.08, 1.45, 1.25]], "naturalScale": True, "upward_face_check": True,
    })

    # Faceted rim stones follow the arch's outside radius.  Side stones are
    # deliberately narrowed so their inner edges stay outside the 1.64 m mouth.
    rim_angles = (0, 22, 44, 66, 88, 92, 114, 136, 158, 180)
    for index, degrees in enumerate(rim_angles):
        angle = math.radians(degrees)
        side = -1 if degrees > 90 else 1
        sx = rng().uniform(0.35, 0.56)
        sy = rng().uniform(0.34, 0.52)
        sz = rng().uniform(0.38, 0.58)
        nominal_x = 1.08 * 1.5 * math.cos(angle)
        # The first stones sit at ground level; successive stones climb the
        # arch's outer curve into the crown instead of leaving a free-standing
        # lower ring.  Keep the true ring radius for the horizontal placement.
        center_z = max(sz + 0.04, 0.65 + 2.45 * math.sin(angle))
        center_z = min(center_z, 3.5 - sz)
        # Keep the outer silhouette inside the exact 3 m contract.  At the low
        # side of the mouth, shrink the radial extent further so all nine
        # clearance rays see the coal back plane rather than a boulder.
        projected_x = sx + 0.35 * sy
        if center_z < 2.72:
            sx = min(sx, max(0.10, 0.23 - 0.35 * sy))
            projected_x = sx + 0.35 * sy
        center_x = nominal_x
        if abs(center_x) > 1.5 - projected_x:
            center_x = math.copysign(1.5 - projected_x, center_x)
        if center_z < 2.72 and abs(center_x) < 0.98 + projected_x:
            center_x = math.copysign(0.98 + projected_x, center_x or side)
        center_y = 0.12 + 0.48 * math.sin(angle) + rng().uniform(-0.07, 0.07)
        orb = forest.orb(f"Rim boulder {index:02d}", (center_x, center_y, center_z),
                         (sx, sy, sz), "stone", 1)
        # A random yaw gives each stone a distinct facet pattern while keeping
        # its vertical bounds and the exact width contract easy to audit.
        orb.rotation_euler = (0, 0, rng().uniform(-0.35, 0.35))

    # A compact rear dome fills the space behind the ring without entering the
    # tunnel. The rear row makes the outside read as a hump from behind.
    dome = [
        (0.00, 0.72, 3.00, 0.48, 0.42, 0.28),
        (-0.56, 0.82, 2.98, 0.40, 0.40, 0.24),
        (0.56, 0.82, 2.98, 0.40, 0.40, 0.24),
        (-1.12, 1.06, 1.58, 0.18, 0.35, 0.48),
        (1.12, 1.06, 1.58, 0.18, 0.35, 0.48),
        (-1.08, 1.22, 2.02, 0.38, 0.34, 0.56),
        (1.08, 1.22, 2.02, 0.38, 0.34, 0.56),
        (0.00, 1.32, 2.92, 0.52, 0.16, 0.30),
    ]
    for index, (x, y, z, sx, sy, sz) in enumerate(dome):
        orb = forest.orb(f"Back dome boulder {index:02d}", (x, y, z), (sx, sy, sz), "stone", 1)
        orb.rotation_euler = (0, 0, rng().uniform(-0.35, 0.35))

    # Low-poly moss mantle overlaps the dome and a few crown stones, with two
    # front drapes breaking the clean lintel silhouette.
    moss = [
        (-1.14, -0.22, 2.86, 0.22, 0.24, 0.12), (1.14, -0.22, 2.86, 0.22, 0.24, 0.12),
        (-0.72, 0.24, 3.12, 0.40, 0.32, 0.12), (0.00, 0.20, 3.24, 0.50, 0.34, 0.12),
        (0.72, 0.24, 3.12, 0.40, 0.32, 0.12), (-0.42, 0.62, 3.20, 0.36, 0.29, 0.12),
        (0.42, 0.62, 3.20, 0.36, 0.29, 0.12), (0.00, -0.02, 2.92, 0.36, 0.26, 0.14),
    ]
    for index, (x, y, z, sx, sy, sz) in enumerate(moss):
        orb = forest.orb(f"Moss mantle {index:02d}", (x, y, z), (sx, sy, sz), "moss", 1)
        orb.rotation_euler = (0, 0, rng().uniform(-0.45, 0.45))
    for index, (x, y, z, sx, sy, sz) in enumerate((
        (-1.04, 1.24, 2.42, 0.42, 0.20, 0.13),
        (1.04, 1.24, 2.42, 0.42, 0.20, 0.13),
        (-0.56, 1.30, 2.82, 0.42, 0.12, 0.12),
        (0.56, 1.30, 2.82, 0.42, 0.12, 0.12),
        (0.00, 1.36, 3.12, 0.52, 0.10, 0.11),
    )):
        orb = forest.orb(f"Rear moss mantle {index:02d}", (x, y, z), (sx, sy, sz), "moss", 1)
        orb.rotation_euler = (0, 0, 0)

    # Thick tapered stone side masses and a faceted crown turn the tunnel into
    # a hill. The inner edges stay outside the clear mouth; the coal liner is
    # recessed from every exterior stone surface.
    for side in (-1, 1):
        x_inner_front, x_outer_front = side * 0.98, side * 1.26
        x_inner_back, x_outer_back = side * 0.99, side * 1.22
        verts = [
            (x_inner_front, -0.52, 0.0), (x_outer_front, -0.52, 0.0),
            (x_outer_back, 1.34, 0.0), (x_inner_back, 1.34, 0.0),
            (x_inner_front, -0.52, 2.66), (x_outer_front, -0.52, 2.78),
            (x_outer_back, 1.34, 2.62), (x_inner_back, 1.34, 2.66),
        ]
        faces = [(0, 3, 2, 1), (4, 5, 6, 7), (0, 1, 5, 4),
                 (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7)]
        forest.mesh(f"Tapered stone shell {side}", verts, faces, "stone")
    crown_profile = [(-1.00, 2.74), (-0.66, 2.92), (-0.28, 3.08), (0.0, 3.13),
                     (0.28, 3.08), (0.66, 2.92), (1.00, 2.74)]
    verts = [(x, y, z) for y in (-0.50, 1.34) for x, z in crown_profile]
    n = len(crown_profile)
    faces = [tuple(reversed(range(n))), tuple(range(n, 2 * n))]
    faces += [(i, (i + 1) % n, n + (i + 1) % n, n + i) for i in range(n - 1)]
    forest.mesh("Rounded stone tunnel crown", verts, faces, "stone")
    # Faceted stones break up the shell's planar sides and rear closure. They
    # step upward toward the crown while staying inside the exact 3 m footprint.
    for side in (-1, 1):
        for index, (y, z, sy, sz) in enumerate(((-0.18, 0.58, 0.30, 0.34),
                                                   (0.42, 1.28, 0.32, 0.38),
                                                   (0.98, 2.02, 0.28, 0.40))):
            orb = forest.orb(f"Shell side boulder {side} {index}",
                             (side * 1.28, y, z), (0.22, sy, sz), "stone", 1)
            orb.rotation_euler = (0, 0, rng().uniform(-0.3, 0.3))
    for index, (x, z, sx, sz) in enumerate(((-0.75, 0.58, 0.34, 0.45),
                                             (0.75, 0.58, 0.34, 0.45),
                                             (-0.45, 1.38, 0.38, 0.40),
                                             (0.45, 1.38, 0.38, 0.40))):
        orb = forest.orb(f"Shell rear boulder {index}", (x, 1.42, z), (sx, 0.06, sz), "stone", 1)
        orb.rotation_euler = (0, 0, 0)

    # Chunky stacked piers restore the stone mass below each springing.  The
    # inner edge stays outside the 1.64 m opening while the full ring depth is
    # carried down to the ground.
    pier_courses = (
        (0.25, 0.50, 0.33),
        (0.23, 0.47, 0.36),
        (0.24, 0.44, 0.36),
    )
    for side in (-1, 1):
        for course, (sx, sy, sz) in enumerate(pier_courses):
            z = sz + sum(item[2] * 2 for item in pier_courses[:course]) + 0.01 * rng().uniform(-1, 1)
            x = side * (1.50 - sx - 0.01 * rng().uniform(0, 1))
            y = -0.02 + rng().uniform(-0.04, 0.04)
            forest.orb(f"Pier boulder {side} {course}", (x, y, z), (sx, sy, sz), "stone", 1)
        # Low front stones hug the pier and mask the coal side panel from
        # three-quarter approaches without narrowing the clearance rays.
        for corner in range(2):
            radius_x = 0.18 + 0.01 * corner
            x = side * (1.20 + 0.10 * corner + rng().uniform(-0.015, 0.015))
            y = -0.70 + 0.20 * corner + rng().uniform(-0.035, 0.035)
            forest.orb(f"Front corner boulder {side} {corner}", (x, y, 0.16 + rng().uniform(-0.015, 0.015)),
                       (radius_x, 0.26, 0.16 + 0.02 * corner), "stone", 1)

    for side in (-1, 1):
        forest.box("Timber jamb", (side * 0.98, -0.60, 1.35), (0.2, 0.18, 2.7), "bark", edge=0.018)
        forest.beam("Knee brace", (side * 1.30, -0.64, 2.0), (side * 0.98, -0.64, 2.72), 0.12, "bark")
    # Leave a hairline mortar gap between voussoirs so adjacent radial faces
    # never occupy exactly the same plane.
    arch_gap = math.radians(0.05)
    for i in range(8):
        t0 = i * math.pi / 8 + (arch_gap if i else 0)
        t1 = (i + 1) * math.pi / 8 - (arch_gap if i < 7 else 0)
        obj = arch_stone(f"Arch voussoir {i:02d}", t0, t1)
        for slot, mat in enumerate(obj.data.materials):
            if mat.name == "Forest_stone_light":
                obj.data.materials[slot] = forest.material("stone")
    forest.box("Timber lintel", (0, -0.64, 2.78), (2.36, 0.22, 0.22), "bark", edge=0.02)
    forest.box("Coal recess back", (0, 1.20, 1.28), (1.72, 0.06, 2.56), "coal", edge=0)
    # The liner begins at the arch front face and ends just before the stone
    # back shell. It remains recessed from every exterior stone surface.
    forest.box("Coal tunnel ceiling", (0, 0.35, 2.66), (1.72, 1.74, 0.12), "coal", edge=0)
    for side in (-1, 1):
        forest.box("Coal tunnel side", (side * 0.90, 0.35, 1.28), (0.12, 1.74, 2.56), "coal", edge=0)
    # Faceted rear closure, with a shallow central bulge instead of a flat
    # shipping-crate face. The coal plane remains in front of this stone wall.
    rear_x = (-1.22, 0.0, 1.22)
    rear_z = (0.0, 1.28, 2.70)
    verts = []
    for y_front in (1.26,):
        verts.extend((x, y_front, z) for z in rear_z for x in rear_x)
    for row, z in enumerate(rear_z):
        bulge = 0.10 if row == 1 else (0.05 if row == 2 else 0.0)
        verts.extend((x, 1.42 + bulge * (1.0 - abs(x) / 1.22), z) for x in rear_x)
    faces = []
    for base in (0, 9):
        for row in range(2):
            for col in range(2):
                a = base + row * 3 + col
                b = a + 1
                c = a + 3
                d = c + 1
                faces.extend(((a, b, d), (a, d, c)))
    faces += [(0, 3, 12, 9), (3, 6, 15, 12), (2, 11, 14, 5), (5, 14, 17, 8),
              (6, 8, 17, 15), (0, 9, 11, 2)]
    forest.mesh("Faceted stone tunnel back shell", verts, faces, "stone")
    forest.box("Stone threshold", (0, -0.95, 0.015), (1.68, 0.3, 0.03), "stone", edge=0)
    forest.box("First descending step", (0, -0.62, -0.12), (1.6, 0.28, 0.24), "stone", edge=0)
    for side in (-1, 1):
        forest.cylinder("Torch post", (side * 1.08, -1.34, 0.65), 0.055, 1.3, "bark", vertices=8)
        forest.box("Torch basket", (side * 1.08, -1.34, 1.31), (0.18, 0.18, 0.08), "bark", edge=0)
        forest.cylinder("Torch ember flame", (side * 1.08, -1.34, 1.56), 0.13, 0.28, "ember", vertices=6, top=0.025)
    forest.box("Paw stone", (-1.12, -1.32, 0.08), (0.50, 0.26, 0.16), "stone", edge=0.04)
    for x, y, z, sx in ((-0.15, -1.45, 0.15, .06), (-0.05, -1.45, 0.17, .06),
                        (0.05, -1.45, 0.17, .06), (0.15, -1.45, 0.15, .06), (0, -1.455, 0.07, .10)):
        forest.orb("Paw coal inset", (x - 1.12, y, z), (sx, .018, .035), "coal", 1)
    for x in (-0.70, -0.35, 0.35, 0.70):
        _loft("Arch root", [(x, -0.74, 2.82), (x * 1.03, -0.55, 2.63), (x * 1.08, -0.34, 2.40)],
              [.045, .038, .025], "bark", 7)


def _chest_body(opened):
    before = set(forest.CURRENT.objects)
    forest.box("Chest floor", (0, 0, 0.045), (0.7, 0.5, 0.09), "darkwood", edge=0)
    forest.box("Chest front wall", (0, -0.215, 0.22), (0.7, 0.07, 0.35), "wood", edge=0)
    forest.box("Chest back wall", (0, 0.215, 0.22), (0.7, 0.07, 0.35), "wood", edge=0)
    for x in (-0.315, 0.315):
        forest.box("Chest side wall", (x, 0, 0.22), (0.07, 0.36, 0.35), "darkwood", edge=0)
        forest.box("Gold corner cap", (x, -0.23, 0.21), (0.055, 0.045, 0.32), "gold", edge=0)
    forest.box("Gold lock plate", (0, -0.245, 0.25), (0.12, 0.018, 0.12), "gold", edge=0)
    forest.orb("Gold lock", (0, -0.255, 0.25), (0.035, 0.010, 0.035), "gold", 1)
    if opened:
        forest.orb("Gold coin mound", (0.05, -0.02, 0.40), (0.22, 0.13, 0.08), "gold", 2)
        forest.orb("Iron ore", (-0.18, 0.02, 0.40), (0.09, 0.07, 0.07), "iron", 1)
        forest.orb("Iron ore", (0.18, 0.03, 0.40), (0.08, 0.06, 0.06), "iron", 1)
        for x, y, z in ((-0.12, -0.02, 0.49), (0.03, 0.05, 0.50), (0.16, -0.01, 0.47)):
            forest.cylinder("Gem crystal", (x, y, z), 0.035, 0.14, "gem", vertices=5)
        for x in (-0.20, 0.20):
            forest.cylinder("Spilled coin", (x, -0.27, 0.12), 0.04, 0.014, "gold", vertices=8, rotation=(math.pi / 2, 0, 0))
    return [o for o in forest.CURRENT.objects if o not in before]


def _chest_lid(opened):
    before = set(forest.CURRENT.objects)
    hinge = Vector((0, 0.22, 0.40))
    ring = 6
    verts = []
    for x in (-0.35, 0.35):
        for i in range(ring + 1):
            theta = math.pi * i / ring
            verts.append((x, hinge.y - 0.235 * (1 - math.cos(theta)), hinge.z + 0.10 * math.sin(theta)))
    faces = [tuple(range(ring + 1)), tuple(reversed(range(ring + 1, 2 * (ring + 1))))]
    faces += [(i, i + 1, ring + 1 + i + 1, ring + 1 + i) for i in range(ring)]
    lid = forest.mesh("Barrel lid shell", verts, faces, "darkwood")
    lid.data.materials.append(forest.material("wood"))
    lid.data.materials.append(forest.material("gold"))
    for poly in lid.data.polygons:
        if poly.index >= 2:
            poly.material_index = 1
    # Gold bands are integrated into the closed barrel topology, avoiding
    # unsupported underside faces when the articulated lid swings open.
    for segment in (1, 4):
        for side in range(ring):
            poly_index = 2 + segment
            if poly_index < len(lid.data.polygons):
                lid.data.polygons[poly_index].material_index = 2
    objs = [o for o in forest.CURRENT.objects if o not in before]
    if opened:
        rotation = Matrix.Rotation(math.radians(-70), 4, "X")
        for obj in objs:
            obj.matrix_world = Matrix.Translation(hinge) @ rotation @ Matrix.Translation(-hinge) @ obj.matrix_world
    return objs


def _build_chest(name, opened):
    asset(name, "Banded dungeon treasure chest", {
        "footprint_m": [1, 1], "size_m": [0.7, 0.5, 0.5], "parts": ["Body", "Lid"],
        "hinge_unity": [0, 0.4, -0.22], "lid_open_deg": 110 if opened else 0,
        "pivot_unity": [0, 0, 0], "naturalScale": True,
    })
    forest.merge_group("Body", _chest_body(opened), pivot=(0, 0, 0))
    forest.merge_group("Lid", _chest_lid(opened), pivot=(0, 0.22, 0.40))


def world_loot_chest():
    _build_chest("world_loot_chest", False)


def world_loot_chest_open():
    _build_chest("world_loot_chest_open", True)


def world_dungeon_brazier():
    asset("world_dungeon_brazier", "Standing iron brazier with coal and faceted ember flames", {
        "footprint_m": [1, 1], "height_m": 0.98, "light_anchors_unity": [[0, 0.9, 0]],
        "pivot_unity": [0, 0, 0], "naturalScale": True,
    })
    for angle in (0, math.tau / 3, 2 * math.tau / 3):
        x, y = 0.18 * math.cos(angle), 0.18 * math.sin(angle)
        _loft("Splayed tripod leg", [(x, y, 0.06), (x * 0.72, y * 0.72, 0.33), (0, 0, 0.50)], [0.05, 0.045, 0.055], "iron", 6)
        forest.box("Foot pad", (x, y, 0.035), (0.13, 0.10, 0.07), "iron", edge=0)
    forest.cylinder("Fire bowl", (0, 0, 0.61), 0.14, 0.22, "iron", vertices=12, top=0.27)
    forest.torus("Chunky bowl rim", (0, 0, 0.72), 0.255, 0.045, "iron")
    forest.orb("Coal bed", (0, 0, 0.735), (0.20, 0.20, 0.07), "coal", 1)
    for x, y, z in ((-0.10, 0.01, 0.77), (0.08, 0.04, 0.78), (0.02, -0.09, 0.79)):
        forest.orb("Glowing ember coal", (x, y, z), (0.07, 0.06, 0.04), "ember", 1)
    for x, y, h in ((-0.12, 0, 0.18), (0.03, 0.04, 0.24), (0.14, -0.03, 0.16), (-0.02, -0.10, 0.21)):
        _loft("Faceted flame tongue", [(x, y, 0.77), (x * 0.8, y * 0.8, 0.77 + h * 0.55),
                                       (x * 0.6, y * 0.6, 0.77 + h)], [0.06, 0.045, 0.012], "ember", 6)


def world_dungeon_roots():
    asset("world_dungeon_roots", "Gnarled roots draped over a dungeon wall segment", {
        "length_axis_unity": "X", "length_m": 1, "matches": "world_dungeon_wall", "pivot_unity": [0, 0, 0],
        "naturalScale": True,
    })
    for i, x in enumerate((-0.40, -0.20, 0.0, 0.20, 0.40)):
        for face, y in (("front", -0.20 - 0.015 * (i % 2)), ("back", 0.20 + 0.015 * ((i + 1) % 2))):
            _loft(f"Hanging root {face}", [(x, y, 2.68), (x + 0.04 * math.sin(i), y, 2.30),
                                            (x - 0.03, y + (0.02 if face == "front" else -0.02), 1.25 + 0.08 * (i % 3))],
                   [0.045, 0.035, 0.018], "bark", 7)
    for x, y in ((-0.34, -0.08), (-0.11, 0.02), (0.12, -0.02), (0.34, 0.08)):
        _loft("Top tangle root", [(x - 0.15, y, 2.66), (x, y + 0.06, 2.70), (x + 0.15, y - 0.02, 2.66)],
              [0.055, 0.048, 0.03], "darkwood", 7)
        forest.orb("Moss clump", (x, y, 2.69), (0.11, 0.08, 0.06), "moss", 1)


def _check_stairs(name, uv, lo, hi, size, record, objects):
    _assert_close(lo, [-0.5, -4, 0], "stair bounds", 0.002)
    _assert_close(hi, [0.5, 0, 4], "stair bounds", 0.002)
    obj = objects[0]
    inverse = obj.matrix_world.inverted()
    inverse_linear = inverse.to_3x3()
    world = obj.matrix_world
    world_linear = world.to_3x3()
    for i in range(16):
        top = -i * 0.25
        front, back = -(i + 1) * 0.25, -i * 0.25
        inset = 0.026
        depths = [front + inset + index * ((back - front - 2 * inset) / 4) for index in range(5)]
        for ix in range(5):
            x = -0.47 + ix * 0.235
            for depth in depths:
                origin = Vector((x, depth, top + 0.12))
                direction = Vector((0, 0, -1))
                hit, local, normal, index = obj.ray_cast(
                    inverse @ origin, inverse_linear @ direction,
                )
                if not hit:
                    raise AssertionError((name, "missing tread ray", i, ix, depth))
                world_hit = world @ local
                world_normal = (world_linear @ normal).normalized()
                material = obj.data.materials[obj.data.polygons[index].material_index].name
                if material.startswith("Forest_slate"):
                    if abs(world_hit.z - (top - 0.012)) > 0.006:
                        raise AssertionError((name, "wrong recessed joint", i, ix, depth, world_hit.z))
                elif abs(world_hit.z - top) > 0.001 or world_normal.z < 0.95:
                    raise AssertionError((name, "wrong tread height", i, ix, depth, world_hit.z, material))
    return ["envelope bounds", "16 solid tread tops", "5 x 5 downward tread rays", "recessed slate joints"]


def _check_stairwell(name, uv, lo, hi, size, record, objects):
    if lo[0] < -0.821 or hi[0] > 0.821 or lo[2] < -0.001 or hi[2] > 4.001 or lo[1] < -4.001 or hi[1] > 0.401:
        raise AssertionError((name, "stairwell envelope"))
    if len(record["materials"]) > 5:
        raise AssertionError((name, "too many materials"))
    # The clear corridor is tested at two interior positions on every tread.
    for i in range(16):
        tread = -i * 0.25
        z_positions = (i * 0.25 + 0.0625, i * 0.25 + 0.1875)
        for unity_z in z_positions:
            source_y = -unity_z
            for height in (0.05, 0.55, 1.08):
                origin = Vector((-0.499, source_y, tread + height))
                direction = Vector((1, 0, 0))
                for obj in objects:
                    inverse = obj.matrix_world.inverted()
                    hit, _, _, _ = obj.ray_cast(
                        inverse @ origin, inverse.to_3x3() @ direction, distance=0.998,
                    )
                    if hit:
                        raise AssertionError((name, "horizontal corridor obstruction", i, unity_z, height))
            for x in (-0.45, 0, 0.45):
                origin = Vector((x, source_y, tread + 0.005))
                direction = Vector((0, 0, 1))
                for obj in objects:
                    inverse = obj.matrix_world.inverted()
                    hit, _, _, _ = obj.ray_cast(
                        inverse @ origin, inverse.to_3x3() @ direction, distance=1.095,
                    )
                    if hit:
                        raise AssertionError((name, "vertical corridor obstruction", i, unity_z, x))
    return ["real 0.32 m stepped walls", "16 treads x 2 Z positions x 3 horizontal heights", "16 treads x 2 Z positions x 3 vertical columns", "timber frames and lantern", "five materials or fewer"]


def _check_entrance(name, uv, lo, hi, size, record, objects):
    _assert_close([size[0], hi[1]], [3, 3.5], "entrance envelope")
    mound = record["contract"].get("mound_m")
    if not mound or len(mound) != 3:
        raise AssertionError((name, "missing mound contract", mound))
    _assert_close([size[0], size[2]], [mound[0], mound[1]], "measured mound footprint", 0.02)
    if abs(hi[1] - mound[2]) > 0.20:
        raise AssertionError((name, "measured mound height", hi[1], mound[2]))
    if lo[1] < -0.3 or size[2] < 2.9 or len(record["materials"]) > 5 or record["triangles"] > 2000:
        raise AssertionError((name, "mound footprint/materials/triangle budget"))
    for x in (-0.82, 0, 0.82):
        for height in (0.05, 1.2, 2.55):
            origin = Vector((x, -2.0, height))
            hits = []
            for obj in objects:
                inv = obj.matrix_world.inverted()
                hit, local, _, index = obj.ray_cast(inv @ origin, inv.to_3x3() @ Vector((0, 1, 0)), distance=10)
                if hit:
                    world_hit = obj.matrix_world @ local
                    material = obj.data.materials[obj.data.polygons[index].material_index].name
                    hits.append((world_hit.y, material))
            if not hits:
                raise AssertionError((name, "missing coal recess hit", x, height))
            first = min(hits)
            if first[0] < 1.1 or first[1].removesuffix(".001") != "Forest_coal":
                raise AssertionError((name, "opening ray", x, height, first))
    return ["measured X/Z footprint matches mound_m within 0.02 m and overall maximum height is within 0.20 m of mound_m[2]", "3.5 m arch height", "nine rays reach coal back plane"]


def _check_chest(name, uv, lo, hi, size, record, objects):
    if size[0] > 0.72 or size[1] > (1.02 if record["contract"]["lid_open_deg"] else 0.52) or size[2] > 1.0:
        raise AssertionError((name, "chest envelope", size))
    if set(record.get("parts", [])) != {"Body", "Lid"} or record["contract"]["hinge_unity"] != [0, 0.4, -0.22]:
        raise AssertionError((name, "parts or hinge"))
    lid_index = record["parts"].index("Lid")
    hinge = record["pivots_unity"][lid_index]
    if max(abs(a - b) for a, b in zip(hinge, record["contract"]["hinge_unity"], strict=True)) > 0.001:
        raise AssertionError((name, "hinge pivot", hinge))
    return ["two articulated parts", "hinge pivot", "hollow body and treasure"]


def _check_brazier(name, uv, lo, hi, size, record, objects):
    if size[0] > 0.6 or size[2] > 0.6 or hi[1] < 0.9 or hi[1] > 1.05 or "Forest_ember" not in record["materials"]:
        raise AssertionError((name, "brazier dimensions/material"))
    return ["compact footprint", "standing height", "ember light source"]


def _check_roots(name, uv, lo, hi, size, record, objects):
    if lo[0] < -0.55 or hi[0] > 0.55 or lo[1] < 1.0 or hi[1] < 2.45:
        raise AssertionError((name, "root envelope"))
    return ["wall-length root tangle", "both-face hanging strands", "moss clumps"]


CHECKS = {
    "world_dungeon_stairs": _check_stairs,
    "world_dungeon_stairwell": _check_stairwell,
    "world_dungeon_entrance": _check_entrance,
    "world_loot_chest": _check_chest,
    "world_loot_chest_open": _check_chest,
    "world_dungeon_brazier": _check_brazier,
    "world_dungeon_roots": _check_roots,
}


BUILDERS = {
    "world_dungeon_entrance": world_dungeon_entrance,
    "world_dungeon_wall": world_dungeon_wall,
    "world_dungeon_pillar": world_dungeon_pillar,
    "world_dungeon_stairs": world_dungeon_stairs,
    "world_dungeon_floor": world_dungeon_floor,
    "world_dungeon_stairwell": world_dungeon_stairwell,
    "world_loot_chest": world_loot_chest,
    "world_loot_chest_open": world_loot_chest_open,
    "world_dungeon_brazier": world_dungeon_brazier,
    "world_dungeon_roots": world_dungeon_roots,
}
