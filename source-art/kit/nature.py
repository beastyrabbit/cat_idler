"""Original geometry, one builder per asset."""

from .helpers import (
    beam,
    cylinder,
    finish_asset,
    garden,
    log,
    math,
    new_asset,
    orb,
)

from . import helpers as H


def tree_oak():
    new_asset("tree_oak", "Broad layered oak crown, exposed branching trunk and flared roots")
    cylinder("Tapered trunk", (0, 0, 1.3), 0.24, 2.6, "bark", 9, top=0.15)
    for i in range(6):
        a = i * math.tau / 6 + 0.13
        beam("Root flare", (0, 0, 0.35), (math.cos(a) * 0.5, math.sin(a) * 0.5, 0.03), 0.15, "bark")
        beam("Fork branch", (0, 0, 1.4), (math.cos(a) * 0.92, math.sin(a) * 0.92, 2.52), 0.15, "bark")
        crown = (math.cos(a) * (0.8 + i % 2 * 0.15), math.sin(a) * 0.83, 2.55 + i % 3 * 0.2)
        orb("Broad leaf crown", crown, (0.86, 0.71, 0.62), "pine" if i == 4 else "leaf", 2)
        orb(
            "Sunlit leaf spray",
            (crown[0] + 0.13, crown[1] - 0.09, crown[2] + 0.35),
            (0.55, 0.5, 0.32),
            "leaf_light" if i % 2 == 0 else "moss",
            1,
        )
    orb("Oak crown top", (0.05, 0.1, 3.26), (0.87, 0.8, 0.54), "leaf_light", 2)
    for pos in ((-0.35, -0.19, 0.06), (0.23, 0.3, 0.06)):
        orb("Root moss", pos, (0.23, 0.18, 0.07), "leaf", 1)
    finish_asset("tree_oak")


def tree_pine():
    new_asset("tree_pine", "Tiered conifer with visible trunk and irregular bough silhouette")
    cylinder("Pine trunk", (0, 0, 1.65), 0.17, 3.3, "bark", 8, top=0.06)
    for i in range(5):
        radius = 1.06 - i * 0.19
        height = 1.19 + i * 0.47
        cylinder("Pine bough core", (0.035 * (i % 2), 0, height + 0.18), radius * 0.67, 0.98, "pine", 9, top=0.025)
        for j in range(5):
            a = math.tau * j / 5 + i * 0.73
            pos = (math.cos(a) * radius * 0.59, math.sin(a) * radius * 0.59, height - 0.15)
            orb(
                "Sweeping needle bough",
                pos,
                (radius * 0.53, radius * 0.45, 0.24),
                "leaf" if (i + j) % 4 == 0 else "pine",
                1,
            )
            beam("Pine branch", (0, 0, height - 0.2), (pos[0], pos[1], height - 0.15), 0.06, "bark")
    finish_asset("tree_pine")


def shrub():
    name = "shrub"
    mat = "leaf"
    new_asset(name, "Low branching shrub" if name == "shrub" else "Round berry bush with visible ripe pink fruit")
    for i in range(5):
        a = i * math.tau / 5
        pos = (math.cos(a) * 0.28, math.sin(a) * 0.28, 0.44 + 0.06 * (i % 2))
        beam("Bush stem", (0, 0, 0.02), pos, 0.06, "bark")
        orb("Bush foliage", pos, (0.4, 0.38, 0.4), mat, 2)
    finish_asset(name)


def berry_bush():
    name = "berry_bush"
    mat = "leaf_light"
    new_asset(name, "Low branching shrub" if name == "shrub" else "Round berry bush with visible ripe pink fruit")
    for i in range(5):
        a = i * math.tau / 5
        pos = (math.cos(a) * 0.28, math.sin(a) * 0.28, 0.44 + 0.06 * (i % 2))
        beam("Bush stem", (0, 0, 0.02), pos, 0.06, "bark")
        orb("Bush foliage", pos, (0.4, 0.38, 0.4), mat, 2)
        for j in range(3):
            orb("Ripe berry", (pos[0] + (j - 1) * 0.13, pos[1] - 0.27, pos[2] + 0.16), (0.075,) * 3, "berry", 1)
    finish_asset(name)


def stump():
    new_asset("stump", "Cut stump with exposed growth rings and roots")
    log((0, 0, 0.3), 0.6, 0.39, True)
    for i in range(4):
        a = i * math.tau / 4
        beam("Stump root", (0, 0, 0.2), (math.cos(a) * 0.6, math.sin(a) * 0.6, 0.04), 0.18, "bark")
    finish_asset("stump")


def rock():
    name = "rock"
    fleck = None
    new_asset(name, "Angular stone outcrop" + (" with visible " + fleck + " inclusions" if fleck else ""))
    orb("Main outcrop", (0, 0, 0.42), (0.74, 0.61, 0.6), "stone", 1)
    orb("Split rock", (0.48, 0.22, 0.25), (0.4, 0.36, 0.35), "slate", 1)
    finish_asset(name)


def ore_iron():
    name = "ore_iron"
    fleck = "copper"
    new_asset(name, "Angular stone outcrop" + (" with visible " + fleck + " inclusions" if fleck else ""))
    orb("Main outcrop", (0, 0, 0.42), (0.74, 0.61, 0.6), "stone", 1)
    orb("Split rock", (0.48, 0.22, 0.25), (0.4, 0.36, 0.35), "slate", 1)
    for i in range(5):
        orb("Ore inclusion", (-0.37 + i * 0.16, -0.43, 0.42 + i % 2 * 0.2), (0.15, 0.1, 0.16), fleck, 1)
    finish_asset(name)


def ore_coal():
    name = "ore_coal"
    fleck = "coal"
    new_asset(name, "Angular stone outcrop" + (" with visible " + fleck + " inclusions" if fleck else ""))
    orb("Main outcrop", (0, 0, 0.42), (0.74, 0.61, 0.6), "stone", 1)
    orb("Split rock", (0.48, 0.22, 0.25), (0.4, 0.36, 0.35), "slate", 1)
    for i in range(5):
        orb("Ore inclusion", (-0.37 + i * 0.16, -0.43, 0.42 + i % 2 * 0.2), (0.15, 0.1, 0.16), fleck, 1)
    finish_asset(name)


def grain_plot():
    name = "grain_plot"
    new_asset(name, "Raised grain bed with separate readable stems")
    garden("grain", 1.7, 1.3)
    finish_asset(name)


def herb_plot():
    name = "herb_plot"
    new_asset(name, "Raised herbs bed with separate readable stems")
    garden("herbs", 1.7, 1.3)
    finish_asset(name)


def catnip_plot():
    name = "catnip_plot"
    new_asset(name, "Raised catnip bed with separate readable stems")
    garden("catnip", 1.7, 1.3)
    finish_asset(name)


def reeds():
    new_asset("reeds", "Waterside reeds with brown seed heads")
    for _i in range(9):
        x, y, h = (H.rng().uniform(-0.4, 0.4), H.rng().uniform(-0.3, 0.3), H.rng().uniform(0.55, 1.1))
        beam("Reed stem", (x, y, 0), (x + 0.1, y, h), 0.028, "leaf")
        cylinder("Cattail seed head", (x + 0.1, y, h - 0.08), 0.06, 0.22, "bark", 7)
        beam("Reed blade", (x, y, 0.12), (x - 0.15, y + 0.1, h * 0.8), 0.045, "leaf_light")
    finish_asset("reeds")


BUILDERS = {
    "tree_oak": tree_oak,
    "tree_pine": tree_pine,
    "shrub": shrub,
    "berry_bush": berry_bush,
    "stump": stump,
    "rock": rock,
    "ore_iron": ore_iron,
    "ore_coal": ore_coal,
    "grain_plot": grain_plot,
    "herb_plot": herb_plot,
    "catnip_plot": catnip_plot,
    "reeds": reeds,
}
