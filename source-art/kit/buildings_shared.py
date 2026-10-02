"""Building shapes for an asset already registered by its builder."""

from .helpers import beam, book, box, cylinder, lantern, math, orb, plank_floor, shingled_roof, table


def workplace_shell(roof="teal", w=3.1, d=2.6, wall="wood", roof_height=2.48, home=False):
    plank_floor(w, d)
    for x in (-w / 2 + 0.12, w / 2 - 0.12):
        for y in (-d / 2 + 0.1, d / 2 - 0.1):
            height = 1.71 if y > 0 else 1.05
            box("Chamfered post", (x, y, 0.23 + height / 2), (0.18, 0.18, height), "bark")
            box("Stone post shoe", (x, y, 0.36), (0.24, 0.24, 0.26), "stone_light", 0.035)
            cylinder("Oak peg", (x, y - 0.1, 0.75), 0.029, 0.024, "endgrain", 6, rotation=(math.pi / 2, 0, 0))
        for z in (0.36, 0.58):
            box("Cutaway side", (x, 0.18, z), (0.09, d - 0.48, 0.17), wall)
    for z in (0.35, 0.59, 0.83):
        box("Back wall", (0, d / 2 - 0.09, z), (w - 0.25, 0.12, 0.2), wall)
    beam("Lintel", (-w / 2, d / 2 - 0.1, 1.83), (w / 2, d / 2 - 0.1, 1.83), 0.18, "bark")
    shingled_roof(w, d / 2 + 0.1, 0.02 if home else 0.51, 1.93, roof_height, roof)
    for x in (-w / 2 + 0.1, w / 2 - 0.1):
        beam("Knee brace", (x, 0.5, 1.25), (x, 1.1, 1.8), 0.1, "wood")
    for y, z, width in ((-d / 2 - 0.12, 0.08, 1.2), (-d / 2 - 0.29, 0.04, 1.38)):
        box("Welcome step", (0, y, z), (width, 0.22, z * 2), "stone_light", 0.025)
    lantern((-w / 2 + 0.13, -d / 2 + 0.01, 1.32))
    for x in (-w / 2 + 0.22, w / 2 - 0.22):
        box("Foundation corner course", (x, -d / 2 - 0.01, 0.09), (0.31, 0.24, 0.16), "slate", 0.025)


def bed_row(positions):
    """Build framed nests at the supplied positions in the current asset."""
    for i, (x, y) in enumerate(positions):
        box("Low bed frame", (x, y, 0.34), (0.75, 0.88, 0.18), "wood")
        box("Bed headboard", (x, y + 0.38, 0.56), (0.77, 0.065, 0.42), "bark")
        orb("Woven nest", (x, y, 0.48), (0.33, 0.38, 0.14), "straw", 2)
        orb("Bed quilt", (x, y - 0.08, 0.56), (0.29, 0.27, 0.06), "cloth" if i % 2 else "teal_light", 2)
        box("Quilt stripe", (x, y - 0.12, 0.617), (0.51, 0.055, 0.012), "cream", 0.004)
        orb("Pillow", (x, y + 0.24, 0.56), (0.24, 0.11, 0.08), "cream", 2)


def study_table():
    """Build a table and three books in the current asset."""
    table(0, 0.25, 2.1, 0.8)
    for i, x in enumerate((-0.7, -0.15, 0.4)):
        book((x, 0.22, 0.94), ["cloth", "teal", "terracotta"][i])


def timber_bench():
    """Build the timber workbench in the current asset."""
    table(0.25, -0.2, 1.6, 0.8)
