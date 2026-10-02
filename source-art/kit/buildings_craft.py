"""Original geometry, one builder per asset."""

from .buildings_shared import timber_bench, workplace_shell
from .helpers import (
    anvil,
    axe,
    beam,
    bowl,
    box,
    crate,
    cylinder,
    finish_asset,
    furnace,
    gear,
    log,
    math,
    mesh,
    new_asset,
    orb,
    table,
    torus,
    wheel,
)


def workshop():
    name = "workshop"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("teal")
    table(-0.25, -0.1, 1.8, 0.85)
    bowl((-0.7, 0, 0.85), 0.21, "herb")
    box("Mortar pestle", (-0.68, 0, 1.16), (0.075, 0.075, 0.35), "stone_light", rotation=(0.3, 0.25, 0))
    for i in range(3):
        cylinder("Preserve jar", (0.25 + i * 0.33, 0.55, 0.52), 0.14, 0.48, "berry", 10)
        cylinder("Jar lid", (0.25 + i * 0.33, 0.55, 0.77), 0.15, 0.045, "cream", 10)
    crate((0.98, -0.65, 0.24), 0.46)
    finish_asset(name)


def smithy():
    name = "smithy"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("teal")
    furnace(0.73, 0.44, name == "smelter")
    anvil((-0.65, -0.32, 0.24))
    axe((-1.13, 0.6, 0.25))
    finish_asset(name)


def smelter():
    name = "smelter"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("terracotta")
    furnace(0.73, 0.44, name == "smelter")
    for i in range(5):
        box("Metal ingot", (-0.8 + i % 2 * 0.5, -0.4 + i // 2 * 0.38, 0.38), (0.4, 0.24, 0.18), "copper", 0.04)
    crate((-0.82, 0.78, 0.24), 0.46)
    finish_asset(name)


def barracks():
    name = "barracks"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("terracotta")
    for x in (-0.9, 0.9):
        box("Weapon rack post", (x, 0.75, 0.93), (0.1, 0.1, 1.4), "wood")
    beam("Weapon rack rail", (-0.94, 0.75, 1.27), (0.94, 0.75, 1.27), 0.11, "wood")
    for i in range(4):
        axe((-0.85 + i * 0.45, 0.64, 0.45))
    for x in (-0.68, 0.58):
        cylinder("Training dummy post", (x, -0.5, 0.75), 0.09, 1, "wood", 8)
        orb("Training dummy", (x, -0.5, 1.05), (0.28, 0.23, 0.39), "straw", 2)
        torus("Dummy target", (x, -0.715, 1.09), 0.13, 0.025, "terracotta", rotation=(math.pi / 2, 0, 0))
    finish_asset(name)


def wood_cutter():
    name = "wood_cutter"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("teal")
    timber_bench()
    log((-0.82, -0.4, 0.55), 0.61, 0.34, True)
    axe((-0.85, -0.4, 0.68))
    for i in range(4):
        log((-0.7 + i * 0.38, 0.78, 0.44), 0.7, 0.15)
    finish_asset(name)


def stone_prep():
    name = "stone_prep"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("teal")
    table(0, -0.1, 1.85, 0.9, 0.65)
    for x in (-0.55, 0.05, 0.65):
        box("Dressed stone block", (x, -0.1, 0.92), (0.48, 0.52, 0.42), "stone_light", 0.04)
    for i in range(3):
        orb("Rough quarry stone", ((i - 1) * 0.65, 0.85, 0.48), (0.32, 0.3, 0.3), "stone", 1)
    beam("Chisel", (0.2, -0.6, 0.77), (0.55, -0.6, 0.77), 0.07, "iron")
    finish_asset(name)


def woodworking():
    name = "woodworking"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("teal")
    timber_bench()
    for i in range(4):
        box("Planed timber stack", (-0.5, 0.7, 0.35 + i * 0.12), (1.3, 0.5, 0.09), "endgrain")
    box("Chair seat", (0.88, 0.2, 0.56), (0.54, 0.6, 0.1), "wood")
    for xx in (0.67, 1.09):
        box("Chair upright", (xx, 0.43, 0.75), (0.065, 0.065, 1.03), "wood")
    box("Chair back", (0.88, 0.43, 1.15), (0.45, 0.065, 0.16), "endgrain")
    box("Wood plane", (-0.2, -0.18, 0.93), (0.32, 0.18, 0.17), "slate")
    finish_asset(name)


def clothier():
    name = "clothier"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("cloth")
    for x in (-0.87, 0.42):
        box("Loom upright", (x, 0.08, 1.0), (0.12, 0.15, 1.5), "wood")
    for z in (0.5, 1.61):
        beam("Loom beam", (-0.92, 0.08, z), (0.49, 0.08, z), 0.15, "endgrain")
    box("Woven cloth on loom", (-0.22, 0.05, 0.97), (1.05, 0.04, 0.62), "cloth")
    for i in range(10):
        beam("Warp thread", (-0.75 + i * 0.115, 0.025, 0.64), (-0.75 + i * 0.115, 0.025, 1.59), 0.013, "cream")
    wheel((0.97, -0.46, 0.75), 0.44)
    table(0.83, 0.62, 0.8, 0.5)
    for i in range(3):
        cylinder("Thread spool", (0.61 + i * 0.23, 0.62, 0.98), 0.087, 0.26, "cream", 8)
    finish_asset(name)


def tannery():
    name = "tannery"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("terracotta")
    for x in (-0.97, 0.38):
        box("Drying frame post", (x, 0.5, 1.01), (0.11, 0.11, 1.54), "wood")
    beam("Drying frame rail", (-1.02, 0.5, 1.69), (0.43, 0.5, 1.69), 0.1, "wood")
    mesh(
        "Stretched hide",
        [
            (-0.8, 0.48, 1.52),
            (-0.64, 0.48, 1.11),
            (-0.86, 0.48, 0.6),
            (-0.29, 0.48, 0.77),
            (0.2, 0.48, 0.6),
            (0.01, 0.48, 1.11),
            (0.2, 0.48, 1.52),
            (-0.3, 0.48, 1.4),
        ],
        [tuple(range(8))],
        "leather",
    )
    bowl((0.79, -0.28, 0.24), 0.43, "bark")
    table(-0.63, -0.7, 1.1, 0.42, 0.65)
    finish_asset(name)


def sawmill():
    name = "sawmill"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("teal")
    timber_bench()
    gear((0.22, -0.17, 0.93), 0.47)
    for y in (-0.65, 0.3):
        box("Saw feed rail", (0, y, 0.92), (2.4, 0.09, 0.11), "iron")
    for i in range(3):
        log((-0.98 + i * 0.35, 0.75, 0.42), 0.6, 0.14)
    wheel((1.35, 0.2, 0.69), 0.56)
    finish_asset(name)


def walls():
    name = "walls"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    for i in range(8):
        x = (i - 3.5) * 0.3
        cylinder("Palisade stake", (x, 0, 0.73), 0.16, 1.46, "wood", 6)
        cylinder("Sharpened stake", (x, 0, 1.58), 0.16, 0.25, "endgrain", 6, top=0)
    for z in (0.36, 1.12):
        beam("Palisade rail", (-1.2, -0.18, z), (1.2, -0.18, z), 0.12, "bark")
    finish_asset(name)


BUILDERS = {
    "workshop": workshop,
    "smithy": smithy,
    "smelter": smelter,
    "barracks": barracks,
    "wood_cutter": wood_cutter,
    "stone_prep": stone_prep,
    "woodworking": woodworking,
    "clothier": clothier,
    "tannery": tannery,
    "sawmill": sawmill,
    "walls": walls,
}
