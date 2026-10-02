"""World asset builders."""

from . import helpers as H
from .helpers import (
    PORTS,
    REGIONS,
    area,
    asset,
    clip_rect,
    forest,
    grass,
    math,
    prism,
    voronoi_cells,
)


def road_tile(name, kind):
    """Build one road using its own stream and the ordered ports for its kind."""
    ports = PORTS[kind]
    asset(
        name,
        "Irregular light paving with low soil and moss shoulders",
        {
            "grid_m": 1,
            "ports_unity": ports,
            "paving_width_m": 0.68,
            "paving_y_m": 0.045,
            "pivot_unity": [0, 0, 0],
            "naturalScale": True,
        },
    )
    forest.box("Low soil bed", (0, 0, -0.0145), (1, 1, 0.041), "earth", edge=0)
    regions = [(-0.34, -0.34, 0.34, 0.34)] + [REGIONS[p] for p in ports]
    interior = (
        -0.4 if "-X" in ports else -0.5,
        -0.4 if "+Z" in ports else -0.5,
        0.4 if "+X" in ports else 0.5,
        0.4 if "-Z" in ports else 0.5,
    )
    for index, poly in enumerate(voronoi_cells()):
        tint = ("stone_light", "stone_light", "stone", "stone_light", "earth")[index % 5]
        for region in regions:
            piece = clip_rect(clip_rect(poly, region), interior)
            if len(piece) >= 3 and area(piece) > 0.00015:
                prism("Paving stone %02d" % index, piece, 0.006, 0.045, tint)
    for port in ports:
        for j, (left, right) in enumerate(((-0.34, -0.118), (-0.11, 0.11), (0.118, 0.34))):
            poly = [
                (left, -0.5),
                (right, -0.5),
                (right, -0.43),
                (right - 0.016, -0.406),
                (left + 0.013, -0.408),
                (left, -0.429),
            ]
            if port == "-Z":
                poly = [(-x, -y) for x, y in poly]
            elif port == "+X":
                poly = [(-y, x) for x, y in poly]
            elif port == "-X":
                poly = [(y, -x) for x, y in poly]
            prism("Connection stone " + port + str(j), poly, 0.006, 0.045, "stone_light" if j != 1 else "stone")
    for i in range(16):
        x, y = (H.rng().uniform(-0.455, 0.455), H.rng().uniform(-0.455, 0.455))
        if any((a - 0.045 < x < c + 0.045 and b - 0.045 < y < d + 0.045 for a, b, c, d in regions)):
            continue
        poly = [(x + math.cos(t * math.tau / 5) * 0.028, y + math.sin(t * math.tau / 5) * 0.026) for t in range(5)]
        prism("Flat moss patch", poly, 0.005, 0.009, "leaf" if i % 2 else "moss")
        if i % 2:
            grass("Shoulder blades", (x, y, 0.007), 0.042, "leaf")


BUILDERS = {
    "world_road_straight": lambda: road_tile("world_road_straight", "straight"),
    "world_road_corner": lambda: road_tile("world_road_corner", "corner"),
    "world_road_junction": lambda: road_tile("world_road_junction", "junction"),
    "world_road_cross": lambda: road_tile("world_road_cross", "cross"),
    "world_road_end": lambda: road_tile("world_road_end", "end"),
}
