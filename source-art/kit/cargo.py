"""Original geometry, one builder per asset."""

from .helpers import (
    axe,
    beam,
    box,
    cylinder,
    finish_asset,
    flower,
    log,
    math,
    mesh,
    new_asset,
    orb,
    sack,
    torus,
)


def cargo_food():
    name = "cargo_food"
    new_asset(name, "Distinct carried food bundle, ground pivot for shoulder attachment")
    mat = "berry"
    sack((0, 0, 0), mat, 0.23, 0.45)
    finish_asset(name)


def cargo_fish():
    name = "cargo_fish"
    new_asset(name, "Distinct carried fish bundle, ground pivot for shoulder attachment")
    orb("Silver fish body", (0, 0, 0.12), (0.14, 0.31, 0.11), "water", 2)
    mesh("Fish tail fin", [(-0.16, 0.45, 0.11), (0.16, 0.45, 0.11), (0, 0.24, 0.13)], [(0, 2, 1)], "teal")
    orb("Fish eye", (0.095, -0.19, 0.17), (0.021,) * 3, "eye")
    finish_asset(name)


def cargo_blessings():
    name = "cargo_blessings"
    new_asset(name, "Distinct carried blessings bundle, ground pivot for shoulder attachment")
    for i in range(3):
        flower(((i - 1) * 0.12, 0, 0), "gold", 0.4)
    torus("Bundle tie", (0, 0, 0.16), 0.15, 0.023, "straw")
    finish_asset(name)


def cargo_materials():
    name = "cargo_materials"
    new_asset(name, "Distinct carried materials bundle, ground pivot for shoulder attachment")
    for i in range(3):
        log(((i % 2 - 0.5) * 0.16, 0, 0.085 + i // 2 * 0.14), 0.65, 0.079)
    for y in (-0.18, 0.18):
        box("Binding strap", (0, y, 0.17), (0.38, 0.035, 0.055), "straw", edge=0.009)
    finish_asset(name)


def cargo_stone():
    name = "cargo_stone"
    new_asset(name, "Distinct carried stone bundle, ground pivot for shoulder attachment")
    mat = "stone"
    for i in range(3):
        pos = ((i % 2 - 0.5) * 0.21, 0, 0.1 + i // 2 * 0.16)
        orb("Raw material", pos, (0.15, 0.2, 0.13), mat, 1)
    finish_asset(name)


def cargo_refined():
    name = "cargo_refined"
    new_asset(name, "Distinct carried refined bundle, ground pivot for shoulder attachment")
    mat = "teal"
    sack((0, 0, 0), mat, 0.23, 0.45)
    finish_asset(name)


def cargo_logs():
    name = "cargo_logs"
    new_asset(name, "Distinct carried logs bundle, ground pivot for shoulder attachment")
    for i in range(3):
        log(((i % 2 - 0.5) * 0.16, 0, 0.085 + i // 2 * 0.14), 0.65, 0.079)
    for y in (-0.18, 0.18):
        box("Binding strap", (0, y, 0.17), (0.38, 0.035, 0.055), "straw", edge=0.009)
    finish_asset(name)


def cargo_lumber():
    name = "cargo_lumber"
    new_asset(name, "Distinct carried lumber bundle, ground pivot for shoulder attachment")
    for i in range(3):
        box(
            "Bound boards",
            (0, 0, 0.035 + i * 0.075),
            (0.33, 0.63, 0.055),
            "wood",
        )
    for y in (-0.18, 0.18):
        box("Binding strap", (0, y, 0.17), (0.38, 0.035, 0.055), "straw", edge=0.009)
    finish_asset(name)


def cargo_planks():
    name = "cargo_planks"
    new_asset(name, "Distinct carried planks bundle, ground pivot for shoulder attachment")
    for i in range(3):
        box(
            "Bound boards",
            (0, 0, 0.035 + i * 0.075),
            (0.44, 0.63, 0.055),
            "endgrain",
        )
    for y in (-0.18, 0.18):
        box("Binding strap", (0, y, 0.17), (0.38, 0.035, 0.055), "straw", edge=0.009)
    finish_asset(name)


def cargo_blocks():
    name = "cargo_blocks"
    new_asset(name, "Distinct carried blocks bundle, ground pivot for shoulder attachment")
    mat = "stone_light"
    for i in range(3):
        pos = ((i % 2 - 0.5) * 0.21, 0, 0.1 + i // 2 * 0.16)
        box("Cut material", pos, (0.2, 0.38, 0.14), mat)
    finish_asset(name)


def cargo_tools():
    name = "cargo_tools"
    new_asset(name, "Distinct carried tools bundle, ground pivot for shoulder attachment")
    axe((-0.12, 0, 0))
    finish_asset(name)


def cargo_water():
    name = "cargo_water"
    new_asset(name, "Distinct carried water bundle, ground pivot for shoulder attachment")
    mat = "water"
    cylinder("Cargo jar", (0, 0, 0.22), 0.18, 0.38, mat, 10, top=0.145)
    cylinder("Jar stopper", (0, 0, 0.44), 0.1, 0.09, "cream", 8)
    torus("Jar handle", (0.17, 0, 0.29), 0.09, 0.025, "wood", rotation=(math.pi / 2, 0, 0))
    finish_asset(name)


def cargo_catnip():
    name = "cargo_catnip"
    new_asset(name, "Distinct carried catnip bundle, ground pivot for shoulder attachment")
    for i in range(3):
        flower(((i - 1) * 0.12, 0, 0), "flower", 0.4)
    torus("Bundle tie", (0, 0, 0.16), 0.15, 0.023, "straw")
    finish_asset(name)


def cargo_grain():
    name = "cargo_grain"
    new_asset(name, "Distinct carried grain bundle, ground pivot for shoulder attachment")
    mat = "grain"
    sack((0, 0, 0), mat, 0.23, 0.45)
    finish_asset(name)


def cargo_flour():
    name = "cargo_flour"
    new_asset(name, "Distinct carried flour bundle, ground pivot for shoulder attachment")
    mat = "cream"
    sack((0, 0, 0), mat, 0.23, 0.45)
    finish_asset(name)


def cargo_preserves():
    name = "cargo_preserves"
    new_asset(name, "Distinct carried preserves bundle, ground pivot for shoulder attachment")
    mat = "berry"
    cylinder("Cargo jar", (0, 0, 0.22), 0.18, 0.38, mat, 10, top=0.145)
    cylinder("Jar stopper", (0, 0, 0.44), 0.1, 0.09, "cream", 8)
    torus("Jar handle", (0.17, 0, 0.29), 0.09, 0.025, "wood", rotation=(math.pi / 2, 0, 0))
    finish_asset(name)


def cargo_medicine():
    name = "cargo_medicine"
    new_asset(name, "Distinct carried medicine bundle, ground pivot for shoulder attachment")
    mat = "herb"
    cylinder("Cargo jar", (0, 0, 0.22), 0.18, 0.38, mat, 10, top=0.145)
    cylinder("Jar stopper", (0, 0, 0.44), 0.1, 0.09, "cream", 8)
    torus("Jar handle", (0.17, 0, 0.29), 0.09, 0.025, "wood", rotation=(math.pi / 2, 0, 0))
    finish_asset(name)


def cargo_brew():
    name = "cargo_brew"
    new_asset(name, "Distinct carried brew bundle, ground pivot for shoulder attachment")
    mat = "gold"
    cylinder("Cargo jar", (0, 0, 0.22), 0.18, 0.38, mat, 10, top=0.145)
    cylinder("Jar stopper", (0, 0, 0.44), 0.1, 0.09, "cream", 8)
    torus("Jar handle", (0.17, 0, 0.29), 0.09, 0.025, "wood", rotation=(math.pi / 2, 0, 0))
    finish_asset(name)


def cargo_herbs():
    name = "cargo_herbs"
    new_asset(name, "Distinct carried herbs bundle, ground pivot for shoulder attachment")
    for i in range(3):
        flower(((i - 1) * 0.12, 0, 0), "herb", 0.4)
    torus("Bundle tie", (0, 0, 0.16), 0.15, 0.023, "straw")
    finish_asset(name)


def cargo_hide():
    name = "cargo_hide"
    new_asset(name, "Distinct carried hide bundle, ground pivot for shoulder attachment")
    mat = "fur"
    for i in range(3):
        box("Folded goods", (0, 0, 0.05 + i * 0.07), (0.4, 0.4 - i * 0.04, 0.06), mat, edge=0.035)
    box("Folded goods strap", (0, 0, 0.245), (0.035, 0.42, 0.026), "cream", edge=0.005)
    finish_asset(name)


def cargo_leather():
    name = "cargo_leather"
    new_asset(name, "Distinct carried leather bundle, ground pivot for shoulder attachment")
    mat = "leather"
    for i in range(3):
        box("Folded goods", (0, 0, 0.05 + i * 0.07), (0.4, 0.4 - i * 0.04, 0.06), mat, edge=0.035)
    box("Folded goods strap", (0, 0, 0.245), (0.035, 0.42, 0.026), "cream", edge=0.005)
    finish_asset(name)


def cargo_fibre():
    name = "cargo_fibre"
    new_asset(name, "Distinct carried fibre bundle, ground pivot for shoulder attachment")
    mat = "straw"
    sack((0, 0, 0), mat, 0.23, 0.45)
    finish_asset(name)


def cargo_thread():
    name = "cargo_thread"
    new_asset(name, "Distinct carried thread bundle, ground pivot for shoulder attachment")
    for x in (-0.12, 0.12):
        cylinder("Thread spool", (x, 0, 0.18), 0.09, 0.3, "cream", 10)
        for z in (0.035, 0.325):
            cylinder("Spool flange", (x, 0, z), 0.12, 0.03, "wood", 10)
    finish_asset(name)


def cargo_cloth():
    name = "cargo_cloth"
    new_asset(name, "Distinct carried cloth bundle, ground pivot for shoulder attachment")
    mat = "cloth"
    for i in range(3):
        box("Folded goods", (0, 0, 0.05 + i * 0.07), (0.4, 0.4 - i * 0.04, 0.06), mat, edge=0.035)
    box("Folded goods strap", (0, 0, 0.245), (0.035, 0.42, 0.026), "cream", edge=0.005)
    finish_asset(name)


def cargo_bone():
    name = "cargo_bone"
    new_asset(name, "Distinct carried bone bundle, ground pivot for shoulder attachment")
    beam("Bone shaft", (0, -0.24, 0.08), (0, 0.24, 0.08), 0.11, "cream")
    for y in (-0.24, 0.24):
        for x in (-0.055, 0.055):
            orb("Bone joint", (x, y, 0.09), (0.08,) * 3, "cream", 1)
    finish_asset(name)


def cargo_ore():
    name = "cargo_ore"
    new_asset(name, "Distinct carried ore bundle, ground pivot for shoulder attachment")
    mat = "copper"
    for i in range(3):
        pos = ((i % 2 - 0.5) * 0.21, 0, 0.1 + i // 2 * 0.16)
        orb("Raw material", pos, (0.15, 0.2, 0.13), mat, 1)
    finish_asset(name)


def cargo_gem():
    name = "cargo_gem"
    new_asset(name, "Distinct carried gem bundle, ground pivot for shoulder attachment")
    mat = "gem"
    for i in range(3):
        pos = ((i % 2 - 0.5) * 0.21, 0, 0.1 + i // 2 * 0.16)
        orb("Raw material", pos, (0.15, 0.2, 0.13), mat, 1)
    finish_asset(name)


def cargo_clay():
    name = "cargo_clay"
    new_asset(name, "Distinct carried clay bundle, ground pivot for shoulder attachment")
    mat = "terracotta"
    for i in range(3):
        pos = ((i % 2 - 0.5) * 0.21, 0, 0.1 + i // 2 * 0.16)
        orb("Raw material", pos, (0.15, 0.2, 0.13), mat, 1)
    finish_asset(name)


def cargo_sand():
    name = "cargo_sand"
    new_asset(name, "Distinct carried sand bundle, ground pivot for shoulder attachment")
    mat = "straw"
    for i in range(3):
        pos = ((i % 2 - 0.5) * 0.21, 0, 0.1 + i // 2 * 0.16)
        orb("Raw material", pos, (0.15, 0.2, 0.13), mat, 1)
    finish_asset(name)


def cargo_metal():
    name = "cargo_metal"
    new_asset(name, "Distinct carried metal bundle, ground pivot for shoulder attachment")
    mat = "iron"
    for i in range(3):
        pos = ((i % 2 - 0.5) * 0.21, 0, 0.1 + i // 2 * 0.16)
        box("Cut material", pos, (0.2, 0.38, 0.14), mat)
    finish_asset(name)


def cargo_weapons():
    name = "cargo_weapons"
    new_asset(name, "Distinct carried weapons bundle, ground pivot for shoulder attachment")
    axe((-0.12, 0, 0))
    beam("Spear shaft", (0.2, 0.05, 0), (0.2, 0.05, 0.7), 0.04, "wood")
    cylinder("Spear tip", (0.2, 0.05, 0.79), 0.065, 0.23, "iron", 4, top=0)
    finish_asset(name)


def cargo_armor():
    name = "cargo_armor"
    new_asset(name, "Distinct carried armor bundle, ground pivot for shoulder attachment")
    orb("Cat breastplate", (0, 0, 0.19), (0.28, 0.22, 0.2), "iron", 2)
    torus("Leather harness", (0, 0, 0.17), 0.24, 0.035, "leather")
    finish_asset(name)


BUILDERS = {
    "cargo_food": cargo_food,
    "cargo_fish": cargo_fish,
    "cargo_blessings": cargo_blessings,
    "cargo_materials": cargo_materials,
    "cargo_stone": cargo_stone,
    "cargo_refined": cargo_refined,
    "cargo_logs": cargo_logs,
    "cargo_lumber": cargo_lumber,
    "cargo_planks": cargo_planks,
    "cargo_blocks": cargo_blocks,
    "cargo_tools": cargo_tools,
    "cargo_water": cargo_water,
    "cargo_catnip": cargo_catnip,
    "cargo_grain": cargo_grain,
    "cargo_flour": cargo_flour,
    "cargo_preserves": cargo_preserves,
    "cargo_medicine": cargo_medicine,
    "cargo_brew": cargo_brew,
    "cargo_herbs": cargo_herbs,
    "cargo_hide": cargo_hide,
    "cargo_leather": cargo_leather,
    "cargo_fibre": cargo_fibre,
    "cargo_thread": cargo_thread,
    "cargo_cloth": cargo_cloth,
    "cargo_bone": cargo_bone,
    "cargo_ore": cargo_ore,
    "cargo_gem": cargo_gem,
    "cargo_clay": cargo_clay,
    "cargo_sand": cargo_sand,
    "cargo_metal": cargo_metal,
    "cargo_weapons": cargo_weapons,
    "cargo_armor": cargo_armor,
}
