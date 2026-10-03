"""Procedural harvestable nodes, plants, saplings and stumps."""

from mathutils import Vector

from .helpers import asset, forest, math, rng


def _contract(resource=None, height=0.7, variant=None):
    result = {
        "footprint_m": [1, 1],
        "max_height_m": height,
        "pivot_unity": [0, 0, 0],
        "naturalScale": True,
    }
    if resource is not None:
        result["resource"] = resource
    if variant:
        result["variant"] = variant
    return result


def _bud(name, pos, size, mat):
    x, y, z = pos
    r = size
    verts = [
        (x - r, y, z),
        (x, y - r, z),
        (x + r, y, z),
        (x, y + r, z),
        (x, y, z + r),
        (x, y, z - r),
    ]
    faces = [
        (0, 1, 4),
        (1, 2, 4),
        (2, 3, 4),
        (3, 0, 4),
        (1, 0, 5),
        (2, 1, 5),
        (3, 2, 5),
        (0, 3, 5),
    ]
    return forest.mesh(name, verts, faces, mat)


def _leaf_cluster(name, pos, size, mat):
    x, y, z = pos
    rx, ry, rz = size
    verts = [
        (x - rx, y, z),
        (x, y - ry, z),
        (x + rx, y, z),
        (x, y + ry, z),
        (x, y, z + rz),
        (x, y, z - rz),
    ]
    faces = [
        (0, 1, 4),
        (1, 2, 4),
        (2, 3, 4),
        (3, 0, 4),
        (1, 0, 5),
        (2, 1, 5),
        (3, 2, 5),
        (0, 3, 5),
    ]
    return forest.mesh(name, verts, faces, mat)


def _tiny_leaf(name, base, length, width, rise, azimuth, mat):
    """Small closed triangular blade for low-poly crop accents."""
    x, y, z = base
    dx, dy = math.cos(azimuth), math.sin(azimuth)
    px, py = -dy, dx
    tx, ty, tz = x + dx * length, y + dy * length, z + rise
    verts = [
        (x + px * width, y + py * width, z),
        (x - px * width, y - py * width, z),
        (tx, ty, tz),
        (x + px * width, y + py * width, z + 0.012),
        (x - px * width, y - py * width, z + 0.012),
        (tx, ty, tz + 0.012),
    ]
    faces = [(0, 1, 2), (5, 4, 3), (0, 3, 4, 1), (1, 4, 5, 2), (2, 5, 3, 0)]
    return forest.mesh(name, verts, faces, mat)


def _spoon_leaf(name, base, length, width, rise, azimuth, mat):
    """Closed spoon-shaped leaf with a broad middle and tapered tip."""
    x, y, z = base
    dx, dy = math.cos(azimuth), math.sin(azimuth)
    px, py = -dy, dx
    mx, my, mz = x + dx * length * 0.58, y + dy * length * 0.58, z + rise * 0.58
    tx, ty, tz = x + dx * length, y + dy * length, z + rise
    verts = [
        (x, y, z - 0.012),
        (mx + px * width, my + py * width, mz - 0.012),
        (tx, ty, tz - 0.008),
        (mx - px * width, my - py * width, mz - 0.012),
        (x, y, z + 0.012),
        (mx + px * width, my + py * width, mz + 0.012),
        (tx, ty, tz + 0.008),
        (mx - px * width, my - py * width, mz + 0.012),
    ]
    faces = [
        (0, 1, 2, 3),
        (7, 6, 5, 4),
        (0, 4, 5, 1),
        (1, 5, 6, 2),
        (2, 6, 7, 3),
        (3, 7, 4, 0),
    ]
    return forest.mesh(name, verts, faces, mat)


def _cupped_spoon_leaf(name, base, length, width, rise, azimuth, mat):
    """Closed broad spoon leaf with lifted edges around a shallow central cup."""
    x, y, z = base
    dx, dy = math.cos(azimuth), math.sin(azimuth)
    px, py = -dy, dx
    outline = ((0, 0), (0.28, 0.70), (0.72, 1), (1, 0), (0.72, -1), (0.28, -0.70))
    verts = []
    for layer in (0, 1):
        for along, across in outline:
            lift = 0.014 * abs(across)
            verts.append((x + dx * length * along + px * width * across,
                          y + dy * length * along + py * width * across,
                          z + rise * along + lift + (0.009 if layer else -0.012)))
    verts.append((x + dx * length * 0.57, y + dy * length * 0.57, z + rise * 0.57 + 0.003))
    faces = [tuple(reversed(range(6)))]
    for i in range(6):
        j = (i + 1) % 6
        faces.extend(((6 + i, 6 + j, 12), (i, j, 6 + j, 6 + i)))
    return forest.mesh(name, verts, faces, mat)


def _leaf_blade(name, base, length, width, rise, azimuth, mat):
    """Closed tapered leaf blade angled up and out from a ground rosette."""
    x, y, z = base
    dx, dy = math.cos(azimuth), math.sin(azimuth)
    px, py = -dy, dx
    tip = (x + dx * length, y + dy * length, z + rise)
    bw, tw, thick = width, width * 0.14, 0.012
    verts = [
        (x + px * bw, y + py * bw, z - thick),
        (x - px * bw, y - py * bw, z - thick),
        (tip[0] - px * tw, tip[1] - py * tw, tip[2] - thick),
        (tip[0] + px * tw, tip[1] + py * tw, tip[2] - thick),
        (x + px * bw, y + py * bw, z + thick),
        (x - px * bw, y - py * bw, z + thick),
        (tip[0] - px * tw, tip[1] - py * tw, tip[2] + thick),
        (tip[0] + px * tw, tip[1] + py * tw, tip[2] + thick),
    ]
    faces = [
        (0, 1, 2, 3),
        (7, 6, 5, 4),
        (0, 4, 5, 1),
        (1, 5, 6, 2),
        (2, 6, 7, 3),
        (3, 7, 4, 0),
    ]
    return forest.mesh(name, verts, faces, mat)


def _ring(name, outer, inner, z, mat):
    n = 10
    verts = []
    for zz in (z - 0.009, z + 0.009):
        for radius in (outer, inner):
            verts += [
                (
                    math.cos(i * math.tau / n) * radius,
                    math.sin(i * math.tau / n) * radius,
                    zz,
                )
                for i in range(n)
            ]
    faces = []
    for i in range(n):
        j = (i + 1) % n
        faces += [(i, j, 2 * n + j, 2 * n + i), (n + i, 3 * n + i, 3 * n + j, n + j)]
        faces += [(i, 2 * n + i, 2 * n + j, j), (n + j, 3 * n + j, 3 * n + i, n + i)]
    return forest.mesh(name, verts, faces, mat)


def _hull(
    name, radius, height, mat, yscale=0.82, center=(0, 0), sides=8, top_scale=0.45
):
    """Closed irregular three-ring hull with jittered faceted silhouette."""
    cx, cy = center
    rings = []
    for ring_index, (z, scale) in enumerate(
        ((-0.02, 1.0), (height * 0.52, 0.94), (height, top_scale))
    ):
        ring = []
        for i in range(sides):
            angle = i * math.tau / sides + (0.07 if ring_index == 1 else 0)
            jitter = 1 + rng().uniform(-0.10, 0.10)
            ring.append(
                (
                    cx + math.cos(angle) * radius * scale * jitter,
                    cy + math.sin(angle) * radius * yscale * scale * jitter,
                    z,
                )
            )
        rings.append(ring)
    verts = [point for ring in rings for point in ring]
    faces = [tuple(reversed(range(sides)))]
    for layer in range(2):
        a, b = layer * sides, (layer + 1) * sides
        for i in range(sides):
            j = (i + 1) % sides
            faces.append((a + i, a + j, b + j, b + i))
    faces.append(tuple(range(2 * sides, 3 * sides)))
    return forest.mesh(name, verts, faces, mat)


def _mound(name, radius, height, mat, yscale=0.82):
    return _hull(name, radius, height, mat, yscale=yscale, sides=7, top_scale=0.5)


def _angled_crystal(name, base, radius, length, tilt, azimuth, mat, sides=6):
    """Closed hexagonal crystal with an axis tilted away from vertical."""
    bx, by, bz = base
    axis = (
        math.sin(tilt) * math.cos(azimuth),
        math.sin(tilt) * math.sin(azimuth),
        math.cos(tilt),
    )
    u = (-math.sin(azimuth), math.cos(azimuth), 0)
    v = (
        axis[1] * u[2] - axis[2] * u[1],
        axis[2] * u[0] - axis[0] * u[2],
        axis[0] * u[1] - axis[1] * u[0],
    )
    verts = []
    for center, scale in (
        ((bx, by, bz), 1.0),
        (
            (
                bx + axis[0] * length * 0.72,
                by + axis[1] * length * 0.72,
                bz + axis[2] * length * 0.72,
            ),
            0.72,
        ),
    ):
        for i in range(sides):
            a = i * math.tau / sides
            verts.append(
                (
                    center[0]
                    + radius * scale * (math.cos(a) * u[0] + math.sin(a) * v[0]),
                    center[1]
                    + radius * scale * (math.cos(a) * u[1] + math.sin(a) * v[1]),
                    center[2]
                    + radius * scale * (math.cos(a) * u[2] + math.sin(a) * v[2]),
                )
            )
    verts.append((bx + axis[0] * length, by + axis[1] * length, bz + axis[2] * length))
    faces = [
        tuple(reversed(range(sides))),
        (
            2 * sides - 1,
            2 * sides - 2,
            2 * sides - 3,
            2 * sides - 4,
            2 * sides - 5,
            2 * sides - 6,
        ),
    ]
    faces += [
        (i, (i + 1) % sides, sides + (i + 1) % sides, sides + i) for i in range(sides)
    ]
    faces += [(sides + i, sides + (i + 1) % sides, 2 * sides) for i in range(sides)]
    return forest.mesh(name, verts, faces, mat)


def _pick_head(name, center, length=0.46):
    x, y, z = center
    points = [
        (-length * 0.5, 0),
        (-length * 0.34, 0.055),
        (-length * 0.10, 0.042),
        (length * 0.10, 0.042),
        (length * 0.34, 0.055),
        (length * 0.5, 0),
        (length * 0.34, -0.055),
        (-length * 0.34, -0.055),
    ]
    verts = [(x + px, y - 0.032, z + pz) for px, pz in points] + [
        (x + px, y + 0.032, z + pz) for px, pz in points
    ]
    faces = [tuple(reversed(range(8))), tuple(range(8, 16))]
    faces += [(i, (i + 1) % 8, 8 + (i + 1) % 8, 8 + i) for i in range(8)]
    return forest.mesh(name, verts, faces, "iron")


def _pick(x, y, z, lean=0.0, height=0.34, head_length=0.34):
    """Lean a compact T-shaped pick against the boulder side."""
    top = (x + lean, y, z + height)
    forest.beam("Pick copper haft", (x, y, z), top, 0.038, "copper")
    _pick_head("Pick double point head", top, head_length)


def world_rock_a():
    asset("world_rock_a", "Tall chunky faceted stone node", _contract("stone", 0.79))
    _hull("Tall boulder", 0.35, 0.68, "stone", yscale=0.82, sides=12, top_scale=0.42)
    forest.orb(
        "Boulder shoulder", (0.02, 0.01, 0.43), (0.22, 0.18, 0.23), "stone_light", 1
    )
    forest.orb("Moss cap", (-0.04, -0.03, 0.68), (0.18, 0.14, 0.06), "moss", 1)


def world_rock_b():
    asset(
        "world_rock_b",
        "Two separate leaning faceted stones touching at their crowns",
        _contract("stone", 0.72),
    )
    _hull("Rock foot", 0.40, 0.10, "stone", yscale=0.82, sides=8, top_scale=0.65)
    _hull(
        "Left leaning stone",
        0.23,
        0.55,
        "stone",
        yscale=0.9,
        center=(-0.15, 0.02),
        sides=10,
        top_scale=0.34,
    )
    _hull(
        "Right leaning stone",
        0.23,
        0.57,
        "stone_light",
        yscale=0.9,
        center=(0.15, 0.02),
        sides=10,
        top_scale=0.34,
    )
    forest.orb("Moss crown", (0.0, -0.02, 0.59), (0.14, 0.09, 0.05), "moss", 1)


def world_rock_c():
    asset(
        "world_rock_c",
        "Low wide irregular slab with a moss cap and loose foot stones",
        _contract("stone", 0.54),
    )
    _hull("Irregular slab", 0.43, 0.32, "stone", yscale=0.73, sides=10, top_scale=0.52)
    forest.orb("Draped moss cap", (0.02, -0.03, 0.34), (0.30, 0.24, 0.07), "moss", 1)
    forest.orb("Moss spill", (0.18, -0.22, 0.27), (0.16, 0.09, 0.06), "moss", 1)
    forest.orb(
        "Loose slab stone", (-0.38, -0.08, 0.06), (0.08, 0.07, 0.06), "stone_light", 1
    )
    forest.orb(
        "Loose slab stone", (0.34, 0.18, 0.05), (0.07, 0.06, 0.05), "stone_light", 1
    )


def _ore_band(prefix, points):
    for i, (x, y, z, radius, length, tilt, azimuth, mat) in enumerate(points):
        _angled_crystal(
            f"{prefix} nugget {i}",
            (x, y, z),
            radius,
            length,
            tilt,
            azimuth,
            mat,
            sides=5,
        )


def world_ore_vein_a():
    asset(
        "world_ore_vein_a",
        "Tall boulder with a bold diagonal copper vein and leaning miner pick",
        _contract("ore", 0.84),
    )
    _hull("Ore boulder", 0.40, 0.62, "stone", yscale=0.78, sides=9, top_scale=0.48)
    # A raised diagonal band follows the visible upper face; copper dominates with a few iron points.
    _ore_band(
        "Ore A diagonal band",
        (
            (-0.29, -0.26, 0.30, 0.065, 0.13, 0.34, -1.45, "copper"),
            (-0.20, -0.27, 0.35, 0.070, 0.14, 0.30, -1.30, "copper"),
            (-0.10, -0.25, 0.40, 0.068, 0.15, 0.26, -1.16, "copper"),
            (0.00, -0.22, 0.44, 0.064, 0.14, 0.30, -0.98, "copper"),
            (0.10, -0.18, 0.47, 0.060, 0.13, 0.34, -0.82, "iron"),
            (0.20, -0.13, 0.45, 0.058, 0.12, 0.38, -0.66, "copper"),
            (0.27, -0.08, 0.41, 0.052, 0.11, 0.42, -0.52, "copper"),
        ),
    )
    forest.beam("Copper vein ridge", (-0.31, -0.27, 0.31), (0.25, -0.12, 0.44), 0.026, "copper")
    _pick(0.30, -0.38, 0.03, -0.04, height=0.34, head_length=0.34)


def world_ore_vein_b():
    asset(
        "world_ore_vein_b",
        "Split ore boulder with a vein running along the cleft and a leaning miner pick",
        _contract("ore", 0.84),
    )
    _hull(
        "Split ore left",
        0.31,
        0.62,
        "stone",
        yscale=0.82,
        center=(-0.14, 0.03),
        sides=8,
        top_scale=0.50,
    )
    _hull(
        "Split ore right",
        0.27,
        0.42,
        "stone",
        yscale=0.84,
        center=(0.19, 0.04),
        sides=8,
        top_scale=0.46,
    )
    # The split is the signature: chunky copper nuggets bridge it from the front face to the crown.
    _ore_band(
        "Ore B split band",
        (
            (-0.05, -0.25, 0.28, 0.060, 0.12, 0.40, -1.38, "copper"),
            (-0.02, -0.23, 0.33, 0.068, 0.14, 0.34, -1.22, "copper"),
            (0.00, -0.20, 0.38, 0.070, 0.15, 0.28, -1.05, "iron"),
            (0.03, -0.16, 0.43, 0.065, 0.14, 0.25, -0.88, "copper"),
            (0.08, -0.10, 0.46, 0.060, 0.13, 0.30, -0.70, "copper"),
            (0.20, 0.04, 0.39, 0.050, 0.11, 0.42, -0.35, "iron"),
        ),
    )
    forest.beam("Split vein ridge", (-0.06, -0.26, 0.29), (0.20, 0.03, 0.40), 0.025, "copper")
    _pick(-0.34, -0.38, 0.03, 0.08, height=0.35, head_length=0.34)


def world_gem_vein():
    asset(
        "world_gem_vein",
        "Irregular slate outcrop with chunky gems bursting from top and sides",
        _contract("gem", 0.99),
    )
    _hull(
        "Faceted slate outcrop",
        0.44,
        0.39,
        "slate",
        yscale=0.75,
        sides=9,
        top_scale=0.48,
    )
    forest.orb("Stone rim rubble", (-0.26, -0.02, 0.22), (0.15, 0.12, 0.10), "stone", 1)
    forest.orb("Stone rim rubble", (0.23, 0.12, 0.21), (0.14, 0.10, 0.08), "stone", 1)
    crystals = (
        (-0.22, -0.06, 0.37, 0.075, 0.38, 0.28, -2.5),
        (-0.08, 0.08, 0.41, 0.12, 0.48, 0.18, -1.4),
        (0.08, -0.02, 0.39, 0.085, 0.36, 0.42, -0.35),
        (0.22, 0.08, 0.37, 0.07, 0.30, 0.34, 0.65),
        (0.28, -0.10, 0.31, 0.06, 0.27, 0.52, 1.15),
        (-0.04, -0.18, 0.32, 0.065, 0.32, 0.45, -2.1),
    )
    for i, (x, y, z, radius, length, tilt, azimuth) in enumerate(crystals):
        _angled_crystal(
            "Gem crystal %d" % i, (x, y, z), radius, length, tilt, azimuth, "gem"
        )


def world_clay_bank():
    asset(
        "world_clay_bank",
        "Low lumpy terracotta clay bank with stepped spade cuts, puddle and bricks",
        _contract("clay", 0.45),
    )
    _hull(
        "Lumpy clay bank",
        0.45,
        0.28,
        "terracotta",
        yscale=0.76,
        sides=9,
        top_scale=0.50,
    )
    for i, (x, z, w) in enumerate(
        ((-0.22, 0.10, 0.30), (-0.05, 0.16, 0.25), (0.12, 0.22, 0.19))
    ):
        forest.box(
            "Spade ledge", (x, -0.37, z), (w, 0.10, 0.045), "terracotta", edge=0.008
        )
    forest.mesh(
        "Flat clay puddle",
        [
            (-0.34, -0.48, -0.015),
            (0.05, -0.49, -0.015),
            (0.29, -0.39, -0.015),
            (0.16, -0.28, -0.015),
            (-0.23, -0.30, -0.015),
            (-0.34, -0.48, 0.012),
            (0.05, -0.49, 0.012),
            (0.29, -0.39, 0.012),
            (0.16, -0.28, 0.012),
            (-0.23, -0.30, 0.012),
        ],
        [
            (0, 1, 2, 3, 4),
            (9, 8, 7, 6, 5),
            (0, 5, 6, 1),
            (1, 6, 7, 2),
            (2, 7, 8, 3),
            (3, 8, 9, 4),
            (4, 9, 5, 0),
        ],
        "water",
    )
    for i in range(2):
        forest.box(
            "Dry clay brick",
            (0.23 + i * 0.15, -0.27, 0.045),
            (0.14, 0.10, 0.09),
            "terracotta",
            edge=0.012,
            rotation=(0, 0, (i - 0.5) * 0.14),
        )


def _asymmetric_dune(name):
    """Closed dune with a long windward slope and a steeper lee shoulder."""
    base = [
        (-0.44, -0.28), (-0.12, -0.37), (0.28, -0.31), (0.46, -0.12),
        (0.42, 0.22), (0.16, 0.34), (-0.20, 0.32), (-0.44, 0.12),
    ]
    shoulder = [
        (-0.34, -0.17), (-0.12, -0.23), (0.23, -0.20), (0.34, -0.07),
        (0.27, 0.14), (0.08, 0.23), (-0.14, 0.20), (-0.31, 0.06),
    ]
    heights = (0.20, 0.19, 0.24, 0.33, 0.39, 0.35, 0.28, 0.22)
    verts = [(x, y, -0.02) for x, y in base] + [
        (x, y, z) for (x, y), z in zip(shoulder, heights, strict=True)
    ] + [(0.0, -0.01, 0.405)]
    faces = [tuple(reversed(range(8)))]
    for i in range(8):
        j = (i + 1) % 8
        faces.append((i, j, 8 + j, 8 + i))
        faces.append((8 + i, 8 + j, 16))
    return forest.mesh(name, verts, faces, "straw")


def _dune_surface(dune, x, y):
    hit, point, *_ = dune.ray_cast(forest.Vector((x, y, 1.0)), forest.Vector((0, 0, -1)))
    if not hit:
        raise AssertionError(("dune detail outside surface", x, y))
    return point.z


def _triangular_ripple(name, dune, points, width=0.038, rise=0.012):
    """Closed straw ridge with a triangular cross-section draped over the dune."""
    verts = []
    for i, (x, y, z) in enumerate(points):
        if i == 0:
            dx, dy = points[1][0] - x, points[1][1] - y
        elif i == len(points) - 1:
            dx, dy = x - points[i - 1][0], y - points[i - 1][1]
        else:
            dx, dy = points[i + 1][0] - points[i - 1][0], points[i + 1][1] - points[i - 1][1]
        length = math.sqrt(dx * dx + dy * dy) or 1.0
        px, py = -dy / length * width * 0.5, dx / length * width * 0.5
        verts.extend((
            (x + px, y + py, _dune_surface(dune, x + px, y + py) - 0.002),
            (x, y, _dune_surface(dune, x, y) + rise),
            (x - px, y - py, _dune_surface(dune, x - px, y - py) - 0.002),
        ))
    n = len(points)
    faces = []
    for i in range(n - 1):
        a, b = i * 3, (i + 1) * 3
        faces.extend(((a, b, b + 1), (a, b + 1, a + 1), (a + 1, b + 1, b + 2), (a + 1, b + 2, a + 2), (a + 2, b + 2, b, a)))
    faces.extend(((0, 1, 2), (3 * (n - 1) + 2, 3 * (n - 1) + 1, 3 * (n - 1))))
    forest.mesh(name, verts, faces, "straw")


def _fan_shell(name, center, radius=0.04):
    """Closed scallop fan with a shallow raised shell face."""
    cx, cy, cz = center
    count = 7
    arc = [
        (cx + math.cos(-math.pi * 0.88 + i * math.pi * 0.76 / (count - 1)) * radius,
         cy + math.sin(-math.pi * 0.88 + i * math.pi * 0.76 / (count - 1)) * radius, cz)
        for i in range(count)
    ]
    verts = arc + [(cx, cy, cz + 0.012)] + [(x, y, z - 0.008) for x, y, z in arc] + [(cx, cy, cz - 0.008)]
    top = count
    bottom = count + 1
    faces = []
    for i in range(count - 1):
        faces.append((i, i + 1, top))
        faces.append((bottom + i + 1, bottom + i, bottom + count))
        faces.append((i, bottom + i, bottom + i + 1, i + 1))
    faces.extend(((0, top, bottom + count, bottom), (count - 1, bottom + count - 1, bottom + count, top)))
    forest.mesh(name, verts, faces, "endgrain")


def world_sand_bank():
    asset(
        "world_sand_bank",
        "Asymmetric rippled sand dune with shell and rooted edge reeds",
        _contract("sand", 0.56),
    )
    dune = _asymmetric_dune("Asymmetric sand dune")
    for i in range(5):
        y = -0.27 + i * 0.085
        half_span = 0.22 + (0.035 if i in (1, 2, 3) else 0)
        points = [(x, y + 0.025 * (x / half_span) ** 2, 0)
                  for x in (-half_span, -half_span * 0.5, 0, half_span * 0.5, half_span)]
        _triangular_ripple("Draped sand ripple", dune, points)
    shell_x, shell_y = -0.10, 0.16
    _fan_shell("Scallop shell", (shell_x, shell_y, _dune_surface(dune, shell_x, shell_y) + 0.007), 0.04)
    # Bases sit in the dune flank; the blades lean outward over the sand edge.
    for x, y, z, h, az in (
        (0.28, 0.16, 0.16, 0.26, 0.45),
        (0.34, 0.10, 0.13, 0.23, 0.25),
        (0.20, 0.23, 0.18, 0.22, 1.05),
        (-0.02, 0.27, 0.16, 0.20, 1.35),
    ):
        _leaf_blade("Rooted sand reed", (x, y, z), 0.15, 0.018, h, az, "moss")


def world_flax():
    asset(
        "world_flax",
        "Dense ripe golden fibre crop with blue flowers and blue-green lower leaves",
        _contract("fibre", 0.90),
    )
    for i in range(14):
        a = i * 2.399 + 0.2
        base_r = 0.045 + (i % 5) * 0.018
        top_r = 0.13 + (i % 6) * 0.024
        bx, by = math.cos(a) * base_r, math.sin(a) * base_r
        h = 0.70 + (i % 4) * 0.035
        lean = 0.02 * math.sin(i * 1.7)
        tx, ty = math.cos(a) * top_r + lean, math.sin(a) * top_r
        stalk_length = math.sqrt((tx - bx) ** 2 + (ty - by) ** 2)
        _leaf_blade("Flax straw stalk", (bx, by, 0.012), stalk_length, 0.014, h - 0.012, a, "straw")
        if i % 4 == 0:
            _tiny_leaf(
                "Low flax herb leaf", (bx, by, 0.16), 0.13, 0.022, 0.045, a + 1.2, "herb"
            )
        _bud("Blue flax star blossom", (tx, ty, h), 0.027, "cloth")
    for i in range(3):
        _tiny_leaf(
            "Loose flax straw tuft", (0.0, 0.0, 0.025), 0.16, 0.025, 0.04, i * 2.1, "straw"
        )


def world_wild_herbs():
    asset(
        "world_wild_herbs",
        "Loose ground rosette of broad leaves with pale purple flower sprigs",
        _contract("herbs", 0.57),
    )
    # Seven broad spoon leaves overlap into a green rosette directly on the ground.
    for i in range(7):
        a = i * math.tau / 7 + 0.2
        _cupped_spoon_leaf(
            "Rosette leaf",
            (math.cos(a) * 0.07, math.sin(a) * 0.07, 0.018),
            0.24 + (i % 3) * 0.012,
            0.092 + (i % 2) * 0.010,
            0.10 + (i % 2) * 0.035,
            a,
            "herb" if i % 2 == 0 else "leaf_light",
        )
    for i in range(4):
        a = i * math.tau / 4 + 0.5
        _spoon_leaf(
            "Small centre herb leaf",
            (math.cos(a) * 0.018, math.sin(a) * 0.018, 0.022),
            0.13 + (i % 2) * 0.012,
            0.055,
            0.065,
            a + 0.4,
            "herb",
        )
    for i in range(5):
        a = i * math.tau / 5 + 0.4
        x, y = math.cos(a) * (0.07 + i * 0.018), math.sin(a) * (0.07 + i * 0.018)
        h = 0.32 + (i % 2) * 0.06
        forest.cylinder("Wild herb flower stem", (x, y, h * 0.5), 0.014, h, "herb", 5, top=0.010)
        _bud("Wild flower blossom", (x + 0.020, y, h), 0.060, "flower")


def world_berry_bush_b():
    asset(
        "world_berry_bush_b",
        "Tall drooping berry bush with visible fruit clusters",
        _contract("food", 0.79),
    )
    _mound("Berry soil", 0.36, 0.07, "leaf", 0.8)
    for i, (x, y, z) in enumerate(
        ((-0.18, 0, 0.33), (0.17, 0.03, 0.36), (-0.03, 0.03, 0.46), (0.0, -0.12, 0.33))
    ):
        forest.beam("Drooping branch", (0, 0, 0.16), (x, y, z), 0.045, "leaf")
        forest.orb("Bush foliage", (x, y, z + 0.11), (0.23, 0.18, 0.20), "leaf", 1)
        _leaf_cluster(
            "Drooping leaf tip",
            (x * 1.12, y * 1.12, z + 0.02),
            (0.11, 0.08, 0.07),
            "leaf_light",
        )
    berries = (
        (-0.26, -0.10, 0.45),
        (-0.13, 0.11, 0.54),
        (0.03, -0.17, 0.42),
        (0.20, -0.08, 0.49),
        (0.25, 0.10, 0.57),
        (-0.02, 0.09, 0.67),
        (-0.30, 0.02, 0.55),
        (0.14, -0.17, 0.62),
        (0.30, 0.02, 0.46),
    )
    for i, (x, y, z) in enumerate(berries):
        for j in range(2 + i % 2):
            _bud(
                "Berry fruit",
                (x + (j - 1) * 0.045, y + (j % 2 - 0.5) * 0.035, z),
                0.05,
                "berry",
            )


def _round_lobe(name, center, radius, height, mat):
    jitter = rng().uniform(0.97, 1.03)
    return forest.orb(name, center, (radius * jitter, radius * 0.95, height * 0.5), mat, 1)


def _sapling_stem(name, mat, crown_mat, crown_kind):
    _mound("Fresh soil mound", 0.18, 0.07, "soil", 0.84)
    forest.cylinder(
        "Curved sapling stem lower",
        (0.018, 0, 0.175),
        0.04,
        0.30,
        mat,
        7,
        rotation=(0, 0.12, 0),
        top=0.028,
    )
    forest.cylinder(
        "Curved sapling stem upper",
        (0.052, 0.008, 0.415),
        0.035,
        0.22,
        mat,
        7,
        rotation=(0, 0.14, 0),
        top=0.024,
    )
    forest.cylinder(
        "Support stake", (0.20, 0.05, 0.325), 0.025, 0.57, "bark", 6, top=0.019
    )
    forest.box(
        "Plant tie band", (0.12, 0.027, 0.43), (0.14, 0.035, 0.035), "soil", edge=0 if crown_kind == "broad" else 0.008
    )
    if crown_kind == "broad":
        # Each tier is a small cluster of closed, near-round leaf lobes rather than flat fronds.
        tufts = (
            ((-0.070, 0.000, 0.35), (0.035, 0.018, 0.35)),
            ((0.105, 0.015, 0.49), (0.205, -0.005, 0.49)),
            ((-0.045, 0.005, 0.61), (0.075, 0.020, 0.61)),
        )
        radii = (0.075, 0.080, 0.090)
        heights = (0.135, 0.145, 0.16)
        for tier, lobes in enumerate(tufts):
            for pos in lobes:
                _round_lobe("Broadleaf rounded lobe", pos, radii[tier], heights[tier], crown_mat)
    else:
        for z, r in ((0.39, 0.24), (0.49, 0.20), (0.59, 0.15)):
            _leaf_cluster("Pine tuft", (0, 0, z), (r, r * 0.72, 0.11), crown_mat)


def world_tree_sapling():
    asset(
        "world_tree_sapling",
        "Young broadleaf sapling with irregular soil mound, stake and tie",
        _contract(None, 0.72),
    )
    _sapling_stem("Broadleaf", "bark", "leaf_light", "broad")


def world_pine_sapling():
    asset(
        "world_pine_sapling",
        "Young pine tuft with irregular soil mound, stake and tie",
        _contract(None, 0.72),
    )
    _sapling_stem("Pine", "bark", "pine", "pine")


def _stump_base(with_axe=False):
    _mound("Stump roots", 0.36, 0.07, "bark", 0.86)
    forest.cylinder("Cut stump", (0, 0, 0.155), 0.275, 0.29, "bark", 10, top=0.25)
    forest.cylinder(
        "Pale cut top", (0, 0, 0.305), 0.24, 0.025, "endgrain", 10, top=0.23
    )
    for radius in (0.16, 0.09):
        _ring("Growth ring", radius, radius - 0.012, 0.323, "bark")
    for x, y in ((-0.36, -0.08), (0.28, -0.13), (0.15, 0.23)):
        forest.orb(
            "Pale wood chip", (x, y, 0.025), (0.055, 0.035, 0.025), "endgrain", 1
        )
    if with_axe:
        # The haft exits the rings and rises beyond the rim; the enlarged iron head is attached at its tip.
        forest.beam("Woodcutter axe haft", (-0.03, -0.01, 0.23), (0.25, -0.08, 0.53), 0.04, "wood")
        cx, cy, cz = 0.25, -0.08, 0.53
        points = [
            (-0.095, -0.075), (-0.080, 0.055), (-0.020, 0.085),
            (0.085, 0.055), (0.095, -0.025), (0.050, -0.085), (-0.050, -0.095),
        ]
        verts = [(cx + x, cy - 0.055, cz + z) for x, z in points] + [(cx + x, cy + 0.055, cz + z) for x, z in points]
        faces = [tuple(reversed(range(7))), tuple(range(7, 14))]
        faces += [(i, (i + 1) % 7, 7 + (i + 1) % 7, 7 + i) for i in range(7)]
        forest.mesh("Wedge axe blade beyond stump rim", verts, faces, "iron")


def world_stump():
    asset(
        "world_stump",
        "Natural cut stump with growth rings, roots and fresh chips",
        _contract(None, 0.36),
    )
    _stump_base(False)


def world_stump_axe():
    asset(
        "world_stump_axe",
        "Cut stump with a recognisable woodcutter axe buried in its rings",
        _contract(None, 0.65, "axe"),
    )
    _stump_base(True)


def _checker(name, uv, lo, hi, size, record, objects):
    contract = record.get("contract", {})
    footprint = contract.get("footprint_m", [1, 1])
    if size[0] > footprint[0] + 1e-4 or size[2] > footprint[1] + 1e-4:
        raise AssertionError((name, "footprint", size))
    if (
        max(abs(lo[0]), abs(hi[0])) > footprint[0] / 2 + 1e-4
        or max(abs(lo[2]), abs(hi[2])) > footprint[1] / 2 + 1e-4
    ):
        raise AssertionError((name, "centred footprint"))
    if not (-0.035 <= lo[1] <= 0.001):
        raise AssertionError((name, "ground contact", lo[1]))
    max_h = contract.get("max_height_m", 1.0)
    minima = {
        "world_rock_a": 0.5,
        "world_rock_b": 0.5,
        "world_rock_c": 0.3,
        "world_ore_vein_a": 0.6,
        "world_ore_vein_b": 0.6,
        "world_gem_vein": 0.45,
        "world_clay_bank": 0.15,
        "world_sand_bank": 0.15,
        "world_flax": 0.7,
        "world_wild_herbs": 0.2,
        "world_berry_bush_b": 0.62,
        "world_tree_sapling": 0.52,
        "world_pine_sapling": 0.52,
        "world_stump": 0.25,
        "world_stump_axe": 0.25,
    }
    if size[1] < minima.get(name, 0.0) - 1e-4 or size[1] > max_h + 1e-4:
        raise AssertionError((name, "height", size[1], minima.get(name, 0.0), max_h))
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
    "world_rock_a": world_rock_a,
    "world_rock_b": world_rock_b,
    "world_rock_c": world_rock_c,
    "world_ore_vein_a": world_ore_vein_a,
    "world_ore_vein_b": world_ore_vein_b,
    "world_gem_vein": world_gem_vein,
    "world_clay_bank": world_clay_bank,
    "world_sand_bank": world_sand_bank,
    "world_flax": world_flax,
    "world_wild_herbs": world_wild_herbs,
    "world_berry_bush_b": world_berry_bush_b,
    "world_tree_sapling": world_tree_sapling,
    "world_pine_sapling": world_pine_sapling,
    "world_stump": world_stump,
    "world_stump_axe": world_stump_axe,
}

CONTRACT_CHECKERS = {
    "world_rock_a": _checker,
    "world_rock_b": _checker,
    "world_rock_c": _checker,
    "world_ore_vein_a": _checker,
    "world_ore_vein_b": _checker,
    "world_gem_vein": _checker,
    "world_clay_bank": _checker,
    "world_sand_bank": _checker,
    "world_flax": _checker,
    "world_wild_herbs": _checker,
    "world_berry_bush_b": _checker,
    "world_tree_sapling": _checker,
    "world_pine_sapling": _checker,
    "world_stump_axe": _checker,
    "world_stump": _checker,
}
