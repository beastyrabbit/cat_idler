"""Procedural slate-roof craft buildings for the Idle Cat Forest demo slice."""

import bpy

from .helpers import (
    ASSETS,
    beam,
    box,
    cylinder,
    finish_asset,
    furnace,
    crate,
    bowl,
    table,
    mesh,
    math,
    merge_group,
    new_asset,
    orb,
    rng,
    torus,
)
from .buildings_shared import workplace_shell


def _base(static, wall="wood", side_walls=True, back_height=1.35, shoe="stone_light", timber="bark", board_alt="endgrain", threshold="endgrain"):
    static.append(box("Stone foundation", (0, 0, 0.10), (3.28, 2.78, 0.20), "stone", 0.07))
    for x in (-1.48, 1.48):
        static.append(box("Foundation corner", (x, -1.22, 0.20), (0.32, 0.26, 0.24), shoe, 0.025))
        static.append(box("Foundation corner", (x, 1.22, 0.20), (0.32, 0.26, 0.24), shoe, 0.025))
    board_count = 12
    for i in range(board_count):
        mat = "wood" if i % 2 else board_alt
        static.append(box("Floor board", (-1.50 + (i + 0.5) * 3.0 / board_count, 0, 0.25), (3.0 / board_count - 0.018, 2.48, 0.10), mat, 0.015))
    for x in (-1.48, 1.48):
        static.append(box("Timber post", (x, 1.23, 0.98), (0.20, 0.20, 1.55), timber, 0.025))
        static.append(box("Post shoe", (x, 1.23, 0.34), (0.25, 0.25, 0.28), shoe, 0.03))
        static.append(box("Front post", (x, -1.20, 0.95), (0.20, 0.20, 1.45), timber, 0.025))
        static.append(box("Front shoe", (x, -1.20, 0.34), (0.25, 0.25, 0.28), shoe, 0.03))
        if side_walls:
            static.append(box("Side half wall", (x, 0.27, 0.62), (0.12, 1.90, 0.72), wall, 0.02))
    static.append(box("Back wall", (0, 1.30, 0.83), (2.94, 0.14, back_height), wall, 0.025))
    static.append(box("Back lintel", (0, 1.21, 1.72), (3.02, 0.18, 0.16), timber, 0.02))
    static.append(box("Welcome step", (0, -1.48, 0.19), (1.28, 0.25, 0.24), shoe, 0.025))
    static.append(box("Threshold", (0, -1.32, 0.31), (1.48, 0.12, 0.12), threshold, 0.015))
    static.append(beam("Left knee brace", (-1.48, 0.55, 1.15), (-1.48, 1.16, 1.72), 0.10, timber))
    static.append(beam("Right knee brace", (1.48, 0.55, 1.15), (1.48, 1.16, 1.72), 0.10, timber))


def _tile_mat(alternate):
    if alternate and rng().random() < 0.10:
        return alternate
    return "slate"


def _lean_roof(static, high_back=2.48, low_front=1.86, alternate="iron", ridge_mat="bark", trim="endgrain"):
    front, back = 0.05, 1.43
    rise, run = high_back - low_front, back - front
    angle = math.atan2(rise, run)
    for row in range(5):
        y = front + (row + 0.5) * run / 5
        z = low_front + (y - front) / run * rise + 0.035
        for col in range(8):
            x = -1.58 + (col + 0.5) * 3.16 / 8
            static.append(box("Slate roof tile", (x, y, z), (0.40, run / 5 + 0.05, 0.085), _tile_mat(alternate), 0.012, rotation=(angle, 0, 0)))
    static.append(beam("Lean ridge cap", (-1.63, back, high_back + 0.03), (1.63, back, high_back + 0.03), 0.15, ridge_mat))
    static.append(beam("Lean eave fascia", (-1.63, front, low_front - 0.015), (1.63, front, low_front - 0.015), 0.12, trim))
    for x in (-1.35, 0, 1.35):
        static.append(beam("Roof rafter", (x, front, low_front - 0.04), (x, back, high_back - 0.04), 0.075, "bark" if ridge_mat != "wood" else "wood"))


def _gable_roof(static, ridge=2.58, eave=1.70, depth=1.40, front=0.05, alternate="iron", ridge_mat="bark", trim="endgrain"):
    half = 1.62
    rise = ridge - eave
    slope = math.atan2(rise, half)
    for side in (-1, 1):
        for row in range(5):
            y = front + (row + 0.5) * depth / 5
            for col in range(4):
                x = side * (half * (col + 0.5) / 4)
                z = ridge - (col + 0.5) * half / 4 * math.tan(slope) + 0.035
                static.append(box("Slate gable tile", (x, y, z), (half / 4 + 0.055, depth / 5 + 0.05, 0.085), _tile_mat(alternate), 0.012, rotation=(0, side * slope, 0)))
    static.append(beam("Gable ridge cap", (0, front - 0.04, ridge + 0.03), (0, front + depth + 0.04, ridge + 0.03), 0.15, ridge_mat))
    static.append(beam("Front gable tie", (-half, front, eave - 0.06), (half, front, eave - 0.06), 0.12, trim))
    static.append(beam("Front king post", (0, front, eave - 0.06), (0, front, ridge - 0.08), 0.10, trim))
    static.append(beam("Back gable tie", (-half, front + depth, eave - 0.06), (half, front + depth, eave - 0.06), 0.12, trim))


def _aframe_roof(static, ridge=2.68, eave=0.56, front=-0.35, depth=1.78, alternate="iron"):
    half = 1.58
    rise = ridge - eave
    slope = math.atan2(rise, half)
    for side in (-1, 1):
        for row in range(5):
            y = front + (row + 0.5) * depth / 5
            for col in range(4):
                x = side * (half * (col + 0.5) / 4)
                z = ridge - (col + 0.5) * half / 4 * math.tan(slope) + 0.035
                static.append(box("Slate A-frame tile", (x, y, z), (half / 4 + 0.06, depth / 5 + 0.05, 0.09), _tile_mat(alternate), 0.012, rotation=(0, side * slope, 0)))
    static.append(beam("A-frame ridge", (0, front - 0.03, ridge + 0.04), (0, front + depth + 0.03, ridge + 0.04), 0.15, "bark"))
    for side in (-1, 1):
        static.append(beam("A-frame front rafter", (0, front, ridge), (side * half, front, eave), 0.11, "bark"))


def _finish_part(name, objects, pivot):
    if len(objects) == 1:
        obj = objects[0]
        obj.name = name
        bpy.context.scene.cursor.location = pivot
        bpy.ops.object.select_all(action="DESELECT")
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.origin_set(type="ORIGIN_CURSOR")
    else:
        obj = merge_group(name, objects, pivot)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    obj.rotation_euler = (0, 0, 0)
    obj.scale = (1, 1, 1)
    return obj


def _finish(name, static, parts=None):
    if static:
        merge_group(name + "_mesh", static)
    finish_asset(name, articulated=True)
    expected = {name + "_mesh"} | set(parts or {})
    actual = {o.name for o in ASSETS[name]["meshes"]}
    assert actual == expected, (name, actual, expected)
    for part in parts or {}:
        obj = parts[part]
        assert obj.name == part and tuple(round(v, 5) for v in obj.rotation_euler) == (0, 0, 0)
        assert tuple(round(v, 5) for v in obj.scale) == (1, 1, 1)


def _wheel_part(name, pos, radius, mat="wood"):
    x, y, z = pos
    pieces = [torus("wheel rim", pos, radius * 0.87, radius * 0.12, mat, rotation=(math.pi / 2, 0, 0)), cylinder("wheel hub", pos, radius * 0.18, 0.24, "iron", 10, rotation=(math.pi / 2, 0, 0))]
    for i in range(8):
        a = i * math.tau / 8
        pieces.append(beam("wheel spoke", pos, (x + math.cos(a) * radius * 0.78, y, z + math.sin(a) * radius * 0.78), 0.075, "endgrain"))
    return pieces


def workshop():
    name = "workshop"
    new_asset(name, "Workshop: open slate lean-to with grindstone, vise and tool wall")
    static = []
    _base(static, back_height=1.40)
    _lean_roof(static)
    # The front-left grindstone is a thick, trestled focal shape.
    for x in (-1.03, -0.54):
        static.append(box("Grindstone trestle", (x, -0.87, 0.72), (0.12, 0.16, 1.05), "bark", 0.02))
    static.append(beam("Grindstone crossbar", (-1.20, -0.87, 0.78), (-0.38, -0.87, 0.78), 0.12, "endgrain"))
    static.extend([box("Crank", (-0.38, -0.99, 1.38), (0.10, 0.34, 0.10), "iron", 0.015), box("Treadle", (-0.78, -0.96, 0.30), (0.62, 0.12, 0.08), "wood", 0.012)])
    grind = [cylinder("grind disc", (-0.78, -0.87, 1.30), 0.53, 0.20, "stone", 16, rotation=(math.pi / 2, 0, 0)), cylinder("grind hub", (-0.78, -0.87, 1.30), 0.14, 0.28, "iron", 10, rotation=(math.pi / 2, 0, 0))]
    for x in (-0.78,):
        static.append(cylinder("Hub cap", (x, -1.03, 1.30), 0.07, 0.04, "endgrain", 10, rotation=(math.pi / 2, 0, 0)))
    # Back tool wall: broad, readable silhouettes rather than thin decoration.
    static.append(box("Tool rack", (0, 0.08, 1.28), (2.42, 0.08, 0.78), "bark", 0.02))
    static.append(box("Tool saw", (-0.72, 0.01, 1.30), (0.62, 0.05, 0.17), "iron", 0.01))
    static.append(beam("Tool saw handle", (-1.05, 0.01, 1.30), (-0.90, 0.01, 1.58), 0.07, "wood"))
    static.append(box("Tool hammer head", (0.10, 0.00, 1.32), (0.36, 0.13, 0.13), "iron", 0.02))
    static.append(beam("Tool hammer handle", (0.10, 0.00, 1.32), (0.10, 0.00, 1.70), 0.065, "wood"))
    static.append(beam("Tool tongs", (0.72, 0.03, 1.18), (1.04, 0.03, 1.55), 0.065, "iron"))
    static.append(beam("Tool tongs", (0.80, 0.03, 1.18), (1.12, 0.03, 1.47), 0.065, "iron"))
    static.extend([box("Workbench top", (0.28, -0.28, 0.88), (1.52, 0.72, 0.14), "endgrain", 0.025), box("Workbench vise", (0.82, -0.68, 1.02), (0.38, 0.18, 0.18), "iron", 0.02), box("Vise screw", (0.82, -0.82, 0.92), (0.08, 0.34, 0.08), "iron", 0.01)])
    for x in (-0.28, 0.05, 0.38):
        static.extend([cylinder("Jar", (x, 0.10, 0.55), 0.14, 0.35, "berry", 10), cylinder("Jar lid", (x, 0.10, 0.74), 0.15, 0.04, "endgrain", 10)])
    static.append(box("Tool crate", (0.98, 0.42, 0.48), (0.52, 0.48, 0.48), "wood", 0.035))
    static.extend([beam("Hoist post", (-1.20, 1.34, 1.72), (-1.20, 1.34, 3.20), 0.16, "bark"), beam("Hoist post", (1.20, 1.34, 1.72), (1.20, 1.34, 3.20), 0.16, "bark"), beam("Hoist gantry", (-1.20, 1.34, 3.20), (1.20, 1.34, 3.20), 0.18, "endgrain"), cylinder("Hoist pulley", (0, 1.25, 2.94), 0.25, 0.14, "iron", 12, rotation=(math.pi / 2, 0, 0))])
    static.append(box("Tool mallet head", (-0.35, 0.01, 1.58), (0.25, 0.14, 0.16), "wood", 0.018))
    static.append(beam("Tool mallet handle", (-0.35, 0.01, 1.20), (-0.35, 0.01, 1.58), 0.08, "iron"))
    _finish(name, static, {"Grindstone": _finish_part("Grindstone", grind, (-0.78, -0.87, 1.30))})


def smithy():
    name = "smithy"
    new_asset(name, "Smithy: rear stone hearth, coursed chimney, bellows and ember bed")
    static = []
    _base(static, wall="stone", back_height=1.48, shoe="stone", timber="wood", board_alt="wood", threshold="wood")
    _gable_roof(static, ridge=2.52, eave=1.70, ridge_mat="wood", trim="wood")
    # Tapered coursed chimney rises a metre above the ridge.
    for i, z in enumerate((2.05, 2.38, 2.71, 3.04, 3.37)):
        static.append(box("Chimney course", (0.55, 1.00, z), (0.52 - i * 0.025, 0.52 - i * 0.025, 0.28), "stone_light" if i in (1, 3) else "stone", 0.035))
    static.append(box("Chimney mouth", (0.55, 1.00, 3.63), (0.42, 0.42, 0.12), "coal", 0.02))
    static.extend([box("Stone hearth", (0.55, 0.10, 0.74), (1.02, 1.10, 0.92), "stone", 0.06), box("Hearth cap", (0.55, 0.10, 1.24), (1.12, 1.16, 0.14), "stone_light", 0.03)])
    for x, y in ((0.20, -0.52), (0.90, -0.52), (0.20, -0.02), (0.90, -0.02)):
        static.append(box("Coal rim stone", (x, y, 1.31), (0.18, 0.13, 0.10), "coal", 0.018))
    glow = _finish_part("ForgeGlow", [box("Forge ember bed", (0.55, -0.28, 1.34), (0.60, 0.40, 0.10), "ember", 0.025)], (0.55, -0.28, 1.34))
    bellows = [box("Bellows body", (-0.10, 0.62, 0.78), (0.55, 0.38, 0.28), "wood", 0.035), box("Bellows leather", (-0.43, 0.62, 0.78), (0.22, 0.34, 0.23), "leather", 0.025), cylinder("Bellows nozzle", (-0.56, 0.62, 0.78), 0.09, 0.30, "iron", 8, rotation=(0, math.pi / 2, 0))]
    bell = _finish_part("Bellows", bellows, (-0.56, 0.62, 0.78))
    static.extend([box("Anvil stump", (-0.72, -0.62, 0.48), (0.48, 0.48, 0.56), "wood", 0.04), box("Anvil face", (-0.72, -0.62, 0.83), (0.78, 0.40, 0.14), "iron", 0.03), box("Anvil waist", (-0.72, -0.62, 0.68), (0.30, 0.25, 0.20), "iron", 0.02)])
    static.append(box("Smith rack", (-0.55, 0.72, 1.30), (1.20, 0.08, 0.72), "wood", 0.02))
    static.append(beam("Smith hammer", (-0.85, 0.67, 1.28), (-0.85, 0.67, 1.68), 0.08, "iron"))
    static.append(box("Smith tongs", (-0.48, 0.67, 1.40), (0.42, 0.08, 0.08), "iron", 0.015))
    static.append(cylinder("Quench barrel", (1.15, -0.83, 0.58), 0.28, 0.58, "wood", 10))
    static.append(torus("Quench barrel band", (1.15, -0.83, 0.48), 0.25, 0.035, "iron"))
    static.append(torus("Quench barrel band", (1.15, -0.83, 0.72), 0.25, 0.035, "iron"))
    _finish(name, static, {"Bellows": bell, "ForgeGlow": glow})
    empty = bpy.data.objects.new("ChimneyTop", None)
    ASSETS[name]["collection"].objects.link(empty)
    empty.parent = ASSETS[name]["root"]
    empty.location = (0.55, 1.00, 3.70)


def barracks():
    name = "barracks"
    new_asset(name, "Barracks: rear slate gable, crossed spear ridge, shields and training dummies")
    static = []
    _base(static, wall="wood", back_height=1.34, shoe="stone")
    _gable_roof(static, ridge=2.58, eave=1.70)
    for y in (0.10, 1.38):
        static.append(beam("Spear crest shaft", (-0.72, y, 2.12), (0.72, y, 3.26), 0.09, "wood"))
        static.append(beam("Spear crest shaft", (0.72, y, 2.12), (-0.72, y, 3.26), 0.09, "wood"))
        static.append(beam("Spear crest head", (-0.72, y, 3.26), (-0.72, y, 3.52), 0.12, "iron"))
        static.append(beam("Spear crest head", (0.72, y, 3.26), (0.72, y, 3.52), 0.12, "iron"))
    static.append(cylinder("Banner pole", (1.25, 0.42, 1.98), 0.065, 3.40, "wood", 8))
    static.append(box("Berry banner", (0.90, 0.42, 2.94), (0.70, 0.06, 0.98), "berry", 0.02))
    for x in (-0.92, 0, 0.92):
        static.append(cylinder("Shield", (x, 0.70, 1.22), 0.30, 0.08, "berry", 12, rotation=(math.pi / 2, 0, 0)))
        static.append(cylinder("Shield boss", (x, 0.64, 1.22), 0.08, 0.10, "iron", 8, rotation=(math.pi / 2, 0, 0)))
    for x in (-0.72, 0.72):
        static.extend([cylinder("Dummy post", (x, -0.78, 0.72), 0.11, 1.05, "wood", 8), orb("Straw target", (x, -0.78, 1.30), (0.31, 0.23, 0.36), "straw", 2), beam("Dummy arms", (x - 0.43, -0.78, 1.28), (x + 0.43, -0.78, 1.28), 0.09, "wood"), torus("Target ring", (x, -1.02, 1.30), 0.16, 0.035, "berry", rotation=(math.pi / 2, 0, 0))])
    _finish(name, static)


def wood_cutter():
    name = "wood_cutter"
    new_asset(name, "Wood cutter: rear sideways slate lean-to, cordwood stack and buried axe")
    static = []
    _base(static, wall="bark", back_height=1.34)
    # A sideways roof slopes toward the open work area but begins at the rear.
    front, back = 0.05, 1.43
    angle = 0.22
    static.append(box("Side lean roof backing", (0, 0.74, 2.28), (3.08, 1.42, 0.13), "slate", 0.02, rotation=(0, angle, 0)))
    for row in range(5):
        x = -1.35 + (row + 0.5) * 2.70 / 5
        z = 2.30 - x * 0.22 + 0.06
        for col in range(4):
            y = 0.22 + col * 0.34
            static.append(box("Slate side tile", (x, y, z), (0.61, 0.38, 0.09), _tile_mat("iron"), 0.014, rotation=(0, angle, 0)))
    static.append(beam("Side ridge", (1.55, back, 1.99), (1.55, front, 1.99), 0.15, "bark"))
    static.append(beam("Side eave", (-1.55, front, 2.34), (-1.55, back, 2.34), 0.12, "endgrain"))
    for row in range(5):
        for col in range(3):
            x = -1.36 + col * 0.29
            z = 0.43 + row * 0.29
            static.append(cylinder("Cordwood log", (x, 0.76, z), 0.155, 0.60, "bark", 8, rotation=(math.pi / 2, 0, 0)))
            for end in (-1, 1):
                static.append(cylinder("Cordwood end", (x, 0.76 + end * 0.305, z), 0.13, 0.025, "endgrain", 8, rotation=(math.pi / 2, 0, 0)))
    static.extend([cylinder("Chopping block", (-0.52, -0.78, 0.43), 0.34, 0.72, "bark", 12), cylinder("Block end", (-0.52, -0.78, 0.80), 0.29, 0.025, "endgrain", 12)])
    axe = [beam("Axe handle", (-0.52, -0.78, 0.79), (-0.39, -0.78, 1.47), 0.09, "wood"), box("Axe head", (-0.29, -0.78, 1.45), (0.40, 0.16, 0.27), "iron", 0.025)]
    axe_obj = _finish_part("Axe", axe, (-0.52, -0.78, 0.79))
    for x in (0.25, 0.75):
        static.append(box("Split log", (x, -0.64, 0.40), (0.42, 0.72, 0.25), "bark", 0.03))
    static.extend([beam("Sawhorse leg", (0.50, 0.25, 0.25), (0.22, 0.25, 0.80), 0.10, "wood"), beam("Sawhorse leg", (1.06, 0.25, 0.25), (1.34, 0.25, 0.80), 0.10, "wood"), beam("Sawhorse top", (0.22, 0.25, 0.80), (1.34, 0.25, 0.80), 0.11, "endgrain")])
    _finish(name, static, {"Axe": axe_obj})


def stone_prep():
    name = "stone_prep"
    new_asset(name, "Stone prep: roofless derrick, hanging dressed block and stepped mason pile")
    static = []
    _base(static, wall="wood", side_walls=False, back_height=0.80)
    for x in (-1.05, 1.05):
        static.append(box("Frame post", (x, 1.12, 1.10), (0.16, 0.16, 1.70), "bark", 0.02))
    static.append(beam("Frame rail", (-1.05, 1.12, 1.85), (1.05, 1.12, 1.85), 0.13, "endgrain"))
    # One clear derrick replaces the old square overhead frame.
    static.extend([beam("Derrick leg", (0, 0.75, 0.35), (-0.78, 0.48, 2.95), 0.14, "bark"), beam("Derrick leg", (0, 0.75, 0.35), (0.78, 0.48, 2.95), 0.14, "bark"), beam("Derrick mast", (0, 0.75, 0.35), (0, 0.75, 3.05), 0.15, "bark"), beam("Derrick jib", (0, 0.75, 2.86), (0, -0.88, 2.58), 0.13, "endgrain"), beam("Derrick rope", (0, -0.88, 2.58), (0, -0.88, 1.35), 0.045, "leather"), box("Hanging dressed block", (0, -0.88, 1.10), (0.48, 0.46, 0.46), "stone_light", 0.03)])
    for row in range(3):
        for col in range(3 - row):
            static.append(box("Dressed block", (-0.86 + col * 0.55 + row * 0.27, -0.82 + row * 0.38, 0.43 + row * 0.34), (0.48, 0.42, 0.32), "stone_light", 0.035))
    for x, y in ((1.05, -0.70), (1.12, 0.10), (-1.12, -0.25)):
        static.append(orb("Quarry stone", (x, y, 0.43), (0.31, 0.27, 0.30), "stone", 1))
    static.extend([box("Mason bench", (0.20, -0.52, 0.76), (1.15, 0.48, 0.12), "endgrain", 0.02), box("Chisel rack", (0.20, -0.78, 1.00), (0.90, 0.08, 0.10), "bark", 0.015)])
    for x in (-0.05, 0.25, 0.55):
        static.append(cylinder("Chisel", (x, -0.82, 1.15), 0.035, 0.32, "iron", 8, rotation=(0.15, 0, 0)))
    for i in range(6):
        static.append(box("Slate flagstone", (-1.25 + i * 0.48, -0.30, 0.32), (0.38, 0.48, 0.035), "slate", 0.008))
    static.append(cylinder("Derrick pulley", (0, -0.88, 2.55), 0.18, 0.12, "wood", 12, rotation=(math.pi / 2, 0, 0)))
    static.append(cylinder("Pulley axle", (0, -0.95, 2.55), 0.05, 0.22, "iron", 8, rotation=(math.pi / 2, 0, 0)))
    for x in (-0.65, 0.95):
        static.append(orb("Quarry stone", (x, 0.62, 0.48), (0.26, 0.23, 0.24), "stone", 2))
    for x, y in ((-1.30, 0.65), (-0.88, 0.68), (-0.42, 0.72), (0.18, 0.72), (0.62, 0.68), (1.30, 0.66), (-1.32, -0.78), (1.32, -0.78)):
        static.append(orb("Quarry stone", (x, y, 0.44), (0.22, 0.20, 0.22), "stone", 2))
    for x in (-0.35, 0.05, 0.45, 0.85):
        static.append(box("Chisel handle", (x, -0.86, 1.28), (0.07, 0.07, 0.36), "wood", 0.012))
    _finish(name, static)


def woodworking():
    name = "woodworking"
    new_asset(name, "Woodworking: steep rear-open A-frame, planing bench and large drying rack")
    static = []
    _base(static, wall="wood", back_height=1.22)
    _aframe_roof(static)
    static.extend([box("Planing bench top", (0, -0.72, 0.86), (1.50, 0.64, 0.14), "endgrain", 0.025), box("Bench vice", (0.46, -1.00, 1.00), (0.28, 0.16, 0.18), "iron", 0.02)])
    for x in (-0.60, 0.60):
        static.append(box("Bench leg", (x, -0.72, 0.50), (0.12, 0.12, 0.58), "bark", 0.015))
    for x, y in ((-0.30, -0.63), (0.05, -0.54), (0.40, -0.64), (0.18, -0.43)):
        static.append(orb("Wood shavings", (x, y, 0.98), (0.16, 0.08, 0.04), "endgrain", 1))
    # Tall rack and six boards form a second, unmistakable A-frame.
    static.append(beam("Drying rack rail", (0.72, 0.55, 2.02), (1.42, 0.55, 2.02), 0.12, "bark"))
    for i in range(6):
        x = 0.76 + i * 0.13
        static.append(box("Drying plank", (x, 0.40, 1.25), (0.12, 0.88, 1.55), "endgrain", 0.018, rotation=(0.12, 0.12 * (i - 2.5) / 3, 0)))
    static.extend([box("Chair seat", (0.88, -0.20, 0.58), (0.52, 0.52, 0.11), "wood", 0.02), box("Chair rail", (0.88, 0.08, 1.16), (0.50, 0.10, 0.14), "endgrain", 0.015), beam("Chair back", (0.68, 0.08, 0.60), (0.68, 0.08, 1.12), 0.08, "wood"), beam("Chair back", (1.08, 0.08, 0.60), (1.08, 0.08, 1.12), 0.08, "wood")])
    _finish(name, static)


def clothier():
    name = "clothier"
    new_asset(name, "Clothier: rear slate gable, upright loom and colourful ridge cloth bolts")
    static = []
    _base(static, wall="wood", back_height=1.36, shoe="stone", timber="wood")
    _gable_roof(static, ridge=2.60, eave=1.68, ridge_mat="wood", trim="wood")
    loom_static = [box("Loom upright", (-0.65, -0.48, 1.18), (0.14, 0.15, 1.78), "wood", 0.02), box("Loom upright", (0.65, -0.48, 1.18), (0.14, 0.15, 1.78), "wood", 0.02), box("Loom top", (0, -0.48, 2.03), (1.46, 0.16, 0.15), "endgrain", 0.02), box("Woven cloth", (0, -0.47, 1.48), (0.94, 0.07, 0.62), "cloth", 0.02)]
    loom_static.extend([beam("Warp thread", (-0.42 + i * 0.14, -0.43, 1.16), (-0.42 + i * 0.14, -0.43, 1.88), 0.018, "cream") for i in range(7)])
    beater = _finish_part("Loom", [box("Loom beater", (0, -0.56, 1.18), (1.16, 0.11, 0.13), "wood", 0.018)], (0, -0.56, 1.18))
    static.extend(loom_static)
    # The high pole carries wide fabric over both roof slopes.
    static.append(beam("Cloth ridge pole", (0, 0.05, 2.74), (0, 1.43, 2.74), 0.10, "wood"))
    for y, mat in ((0.27, "cloth"), (0.74, "berry"), (1.21, "cream")):
        static.append(box("Cloth bolt", (0, y, 2.73), (0.22, 0.40, 0.18), mat, 0.02))
        slope = math.atan2(2.60 - 1.68, 1.62)
        for side in (-1, 1):
            static.append(box("Long cloth drape", (side * 0.81, y, 2.30), (1.84, 0.40, 0.08), mat, 0.018, rotation=(0, side * slope, 0)))
    static.append(box("Spool table", (0.88, -0.12, 0.80), (0.68, 0.48, 0.12), "endgrain", 0.02))
    for x, mat in ((0.68, "berry"), (0.88, "cream"), (1.08, "cloth")):
        static.append(cylinder("Thread spool", (x, -0.12, 1.00), 0.09, 0.28, mat, 8))
    _finish(name, static, {"Loom": beater})


def tannery():
    name = "tannery"
    new_asset(name, "Cutaway colony " + name.replace("_", " "))
    workplace_shell("terracotta")
    for x in (-0.97, 0.38):
        box("Drying frame post", (x, 0.5, 1.01), (0.11, 0.11, 1.54), "wood")
    beam("Drying frame rail", (-1.02, 0.5, 1.69), (0.43, 0.5, 1.69), 0.1, "wood")
    mesh("Stretched hide", [(-0.8, 0.48, 1.52), (-0.64, 0.48, 1.11), (-0.86, 0.48, 0.6), (-0.29, 0.48, 0.77), (0.2, 0.48, 0.6), (0.01, 0.48, 1.11), (0.2, 0.48, 1.52), (-0.3, 0.48, 1.4)], [tuple(range(8))], "leather")
    bowl((0.79, -0.28, 0.24), 0.43, "bark")
    table(-0.63, -0.7, 1.1, 0.42, 0.65)
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


def sawmill():
    name = "sawmill"
    new_asset(name, "Sawmill: rear slate lean-to, vertical frame saw and large drive wheel")
    static = []
    _base(static, back_height=1.36)
    _lean_roof(static, high_back=2.46, low_front=1.84)
    static.extend([box("Saw tower post", (-0.62, -0.48, 2.42), (0.20, 0.20, 3.00), "bark", 0.025), box("Saw tower post", (0.62, -0.48, 2.42), (0.20, 0.20, 3.00), "bark", 0.025), beam("Saw tower top", (-0.62, -0.48, 3.90), (0.62, -0.48, 3.90), 0.20, "endgrain"), beam("Saw tower brace", (-0.62, -0.40, 3.40), (0, -0.40, 3.90), 0.10, "wood"), beam("Saw tower brace", (0.62, -0.40, 3.40), (0, -0.40, 3.90), 0.10, "wood")])
    # Big vertical blade in the tower, with a thick lower and upper tooth rail.
    saw_parts = [box("Saw blade", (0, -0.58, 2.32), (0.24, 0.10, 1.46), "iron", 0.015)]
    for i in range(9):
        saw_parts.append(box("Saw tooth", (0.14, -0.58, 1.68 + i * 0.16), (0.10, 0.10, 0.10), "iron", 0, rotation=(0, 0.5, 0)))
    saw = _finish_part("Saw", saw_parts, (0, -0.58, 2.32))
    for y in (-0.86, -0.15):
        static.append(box("Feed rail", (0, y, 0.88), (2.50, 0.10, 0.11), "iron", 0.018))
    static.extend([box("Log carriage", (0, -0.52, 0.98), (1.58, 0.62, 0.14), "wood", 0.025), cylinder("Feed log", (0, -0.52, 1.18), 0.25, 1.70, "bark", 10, rotation=(math.pi / 2, 0, 0)), cylinder("Feed log end", (0, -1.38, 1.18), 0.21, 0.025, "endgrain", 7, rotation=(math.pi / 2, 0, 0))])
    wheel_pieces = _wheel_part("Wheel", (0.78, -0.18, 1.23), 0.68, "wood")
    wheel_obj = _finish_part("Wheel", wheel_pieces, (0.78, -0.18, 1.23))
    _finish(name, static, {"Saw": saw, "Wheel": wheel_obj})


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
