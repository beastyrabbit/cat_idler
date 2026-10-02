"""Original geometry, one builder per asset."""

from .helpers import (
    beam,
    curve,
    finish_asset,
    fur_marking,
    merge_group,
    mesh,
    new_asset,
    orb,
)

from . import helpers as H


def cat():
    new_asset("cat", "Sculpted ginger cat with head, tail and four paw pivots for procedural animation")
    orb("Torso", (0, 0.02, 0.59), (0.27, 0.47, 0.28), "fur", 3, True)
    orb("Chest bib", (0, -0.3, 0.56), (0.21, 0.18, 0.26), "fur_light", 2, True)
    for angle in (1.42, 1.82, 2.21):
        for side in (-1, 1):
            fur_marking("Tabby haunch marking", (0, 0.02, 0.59), (0.273, 0.474, 0.284), side * angle, 0.06, 0.67, 0.052)
    merge_group("Body", list(H.CURRENT.objects))
    before = set(H.CURRENT.objects)
    orb("Cheek silhouette", (0, -0.44, 0.87), (0.32, 0.265, 0.29), "fur", 3, True)
    for side in (-1, 1):
        x = side * 0.2
        mesh(
            "Pointed ear",
            [(x - 0.115, -0.41, 1.0), (x + 0.115, -0.41, 1.0), (x + side * 0.055, -0.4, 1.34), (x, -0.21, 1.02)],
            [(0, 2, 1), (0, 3, 2), (1, 2, 3), (0, 1, 3)],
            "fur",
        )
        mesh(
            "Ear pink",
            [(x - 0.068, -0.417, 1.055), (x + 0.068, -0.417, 1.055), (x + side * 0.035, -0.412, 1.255)],
            [(0, 2, 1)],
            "nose",
        )
        orb("Cream muzzle", (side * 0.093, -0.665, 0.81), (0.115, 0.073, 0.085), "fur_light", 2, True)
        orb("Eye surround", (side * 0.137, -0.64, 0.965), (0.075, 0.021, 0.087), "fur_dark", 2, True)
        orb("Amber iris", (side * 0.137, -0.654, 0.965), (0.066, 0.019, 0.078), "gold", 2, True)
        orb("Cat slit pupil", (side * 0.137, -0.668, 0.965), (0.02, 0.01, 0.061), "eye", 2, True)
        orb("Eye catchlight", (side * 0.13 - 0.014, -0.68, 0.989), (0.016, 0.009, 0.022), "cream", 1)
        curve(
            "Cat mouth",
            [(0, -0.719, 0.815), (side * 0.024, -0.733, 0.788), (side * 0.069, -0.727, 0.796)],
            0.008,
            "fur_dark",
        )
        for j in range(2):
            beam(
                "Whisker",
                (side * 0.15, -0.7, 0.815 - j * 0.04),
                (side * 0.39, -0.665, 0.84 - j * 0.075),
                0.014,
                "cream",
            )
    orb("Nose", (0, -0.727, 0.845), (0.052, 0.028, 0.036), "nose", 1)
    for angle in (-0.39, 0, 0.39):
        fur_marking("Forehead stripe", (0, -0.44, 0.87), (0.324, 0.269, 0.294), angle, 0.55, 1.01, 0.049)
    merge_group("Head", [o for o in H.CURRENT.objects if o not in before], (0, -0.31, 0.8))
    before = set(H.CURRENT.objects)
    curve(
        "Upright curling tail",
        [(0, 0.38, 0.64), (0.03, 0.6, 0.71), (0.07, 0.72, 0.98), (0.12, 0.65, 1.2), (0.15, 0.51, 1.19)],
        0.085,
        "fur",
    )
    orb("Tail tip", (0.15, 0.51, 1.19), (0.089, 0.11, 0.085), "fur_dark", 2, True)
    merge_group("Tail", [o for o in H.CURRENT.objects if o not in before], (0, 0.36, 0.62))
    for side, label in [(-1, "Left"), (1, "Right")]:
        for y, pair in [(-0.28, "Front"), (0.3, "Back")]:
            before = set(H.CURRENT.objects)
            orb("Leg", (side * 0.18, y, 0.33), (0.105, 0.115, 0.26), "fur", 2, True)
            orb("Cream sock", (side * 0.18, y - 0.035, 0.105), (0.12, 0.15, 0.105), "fur_light", 2, True)
            merge_group("Paw" + pair + label, [o for o in H.CURRENT.objects if o not in before], (side * 0.18, y, 0.48))
    finish_asset("cat", articulated=True)


BUILDERS = {
    "cat": cat,
}
