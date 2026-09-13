"""Render the saved art library without regenerating or exporting it."""

from pathlib import Path

import bpy
from mathutils import Vector


ART = Path(__file__).resolve().parent
bpy.ops.wm.open_mainfile(filepath=str(ART / "idle_cat_forest.blend"))
scene = bpy.context.scene
camera = scene.camera
(ART / "previews").mkdir(exist_ok=True)


def render(name, location, target, size, width=1400, height=1100):
    camera.location = location
    camera.data.ortho_scale = size
    camera.rotation_euler = (Vector(target) - camera.location).to_track_quat("-Z", "Y").to_euler()
    scene.render.resolution_x = width
    scene.render.resolution_y = height
    scene.render.filepath = str(ART / "previews" / (name + ".png"))
    bpy.ops.render.render(write_still=True)


render("catalog", (45, -40, 59), (21.6, 16.8, 0), 57, 2400, 1900)
# Closeups use a continuous stage instead of exposing the catalog floor's edge.
ground = bpy.data.objects["Gallery ground"]
ground.location = (0, 0, -0.1)
ground.dimensions = (120, 120, 0.15)
render("cat", (2.1, -3.4, 1.8), (0, 0, 0.65), 2.25, 1100, 1000)
for coll in bpy.data.collections:
    if coll.name != "Preview only - not exported":
        coll.hide_render = True
for name, pos in {"cat": (0, -1.5, 0), "sawmill": (-2.25, 1.2, 0),
                  "clothier": (2.25, 1.2, 0), "tree_oak": (-4.5, 3.3, 0),
                  "tree_pine": (4.8, 3.6, 0), "cargo_logs": (0, -1.47, 0.67)}.items():
    bpy.data.collections[name].hide_render = False
    bpy.data.objects[name].location = pos
render("workplaces", (8, -12, 12), (0, 1.1, 0.9), 12)
render("cat_control", (2.8, -6, 2.6), (0, 0, 1.0), 7.4)
for coll in bpy.data.collections:
    if coll.name != "Preview only - not exported":
        coll.hide_render = True
for name, pos in {"shrine": (0, 0, 0), "den": (-3.6, 1.8, 0),
                  "tree_oak": (3.7, 2.5, 0), "cat": (0.4, -2.3, 0),
                  "gate": (3.4, -1.25, 0)}.items():
    bpy.data.collections[name].hide_render = False
    bpy.data.objects[name].location = pos
render("sanctuary", (7.5, -10, 11), (-0.15, 0.2, 0.7), 11.5, 1600, 1300)
for name in ("shrine", "tree_oak", "gate"):
    bpy.data.collections[name].hide_render = True
bpy.data.objects["den"].location = (0, 0, 0)
bpy.data.objects["cat"].location = (0, -2.2, 0)
render("den", (5, -7, 6), (0, -0.3, 1), 6, 1400, 1200)
