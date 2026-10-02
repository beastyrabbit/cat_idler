"""Original geometry, one builder per asset."""

from .buildings_shared import workplace_shell
from .helpers import (
    beam,
    box,
    crate,
    cylinder,
    finish_asset,
    flower,
    lantern,
    log,
    math,
    merge_group,
    new_asset,
    orb,
    shingled_roof,
    table,
    torus,
    wheel,
)

from . import helpers as H


def stockpile():
    new_asset("stockpile", "Marked timber stockpile with open crates and stacked logs")
    for x in (-1.4, 1.4):
        box("Stockpile edge", (x, 0, 0.065), (0.12, 2.2, 0.13), "wood")
    for y in (-1.1, 1.1):
        box("Stockpile edge", (0, y, 0.065), (2.8, 0.12, 0.13), "wood")
    crate((-0.83, 0.42, 0), 0.63)
    crate((-0.05, 0.42, 0), 0.63)
    for i in range(3):
        log((0.75 + i % 2 * 0.34, 0.2, 0.19 + i // 2 * 0.29), 1.0, 0.17)
    finish_asset("stockpile")


def road():
    new_asset("road", "One meter continuous warm gravel road with broad inset paving stones")
    box("Compacted path", (0, 0, 0.015), (1, 1, 0.03), "earth", edge=0)
    for row in range(3):
        for col in range(3):
            x, y = ((col - 1) * 0.32, (row - 1) * 0.32)
            box(
                "Inset path paver",
                (x + H.rng().uniform(-0.012, 0.012), y + H.rng().uniform(-0.012, 0.012), 0.028),
                (H.rng().uniform(0.26, 0.3), H.rng().uniform(0.25, 0.3), 0.026),
                "stone_light" if (row + col) % 3 else "stone",
                edge=0.021,
                rotation=(0, 0, H.rng().uniform(-0.045, 0.045)),
            )
    finish_asset("road")


def fence():
    name = "fence"
    new_asset(name, "Meter-scale split rail " + name + "; X-axis rail joins centered posts without corner gaps")
    for x in (-0.5, 0.5) if name == "fence" else (0,):
        box("Fence post", (x, 0, 0.4), (0.16, 0.16, 0.8), "wood", 0.018)
        cylinder("Post cap", (x, 0, 0.84), 0.114, 0.08, "endgrain", 4, top=0)
        box("Fence foot", (x, 0, 0.06), (0.18, 0.18, 0.12), "slate", 0.023)
        for z in (0.29, 0.65):
            cylinder("Rail peg", (x, -0.087, z), 0.026, 0.018, "darkwood", 6, rotation=(math.pi / 2, 0, 0))
    for z in (0.29, 0.65):
        box("Continuous fence rail", (0, 0, z), (1.0, 0.085, 0.095), "wood", 0.012)
        box("Rail weathered grain", (0, -0.046, z + 0.012), (0.88, 0.009, 0.017), "endgrain", 0.003)
    finish_asset(name)


def fence_post():
    name = "fence_post"
    new_asset(name, "Meter-scale split rail " + name + "; X-axis rail joins centered posts without corner gaps")
    for x in (-0.5, 0.5) if name == "fence" else (0,):
        box("Fence post", (x, 0, 0.4), (0.16, 0.16, 0.8), "wood", 0.018)
        cylinder("Post cap", (x, 0, 0.84), 0.114, 0.08, "endgrain", 4, top=0)
        box("Fence foot", (x, 0, 0.06), (0.18, 0.18, 0.12), "slate", 0.023)
        for z in (0.29, 0.65):
            cylinder("Rail peg", (x, -0.087, z), 0.026, 0.018, "darkwood", 6, rotation=(math.pi / 2, 0, 0))
    finish_asset(name)


def fence_rail():
    name = "fence_rail"
    new_asset(name, "Meter-scale split rail " + name + "; X-axis rail joins centered posts without corner gaps")
    for z in (0.29, 0.65):
        box("Continuous fence rail", (0, 0, z), (1.0, 0.085, 0.095), "wood", 0.012)
        box("Rail weathered grain", (0, -0.046, z + 0.012), (0.88, 0.009, 0.017), "endgrain", 0.003)
    finish_asset(name)


def gate():
    new_asset(
        "gate", "Open two-meter village entrance, posts at X plus/minus one meter, overhead cat crest and lanterns"
    )
    for x in (-1, 1):
        box("Gate stone footing", (x, 0, 0.12), (0.24, 0.24, 0.24), "slate", 0.03)
        box("Gate upright", (x, 0, 0.83), (0.18, 0.18, 1.66), "bark", 0.022)
        beam("High gate knee brace", (x, 0, 1.25), (x * 0.68, 0, 1.66), 0.09, "wood")
        lantern((x, -0.15, 1.1))
    beam("Open gate lintel", (-1.09, 0, 1.72), (1.09, 0, 1.72), 0.17, "wood")
    shingled_roof(2.08, 0.24, -0.24, 1.79, 2.12, "leaf")
    cylinder("Gate crest", (0, -0.265, 1.86), 0.15, 0.04, "endgrain", 10, rotation=(math.pi / 2, 0, 0))
    for x in (-0.105, 0.105):
        cylinder("Gate crest ear", (x, -0.265, 2.01), 0.065, 0.15, "endgrain", 3, top=0)
    finish_asset("gate")


def rail():
    new_asset("rail", "One meter narrow gauge track section")
    for y in (-0.4, 0, 0.4):
        box("Rail sleeper", (0, y, 0.05), (1.2, 0.15, 0.1), "wood")
    for x in (-0.38, 0.38):
        box("Steel rail", (x, 0, 0.15), (0.08, 1.0, 0.12), "iron")
    finish_asset("rail")


def dock():
    new_asset("dock", "Timber pier with pilings, mooring posts and rope")
    for i in range(11):
        box("Dock plank", (0, (i - 5) * 0.28, 0.35), (1.85, 0.255, 0.13), "wood" if i % 3 else "endgrain")
    for x in (-0.79, 0.79):
        for y in (-1.25, 1.25):
            cylinder("Dock piling", (x, y, 0.36), 0.14, 0.72, "bark", 8)
            cylinder("Mooring cap", (x, y, 0.78), 0.18, 0.12, "endgrain", 8)
    torus("Coiled mooring rope", (0.4, 0.97, 0.455), 0.2, 0.025, "straw")
    finish_asset("dock")


def cart():
    new_asset("cart", "Four wheel wooden haul cart with open load bed and shafts")
    crate((0, 0, 0.5), 1.1)
    for x in (-0.69, 0.69):
        for y in (-0.37, 0.37):
            before = set(H.CURRENT.objects)
            wheel((0, 0, 0), 0.34)
            obj = merge_group("Cart wheel", [o for o in H.CURRENT.objects if o not in before])
            obj.rotation_euler[2] = math.pi / 2
            obj.location = (x, y, 0.34)
    for x in (-0.43, 0.43):
        beam("Pull shaft", (x, -0.4, 0.59), (x, -1.75, 0.41), 0.085, "wood")
    finish_asset("cart")


def boat():
    new_asset("boat", "Open clinker style river boat with benches and oars")
    outline = [
        (-0.61, -0.8),
        (-0.42, -1.28),
        (0, -1.6),
        (0.42, -1.28),
        (0.61, -0.8),
        (0.64, 0.9),
        (0.4, 1.3),
        (-0.4, 1.3),
        (-0.64, 0.9),
    ]
    for i, p in enumerate(outline):
        q = outline[(i + 1) % len(outline)]
        for z in (0.17, 0.33, 0.49):
            beam(
                "Clinker hull plank",
                (p[0] * (0.65 + z * 0.7), p[1], z),
                (q[0] * (0.65 + z * 0.7), q[1], z),
                0.17,
                "wood",
            )
        beam("Gunwale", (p[0], p[1], 0.62), (q[0], q[1], 0.62), 0.09, "endgrain")
    box("Boat floor", (0, -0.04, 0.12), (0.73, 2.18, 0.12), "darkwood")
    for y in (-0.64, 0.39):
        box("Rowing bench", (0, y, 0.47), (1.09, 0.28, 0.1), "endgrain")
    for side in (-1, 1):
        beam("Oar shaft", (side * 0.15, 0.1, 0.55), (side * 1.22, -0.48, 0.31), 0.05, "wood")
        box("Oar blade", (side * 1.24, -0.49, 0.31), (0.44, 0.19, 0.055), "endgrain", rotation=(0, 0, side * -0.4))
    finish_asset("boat")


def watchtower():
    new_asset("watchtower", "Raised guard platform with ladder and teal canopy")
    for x in (-0.64, 0.64):
        for y in (-0.64, 0.64):
            box("Tower leg", (x, y, 1.14), (0.2, 0.2, 2.28), "bark")
    box("Watch platform", (0, 0, 2.23), (1.58, 1.58, 0.16), "wood")
    for x in (-0.64, 0.64):
        box("Canopy post", (x, 0.64, 2.82), (0.11, 0.11, 1.2), "wood")
        beam("Tower crossbrace", (x, -0.64, 0.25), (x, 0.64, 1.95), 0.13, "wood")
    box("Tower canopy", (0, 0.42, 3.4), (1.8, 1.25, 0.14), "teal", rotation=(0.13, 0, 0))
    for x in (-0.29, 0.29):
        beam("Ladder rail", (x, -1.1, 0), (x, -0.77, 2.33), 0.07, "wood")
    for i in range(8):
        beam(
            "Ladder rung",
            (-0.29, -1.08 + i * 0.04, 0.17 + i * 0.28),
            (0.29, -1.08 + i * 0.04, 0.17 + i * 0.28),
            0.06,
            "endgrain",
        )
    finish_asset("watchtower")


def healing_tent():
    new_asset("healing_tent", "Open herb clinic with examination nest and medicine jars")
    workplace_shell("cream")
    table(-0.42, -0.12, 1.35, 0.9, 0.52)
    orb("Clinic cushion", (-0.42, -0.12, 0.64), (0.59, 0.37, 0.1), "teal_light", 2)
    table(0.95, 0.26, 0.62, 0.95)
    for y in (-0.05, 0.3, 0.63):
        cylinder("Medicine jar", (0.95, y, 1.02), 0.1, 0.33, "herb", 8)
        cylinder("Cork stopper", (0.95, y, 1.21), 0.065, 0.09, "straw", 8)
    for x in (-0.95, -0.55):
        flower((x, 0.88, 0.25), "flower", 0.45)
    finish_asset("healing_tent")


BUILDERS = {
    "stockpile": stockpile,
    "road": road,
    "fence": fence,
    "fence_post": fence_post,
    "fence_rail": fence_rail,
    "gate": gate,
    "rail": rail,
    "dock": dock,
    "cart": cart,
    "boat": boat,
    "watchtower": watchtower,
    "healing_tent": healing_tent,
}
