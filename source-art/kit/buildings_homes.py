"""Original geometry, one builder per asset."""

from .buildings_shared import bed_row, study_table, workplace_shell
from .helpers import (
    beam,
    book,
    bowl,
    box,
    crate,
    curve,
    cylinder,
    finish_asset,
    flower,
    garden,
    lantern,
    math,
    new_asset,
    orb,
    sack,
    table,
    torus,
    wheel,
)

from . import helpers as H


def den():
    name = "den"
    new_asset(name, "Cozy timber den with five quilted beds, pitched moss-green roof, cat gable and warm entry lantern")
    workplace_shell("leaf", roof_height=2.8, home=True)
    bed_row([(-0.92, 0.49), (0, 0.49), (0.92, 0.49), (-0.87, -0.63), (0.87, -0.63)])
    cylinder("Round cat gable", (0, -0.035, 2.3), 0.26, 0.07, "endgrain", 12, rotation=(math.pi / 2, 0, 0))
    for x in (-0.18, 0.18):
        cylinder("Gable cat ear", (x, -0.03, 2.56), 0.1, 0.22, "endgrain", 3, top=0)
        box("Cat gable eye", (x * 0.55, -0.078, 2.34), (0.045, 0.012, 0.075), "darkwood", 0.004)
    orb("Gable nose", (0, -0.08, 2.22), (0.038, 0.014, 0.028), "terracotta", 1)
    finish_asset(name)


def beds():
    name = "beds"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("teal")
    bed_row([(-0.86, 0.26), (0, 0.26), (0.86, 0.26)])
    finish_asset(name)


def nursery():
    name = "nursery"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("terracotta", home=True)
    bed_row([(-0.86, 0.26), (0, 0.26), (0.86, 0.26)])
    for i in range(3):
        orb("Kitten toy", (-0.4 + i * 0.4, -0.88, 0.37), (0.12,) * 3, ["berry", "gold", "cloth"][i], 2)
    curve("Hanging mobile", [(-0.65, 0.6, 1.9), (0, 0.6, 1.73), (0.65, 0.6, 1.9)], 0.028, "gold")
    finish_asset(name)


def elder_corner():
    name = "elder_corner"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("straw", home=True)
    bed_row([(-0.86, 0.26), (0, 0.26), (0.86, 0.26)])
    bowl((1.1, -0.83, 0.24), 0.23, "herb")
    book((-0.95, -0.82, 0.29), "terracotta")
    finish_asset(name)


def food_storage():
    name = "food_storage"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("teal")
    for x in (-0.87, 0, 0.87):
        crate((x, 0.3, 0.24), 0.64, True)
    for x in (-0.8, -0.25, 0.3):
        sack((x, -0.7, 0.24), "straw", 0.23, 0.54)
    finish_asset(name)


def mill():
    name = "mill"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("straw")
    cylinder("Lower millstone", (-0.4, 0, 0.59), 0.7, 0.68, "stone", 12)
    cylinder("Rotating upper millstone", (-0.4, 0, 1), 0.66, 0.16, "stone_light", 12)
    cylinder("Grain hopper", (-0.4, 0, 1.4), 0.16, 0.56, "wood", 4, top=0.42)
    cylinder("Hopper grain", (-0.4, 0, 1.68), 0.35, 0.015, "grain", 4)
    wheel((1.35, 0.16, 0.78), 0.7)
    for y in (-0.7, 0.05, 0.73):
        sack((0.72, y, 0.24), "cream", 0.25, 0.58)
    finish_asset(name)


def mouse_farm():
    name = "mouse_farm"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("teal")
    box("Mouse pen bedding", (0, 0, 0.27), (2.3, 1.65, 0.06), "straw")
    for x in (-1.1, 1.1):
        for i in range(6):
            box("Pen upright", (x, (i - 2.5) * 0.27, 0.63), (0.055, 0.055, 0.74), "wood")
        beam("Pen rail", (x, -0.8, 0.92), (x, 0.8, 0.92), 0.08, "wood")
    for i in range(4):
        x, y = (-0.65 + i % 2 * 0.9, -0.43 + i // 2 * 0.8)
        orb("Mouse", (x, y, 0.38), (0.16, 0.11, 0.1), "stone", 2)
        for yy in (-0.07, 0.07):
            orb("Mouse ear", (x - 0.1, y + yy, 0.48), (0.05, 0.04, 0.06), "nose", 1)
        curve("Mouse tail", [(x + 0.1, y, 0.34), (x + 0.22, y + 0.06, 0.32), (x + 0.32, y, 0.31)], 0.014, "nose")
    finish_asset(name)


def accounting_tent():
    name = "accounting_tent"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("cream")
    study_table()
    box("Counting board", (0.7, 0.16, 0.93), (0.39, 0.4, 0.06), "bark")
    for i in range(9):
        orb("Counter bead", (0.57 + i % 3 * 0.13, 0.02 + i // 3 * 0.13, 1.0), (0.041,) * 3, "gold")
    crate((-0.88, -0.74, 0.24), 0.43)
    finish_asset(name)


def research_hut():
    name = "research_hut"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("cloth")
    study_table()
    orb("Celestial globe", (0.85, 0.4, 1.19), (0.24,) * 3, "teal", 2)
    torus("Globe meridian", (0.85, 0.4, 1.19), 0.3, 0.025, "gold", rotation=(math.pi / 2, 0, 0))
    cylinder("Globe pedestal", (0.85, 0.4, 0.94), 0.14, 0.2, "wood", 8)
    for i in range(6):
        book((-0.9 + i * 0.15, 0.98, 1.16), "cloth" if i % 2 else "terracotta", 0.35, True)
    finish_asset(name)


def school():
    name = "school"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("teal_light")
    study_table()
    box("Chalkboard", (0, 0.9, 1.25), (1.72, 0.11, 0.69), "pine")
    for i in range(4):
        box("Chalk mark", (-0.53 + i * 0.34, 0.83, 1.27), (0.18, 0.015, 0.04), "cream", 0)
    for x in (-0.66, 0.66):
        table(x, -0.73, 0.7, 0.45, 0.55)
    finish_asset(name)


def shrine():
    name = "shrine"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    H.ASSETS[name]["description"] = (
        "Centered three-meter square sanctuary with nine paving bays, seated cat deity, halo and four open approaches"
    )
    box("Nine tile sanctuary base", (0, 0, 0.055), (2.96, 2.96, 0.11), "slate", 0.035)
    for x in (-0.99, 0, 0.99):
        for y in (-0.99, 0, 0.99):
            box(
                "Sanctuary paving bay",
                (x, y, 0.12),
                (0.96, 0.96, 0.08),
                "stone_light" if x == 0 or y == 0 else "stone",
                0.035,
            )
    for x in (-1.2, 1.2):
        for y in (-1.2, 1.2):
            box("Corner moss bed", (x, y, 0.17), (0.3, 0.3, 0.045), "moss", 0.03)
            flower((x, y, 0.19), "flower", 0.28)
    cylinder("Carved altar foot", (0, 0, 0.27), 0.65, 0.23, "stone_light", 8)
    cylinder("Altar pedestal", (0, 0, 0.57), 0.43, 0.48, "stone", 8, top=0.35)
    cylinder("Altar crown", (0, 0, 0.84), 0.49, 0.12, "stone_light", 8)
    orb("Seated deity haunches", (0, 0.11, 1.17), (0.4, 0.33, 0.35), "cream", 2)
    orb("Cat deity chest", (0, 0, 1.43), (0.31, 0.29, 0.47), "cream", 2)
    for x in (-0.16, 0.16):
        orb("Deity foreleg", (x, -0.22, 1.17), (0.1, 0.12, 0.34), "cream", 2)
        orb("Deity paw", (x, -0.3, 0.96), (0.13, 0.16, 0.09), "stone_light", 2)
    orb("Cat deity head", (0, -0.035, 1.89), (0.37, 0.32, 0.33), "cream", 2)
    for x in (-0.23, 0.23):
        cylinder("Deity pointed ear", (x, -0.025, 2.16), 0.17, 0.39, "cream", 3, top=0)
        curve(
            "Deity peaceful eye", [(x * 0.38, -0.33, 1.92), (x * 0.67, -0.356, 1.88), (x, -0.314, 1.9)], 0.018, "slate"
        )
    orb("Deity nose", (0, -0.365, 1.83), (0.065, 0.023, 0.045), "gold", 1)
    curve(
        "Curled statue tail",
        [(0.3, 0.13, 1.08), (0.53, 0.0, 0.97), (0.42, -0.34, 0.96), (0.06, -0.44, 0.98)],
        0.095,
        "cream",
    )
    torus("Sun halo", (0, 0.31, 1.98), 0.68, 0.047, "gold", rotation=(math.pi / 2, 0, 0))
    for i in range(9):
        a = math.pi * i / 8
        beam(
            "Halo sun ray",
            (math.cos(a) * 0.72, 0.31, 1.98 + math.sin(a) * 0.72),
            (math.cos(a) * 0.82, 0.31, 1.98 + math.sin(a) * 0.82),
            0.035,
            "gold",
        )
    for x in (-0.85, 0.85):
        bowl((x, -0.7, 0.17), 0.22, "berry")
        lantern((x, 0.72, 0.47))
    finish_asset(name)


def field():
    name = "field"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    garden("grain" if name == "field" else "herbs", 2.9, 2.3)
    beam("Scarecrow mast", (1.25, 0.9, 0.2), (1.25, 0.9, 1.55), 0.08, "wood")
    beam("Scarecrow arms", (0.85, 0.9, 1.18), (1.65, 0.9, 1.18), 0.06, "wood")
    orb("Scarecrow head", (1.25, 0.9, 1.45), (0.15,) * 3, "straw")
    finish_asset(name)


def herb_garden():
    name = "herb_garden"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    garden("grain" if name == "field" else "herbs", 2.9, 2.3)
    finish_asset(name)


def water_bowl():
    name = "water_bowl"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    for i in range(7):
        a = i * math.tau / 7
        orb("Spring stones", (math.cos(a) * 0.78, math.sin(a) * 0.78, 0.13), (0.26, 0.24, 0.18), "stone", 1)
    bowl((0, 0, 0), 0.72)
    beam("Water spout", (0.55, 0.4, 0.05), (0.55, 0.4, 0.8), 0.18, "wood")
    beam("Spout lip", (0.55, 0.4, 0.75), (0.22, 0.14, 0.75), 0.13, "wood")
    finish_asset(name)


BUILDERS = {
    "den": den,
    "beds": beds,
    "nursery": nursery,
    "elder_corner": elder_corner,
    "food_storage": food_storage,
    "mill": mill,
    "mouse_farm": mouse_farm,
    "accounting_tent": accounting_tent,
    "research_hut": research_hut,
    "school": school,
    "shrine": shrine,
    "field": field,
    "herb_garden": herb_garden,
    "water_bowl": water_bowl,
}
