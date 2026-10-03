"""Authoritative procedural cat contract for Idle Cat Forest.

Source is Z-up and the nose faces -Y.  Every rigid part is baked to identity
rotation and unit scale.  The part list gives ``name: parent, adult pivot,
kitten pivot, purpose`` (kitten pivots are the authored 1.08 scale):

* Body: root, (0,0,0), (0,0,0), torso and bob anchor.
* Head: Body, (0,-0.31,0.80), (0,-0.3348,0.8208), face and neck anchor.
* EarL/EarR: Head, (-0.22,-0.40,1.02)/(+0.22,-0.40,1.02),
  (-0.2538,-0.432,1.0368)/(+0.2538,-0.432,1.0368), expressive ears.
* Jaw: Head, (0,-0.59,0.80), (0,-0.6372,0.81), muzzle and speech jaw.
* EyelidL/EyelidR: Head, (-0.065,-0.505,0.905)/(+0.065,-0.505,0.905),
  (-0.06696,-0.40284,0.8964)/(+0.06696,-0.40284,0.8964), blink lids and lashes; each pivot is the deep eyeball centre.
* Tail: Body, (0,0.36,0.62), (0,0.324,0.5616), tail root.
* Tail_1: Tail, (0.05,0.53,0.72), (0.0378,0.4968,0.6372), first tail link.
* Tail_2: Tail_1, (0.10,0.65,0.92), (0.0702,0.6156,0.7560), second tail link.
* Tail_3: Tail_2, (0.14,0.68,1.10), (0.0810,0.5940,0.8640), tail tip link.
* PawFrontLeft/PawFrontRight: Body, (-0.18,-0.28,0.69)/(+0.18,-0.28,0.69),
  (-0.1836,-0.2484,0.6156)/(+0.1836,-0.2484,0.6156), front upper legs.
* PawBackLeft/PawBackRight: Body, (-0.18,0.30,0.69)/(+0.18,0.30,0.69),
  (-0.1836,0.2484,0.6156)/(+0.1836,0.2484,0.6156), rear upper legs.
* LegFrontLeftLower/LegFrontRightLower: PawFrontLeft/PawFrontRight,
  (-0.18,-0.288,0.455)/(+0.18,-0.288,0.455),
  (-0.1836,-0.2570,0.3672)/(+0.1836,-0.2570,0.3672), lower front legs and paws.
* LegBackLeftLower/LegBackRightLower: PawBackLeft/PawBackRight,
  (-0.18,0.312,0.455)/(+0.18,0.312,0.455),
  (-0.1836,0.2614,0.3672)/(+0.1836,0.2614,0.3672), lower rear legs and paws.
* CarryBack: Body, (0,0.04,0.86), (0,0.04,0.70), identity cargo socket.
* CarryMouth: Head, (0,-0.78,0.83), (0,-0.74,0.79), identity mouth cargo socket.

Left/L is negative source X.  Parts retain these source axes in Unity; the
imported root carries the FBX conversion.  Local X is lateral, local Y follows
the spine with the nose toward -Y, and local Z is up.  Yaw is local Z, pitch is
local X, and roll is local Y; a Body bob therefore uses the parent's local Z.
Positive local X closes an eyelid at +70 degrees and opens the jaw at +25
degrees. Adult eyelid pivots are at `(+/-0.065,-0.505,0.905)` and kitten pivots at
`(+/-0.06696,-0.40284,0.8964)` after the authored 1.08 scale. Adult eyeballs use
radius 0.225 m with a 0.150 m opening and kitten eyeballs use authored radius
0.270 m (0.292 m after scale) with a 0.205 m opening after scale. The visible cap has
20-sided concentric cream, gold and eye circles, a 55% pupil and one outer cream
catchlight. Each eyelid is a 0.009 m concentric shell with 60 triangles, rests
inside the head, and closes over the visible eye at +70 degrees with a short
dark lower lash. The eye cap allows up to 0.030 m rest protrusion. Coat slots are ``Coat_Base`` (main fur), ``Coat_Marking`` (dark tabby
or colour patches), ``Coat_White`` (bib, muzzle and socks), and ``Coat_Point``
(ears, legs and point coat).  Recipes are tabby (base + marking + white),
tuxedo (black base + white), calico (orange/black patches + white), colour
point (light base + dark points), and solid (one colour in every slot).

The kitten is authored at adult length and rendered at width 0.47 versus the
adult width 0.85.  The export wrapper includes the CarryBack and CarryMouth
socket empties in each FBX while keeping mesh records unchanged.
``world_elder_scarf`` attaches under ``Head`` with identity local position,
rotation and scale.  Its measured neck path and berry-dominant knit are owned
by the world accessory builder.
"""

import math
import re
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

from . import helpers as H
from .helpers import finish_asset, merge_group, mesh, new_asset


_COAT_HEX = {
    "Coat_Base": "D89545",
    "Coat_Marking": "995528",
    "Coat_White": "F5DBB0",
    "Coat_Point": "D89545",
}


def _coat_material(slot):
    if slot in H.MATERIALS:
        return H.MATERIALS[slot]
    rgb = tuple(int(_COAT_HEX[slot][i : i + 2], 16) / 255 for i in (0, 2, 4))
    runtime_name = {
        "Coat_Base": "Forest_Coat_Base_fur",
        "Coat_Marking": "Forest_Coat_Marking_fur_dark",
        "Coat_White": "Forest_Coat_White_fur_light",
        "Coat_Point": "Forest_Coat_Point_fur",
    }[slot]
    mat = H.bpy.data.materials.new(runtime_name)
    mat.diffuse_color = (*rgb, 1)
    mat.use_nodes = True
    shader = mat.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = (*rgb, 1)
    shader.inputs["Roughness"].default_value = 0.88
    shader.inputs["Metallic"].default_value = 0
    H.MATERIALS[slot] = mat
    return mat


for _slot in _COAT_HEX:
    _coat_material(_slot)


def _install_export_wrapper():
    """Include sockets in selection while preserving the mesh-only records."""
    if getattr(H.export_one, "_cat_socket_wrapper", False):
        return
    original = H.export_one

    def export_one_with_sockets(name, entry):
        sockets = entry.get("sockets", [])
        targets = list(entry.get("meshes", [])) + list(sockets)
        renames = []
        held = []
        desired = {}
        for obj in targets:
            canonical = re.sub(r"\.\d{3}$", "", obj.name)
            desired[obj.name] = canonical
        canonical_names = set(desired.values())
        for obj in list(H.bpy.context.scene.objects):
            want = desired.get(obj.name)
            if (want is not None or obj.name in canonical_names) and obj not in targets:
                old = obj.name
                obj.name = f"__held_{len(held)}_{old}"
                held.append((obj, old))
        for obj in targets:
            old = obj.name
            new_name = desired[old]
            if old != new_name:
                obj.name = new_name
                renames.append((obj, old))
        try:
            copied = dict(entry)
            copied["meshes"] = list(entry["meshes"]) + list(sockets)
            return original(name, copied)
        finally:
            for obj, old in renames:
                obj.name = old
            for obj, old in held:
                obj.name = old

    export_one_with_sockets._cat_socket_wrapper = True
    H.export_one = export_one_with_sockets


_install_export_wrapper()


def _material(slot):
    return _coat_material(slot) if slot in _COAT_HEX else H.material(slot)


def _multi_mesh(name, verts, faces, slots, face_slots, smooth=True):
    data = H.bpy.data.meshes.new(name)
    data.from_pydata(verts, [], faces)
    data.update()
    obj = H.bpy.data.objects.new(name, data)
    H.CURRENT.objects.link(obj)
    for slot in slots:
        data.materials.append(_material(slot))
    indices = {slot: i for i, slot in enumerate(slots)}
    for poly, slot in zip(data.polygons, face_slots, strict=True):
        poly.material_index = indices[slot]
        poly.use_smooth = smooth
    return obj


def _ellipsoid(name, center, radii, slots, mask, nlon=20, nlat=12, smooth=True):
    """Closed UV ellipsoid with a material chosen from each face centre."""
    cx, cy, cz = center
    rx, ry, rz = radii
    verts = [(cx, cy, cz + rz)]
    for ilat in range(1, nlat):
        theta = math.pi * ilat / nlat
        st, ct = math.sin(theta), math.cos(theta)
        for ilon in range(nlon):
            phi = math.tau * ilon / nlon
            verts.append((cx + rx * st * math.sin(phi), cy - ry * st * math.cos(phi), cz + rz * ct))
    bottom = len(verts)
    verts.append((cx, cy, cz - rz))
    faces = []
    fslots = []
    # top cap
    for ilon in range(nlon):
        j = (ilon + 1) % nlon
        faces.append((0, 1 + j, 1 + ilon))
        p = tuple(sum(verts[k][axis] for k in faces[-1]) / 3 for axis in range(3))
        fslots.append(mask(p))
    # latitude bands
    for ilat in range(nlat - 2):
        row = 1 + ilat * nlon
        nxt = row + nlon
        for ilon in range(nlon):
            j = (ilon + 1) % nlon
            faces.append((row + ilon, row + j, nxt + j, nxt + ilon))
            p = tuple(sum(verts[k][axis] for k in faces[-1]) / 4 for axis in range(3))
            fslots.append(mask(p))
    # bottom cap
    row = 1 + (nlat - 2) * nlon
    for ilon in range(nlon):
        j = (ilon + 1) % nlon
        faces.append((row + ilon, row + j, bottom))
        p = tuple(sum(verts[k][axis] for k in faces[-1]) / 3 for axis in range(3))
        fslots.append(mask(p))
    return _multi_mesh(name, verts, faces, slots, fslots, smooth)


def _capsule(name, a, b, r0, r1, slots, mask, radial=10, cap_rings=2):
    """Smooth, closed tapered capsule with hemispherical caps at both endpoints."""
    a, b = Vector(a), Vector(b)
    axis = b - a
    length = axis.length
    if length < 1e-5:
        raise ValueError(f"capsule {name} has coincident endpoints")
    w = axis.normalized()
    ref = Vector((0, 0, 1)) if abs(w.z) < 0.92 else Vector((1, 0, 0))
    u = w.cross(ref).normalized()
    v = w.cross(u).normalized()
    # Three profile rings per cap when cap_rings=3, plus the pole.  Adult limbs
    # use the denser setting; the smaller kitten keeps the same smooth silhouette
    # at a lower triangle count.
    fractions = [] if cap_rings <= 0 else ([0.72] if cap_rings == 1 else ([0.82, 0.44] if cap_rings == 2 else [0.87, 0.60, 0.30]))
    profile = [(0.0, -r0, 0.0)]
    for f in fractions:
        profile.append((0.0, -r0 * f, math.sqrt(max(0.0, 1 - f * f)) * r0))
    profile.append((0.0, 0.0, r0))
    profile.append((0.5, 0.0, r0 * 0.5 + r1 * 0.5))
    profile.append((1.0, 0.0, r1))
    for f in reversed(fractions):
        profile.append((1.0, r1 * f, math.sqrt(max(0.0, 1 - f * f)) * r1))
    profile.append((1.0, r1, 0.0))
    # Convert (unused, axial offset, ring radius) to world rings.  The first
    # component in the profile is retained for readability of the cap math.
    rings = []
    verts = []
    for _t, offset, radius in profile:
        if len(rings) == 0:
            point = a + w * offset
        elif len(rings) <= len(fractions) + 2:
            t = profile[len(rings)][0]
            point = a + w * (t * length + offset)
        else:
            t = profile[len(rings)][0]
            point = a + w * (t * length + offset)
        if radius < 1e-7:
            rings.append([len(verts)])
            verts.append(tuple(point))
            continue
        ring = []
        for j in range(radial):
            angle = math.tau * j / radial
            p = point + u * (math.cos(angle) * radius) + v * (math.sin(angle) * radius)
            ring.append(len(verts))
            verts.append(tuple(p))
        rings.append(ring)
    faces = []
    fslots = []
    for r_a, r_b in zip(rings, rings[1:]):
        if len(r_a) == 1 and len(r_b) > 1:
            for j in range(radial):
                faces.append((r_a[0], r_b[(j + 1) % radial], r_b[j]))
                center = tuple(sum(verts[k][axis] for k in faces[-1]) / 3 for axis in range(3))
                fslots.append(mask(center, 0.0))
        elif len(r_a) > 1 and len(r_b) == 1:
            for j in range(radial):
                faces.append((r_a[j], r_b[0], r_a[(j + 1) % radial]))
                center = tuple(sum(verts[k][axis] for k in faces[-1]) / 3 for axis in range(3))
                fslots.append(mask(center, 1.0))
        else:
            # Ring order is selected consistently around the axis; finish_asset
            # recalculates the final closed-solid winding after joining.
            for j in range(radial):
                faces.append((r_a[j], r_a[(j + 1) % radial], r_b[(j + 1) % radial], r_b[j]))
                center = tuple(sum(verts[k][axis] for k in faces[-1]) / 4 for axis in range(3))
                projection = (Vector(center) - a).dot(w) / length
                fslots.append(mask(center, max(0.0, min(1.0, projection))))
    return _multi_mesh(name, verts, faces, slots, fslots, True)


def _thin_prism(name, a, b, width, slot):
    a, b = Vector(a), Vector(b)
    axis = (b - a).normalized()
    ref = Vector((0, 0, 1)) if abs(axis.z) < 0.9 else Vector((1, 0, 0))
    u = axis.cross(ref).normalized() * width
    v = axis.cross(u).normalized() * width
    verts = []
    for point in (a, b):
        verts.extend([tuple(point - u - v), tuple(point + u - v), tuple(point + u + v), tuple(point - u + v)])
    faces = [(0, 4, 5, 1), (1, 5, 6, 2), (2, 6, 7, 3), (3, 7, 4, 0)]
    return _multi_mesh(name, verts, faces, [slot], [slot] * len(faces), False)


def _bake(obj):
    H.bpy.context.view_layer.objects.active = obj
    H.bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    H.bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    obj.rotation_euler = (0, 0, 0)
    obj.scale = (1, 1, 1)
    return obj


def _join_part(name, objects, pivot):
    for obj in objects:
        _bake(obj)
    if len(objects) == 1:
        result = objects[0]
        result.name = name
        H.bpy.context.scene.cursor.location = pivot
        H.bpy.ops.object.select_all(action="DESELECT")
        result.select_set(True)
        H.bpy.context.view_layer.objects.active = result
        H.bpy.ops.object.origin_set(type="ORIGIN_CURSOR")
    else:
        result = merge_group(name, objects, pivot)
    result.rotation_euler = (0, 0, 0)
    result.scale = (1, 1, 1)
    result["pivot_source"] = tuple(pivot)
    return result


def _parent_keep_world(obj, parent):
    world = obj.matrix_world.copy()
    obj.parent = parent
    obj.matrix_parent_inverse = parent.matrix_world.inverted()
    obj.matrix_world = world


def _body_mask(center, params):
    x, y, z = center
    cx, cy, cz, rx, ry, rz, kitten = params
    nx, ny, nz = (x - cx) / rx, (y - cy) / ry, (z - cz) / rz
    if (ny < -0.28 and nz < 0.25) or nz < -0.60:
        return "Coat_White"
    # Marking boundaries land on the ellipsoid's latitude rings and meridians.
    # Quantising the longitude keeps the broad patches from making a checkerboard
    # of partial faces while preserving a deliberately asymmetric tabby saddle.
    longitude = (math.atan2(nx, -ny) / math.tau) % 1.0
    sector = int(math.floor(longitude * 18 + 0.5)) % 18
    if nz > 0.309 and ny > -0.45:
        return "Coat_Marking"
    left_blotch = sector in {4, 5, 6} and -0.31 < nz < 0.31
    right_blotch = sector in {12, 13} and -0.16 < nz < 0.24
    if left_blotch or right_blotch:
        return "Coat_Marking"
    return "Coat_Base"


def _head_mask(center, params):
    x, y, z = center
    cx, cy, cz, rx, ry, rz, kitten = params
    nx, ny, nz = (x - cx) / rx, (y - cy) / ry, (z - cz) / rz
    if ny < -0.70 and nz < -0.03:
        return "Coat_White"
    if ny < -0.55 and -0.06 < nz < 0.48 and abs(nx) < 0.55:
        return "Coat_Point"
    # The forehead cap follows complete latitude rows with a few whole
    # meridian columns removed for a soft, asymmetric M rather than square
    # face-by-face patches.
    longitude = (math.atan2(nx, -ny) / math.tau) % 1.0
    sector = int(math.floor(longitude * 14 + 0.5)) % 14
    upper_row = nz > 0.309
    front_columns = sector in {0, 1, 13}
    side_cap = (sector in {2, 3} and nz > 0.0) or (sector in {11, 12} and nz > 0.309)
    if upper_row and ny < 0.16 and (front_columns or side_cap):
        return "Coat_Marking"
    return "Coat_Base"


def _ear_mesh(name, side, kitten):
    base_z = 1.02 if not kitten else 0.96
    tip_z = 1.37 if not kitten else 1.34
    x = side * (0.22 if not kitten else 0.235)
    # A thick four-sided ear sunk into the skull; the inner prism is inset.
    # Keep the back shallow so a flicked ear remains a crisp triangle from
    # the side instead of turning into a broad rounded lobe.
    outer = [(x - 0.13, -0.43, base_z), (x + 0.13, -0.43, base_z), (x + side * 0.055, -0.40, tip_z), (x, -0.30, base_z + 0.045)]
    inner = [(x - 0.078, -0.456, base_z + 0.055), (x + 0.078, -0.456, base_z + 0.055), (x + side * 0.032, -0.425, tip_z - 0.105)]
    verts = outer + [(px, py + 0.035, pz) for px, py, pz in outer]
    faces = [(0, 2, 1), (0, 3, 2), (1, 2, 3), (0, 1, 3), (4, 5, 6), (4, 6, 7), (0, 4, 7, 3), (1, 2, 6, 5), (0, 1, 5, 4), (3, 7, 6, 2)]
    slots = ["Coat_Point"] * len(faces)
    obj = _multi_mesh(name, verts, faces, slots, slots, False)
    # Inset pink inner face as a thin triangular prism, closed on all sides.
    iv = inner + [(px, py + 0.012, pz) for px, py, pz in inner]
    inf = [(0, 1, 2), (3, 5, 4), (0, 3, 4, 1), (1, 4, 5, 2), (2, 5, 3, 0)]
    obj2 = _multi_mesh("inner ear", iv, inf, ["nose"], ["nose"] * len(inf), False)
    return [obj, obj2]


def _unit(value):
    value = Vector(value)
    if value.length < 1e-8:
        raise ValueError("zero direction")
    return value.normalized()


def _spherical_cap(name, center, radius, direction, half_angle, slots, mask, nphi=12, ntheta=4, inner_offset=0.002, ring_angles=None):
    """Closed thin spherical cap with its front surface on the eyeball."""
    center = Vector(center)
    direction = _unit(direction)
    basis = Vector((1, 0, 0))
    side = direction.cross(basis).normalized()
    outer_radius = radius
    inner_radius = radius - inner_offset
    verts = [tuple(center + outer_radius * direction), tuple(center + inner_radius * direction)]
    outer_rings, inner_rings = [], []
    for ring in range(1, ntheta + 1):
        theta = ring_angles[ring - 1] if ring_angles is not None else half_angle * ring / ntheta
        st, ct = math.sin(theta), math.cos(theta)
        outer_ring, inner_ring = [], []
        for phi_index in range(nphi):
            phi = math.tau * phi_index / nphi
            radial = side * math.cos(phi) + direction.cross(side) * math.sin(phi)
            outer_ring.append(len(verts))
            verts.append(tuple(center + outer_radius * (direction * ct + radial * st)))
            inner_ring.append(len(verts))
            verts.append(tuple(center + inner_radius * (direction * ct + radial * st)))
        outer_rings.append(outer_ring)
        inner_rings.append(inner_ring)
    faces, face_slots = [], []
    for phi_index in range(nphi):
        nxt = (phi_index + 1) % nphi
        faces.append((0, outer_rings[0][nxt], outer_rings[0][phi_index]))
        face_slots.append(mask(Vector(center + outer_radius * direction)))
        faces.append((1, inner_rings[0][phi_index], inner_rings[0][nxt]))
        face_slots.append(mask(Vector(center + inner_radius * direction)))
    for ring in range(ntheta - 1):
        outer_a, outer_b = outer_rings[ring], outer_rings[ring + 1]
        inner_a, inner_b = inner_rings[ring], inner_rings[ring + 1]
        for phi_index in range(nphi):
            nxt = (phi_index + 1) % nphi
            faces.append((outer_a[phi_index], outer_a[nxt], outer_b[nxt], outer_b[phi_index]))
            face_slots.append(mask(Vector(sum((verts[index][axis] for index in faces[-1]), 0.0) / 4 for axis in range(3))))
            faces.append((inner_a[nxt], inner_a[phi_index], inner_b[phi_index], inner_b[nxt]))
            face_slots.append(mask(Vector(sum((verts[index][axis] for index in faces[-1]), 0.0) / 4 for axis in range(3))))
    outer_edge, inner_edge = outer_rings[-1], inner_rings[-1]
    for phi_index in range(nphi):
        nxt = (phi_index + 1) % nphi
        faces.append((outer_edge[phi_index], outer_edge[nxt], inner_edge[nxt], inner_edge[phi_index]))
        face_slots.append(mask(Vector(sum((verts[index][axis] for index in faces[-1]), 0.0) / 4 for axis in range(3))))
    return _multi_mesh(name, verts, faces, slots, face_slots, True)


def _eye_geometry(kitten):
    # A large sunk sphere gives the eye a round opening while its front cap
    # stays close to the surrounding head.  The kitten keeps the same
    # construction at a larger relative opening before its authored 1.08
    # uniform scale.
    eye_x = 0.065 if not kitten else 0.062
    eye_y = -0.505 if not kitten else -0.373
    eye_z = 0.905 if not kitten else 0.830
    eye_radius = 0.225 if not kitten else 0.270
    opening_radius = 0.075 if not kitten else 0.095
    iris_radius = 0.063 if not kitten else 0.080
    pupil_radius = iris_radius * 0.55
    return eye_x, eye_y, eye_z, eye_radius, opening_radius, iris_radius, pupil_radius


def _eye_front(side, kitten):
    return Vector((side * 0.40, -math.sqrt(1.0 - 0.40 ** 2 - 0.22 ** 2), 0.22))


def _shape_eye_sockets(head_shell, kitten):
    eye_x, eye_y, eye_z, eye_radius, opening_radius, _iris_radius, _pupil_radius = _eye_geometry(kitten)
    head_front_y = -0.44 if not kitten else -0.37
    for vertex in head_shell.data.vertices:
        point = vertex.co
        if point.y >= head_front_y:
            continue
        for side in (-1, 1):
            centre = Vector((side * eye_x, eye_y, eye_z))
            front = _eye_front(side, kitten)
            delta = point - centre
            radial = (delta - front * delta.dot(front)).length
            blend = max(0.0, min(1.0, (1.45 - radial / opening_radius) / 0.30))
            if blend:
                # Sink only the aperture; leave its outer fur edge as a brow.
                target = math.sqrt(max(0.0, eye_radius ** 2 - radial ** 2)) - 0.018
                point += front * (target - delta.dot(front)) * blend


def _head_front_y(head_center, head_radii, x, z):
    cx, cy, cz = head_center
    rx, ry, rz = head_radii
    nx = (x - cx) / rx
    nz = (z - cz) / rz
    radial = nx * nx + nz * nz
    if radial >= 1.0:
        return None
    return cy - ry * math.sqrt(1.0 - radial)


def _assert_eye_geometry(name, kitten, eye_center, eye_radius, opening_radius, head_center, head_radii):
    """Check that the inserted sphere remains a shallow, recessed eye cap."""
    centre = Vector(eye_center)
    max_protrusion = -math.inf
    front = _eye_front(1 if centre.x >= 0 else -1, kitten)
    u = _unit(Vector((1, 0, 0)) - front * front.x)
    v = front.cross(u)
    for ring in range(13):
        r = opening_radius * ring / 12
        for index in range(40):
            phi = math.tau * index / 40
            p = centre + front * math.sqrt(eye_radius ** 2 - r ** 2) + r * (u * math.cos(phi) + v * math.sin(phi))
            surface_y = _head_front_y(head_center, head_radii, p.x, p.z)
            if surface_y is not None:
                max_protrusion = max(max_protrusion, surface_y - p.y)
    limit = 0.030 / (1.08 if kitten else 1.0)
    if max_protrusion > limit + 1e-5:
        raise AssertionError(f"{name}: {'kitten ' if kitten else ''}eyeball protrudes {max_protrusion * (1.08 if kitten else 1.0):.5f} m beyond the head")


def _assert_eyelid_geometry(name, lid_obj, pivot, eye_radius, iris_radius, pupil_radius, kitten, close_angle, rest_direction, half_angle):
    pivot = Vector(pivot)
    points = [lid_obj.matrix_world @ vertex.co for vertex in lid_obj.data.vertices]
    distances = [(point - pivot).length for point in points]
    if min(distances) < eye_radius - 1e-5 or max(distances) > eye_radius + 0.022 + 1e-5:
        raise AssertionError(f"{name}: {lid_obj.name} is not concentric with its eyeball (r={min(distances):.5f}-{max(distances):.5f}, eye={eye_radius:.5f})")
    silhouette_over = max(math.hypot(*(point - pivot)[::2]) - eye_radius for point in points)
    if silhouette_over > 0.010 + 1e-5:
        raise AssertionError(f"{name}: {lid_obj.name} protrudes {silhouette_over:.5f} m beyond the rest eye silhouette")

    closed_direction = Matrix.Rotation(close_angle, 4, "X") @ rest_direction
    front = _eye_front(1 if pivot.x > 0 else -1, kitten)
    if closed_direction.dot(front) < 1 - 1e-6:
        raise AssertionError(f"{name}: closed lid is not centred on the visible eye")
    # The polygon's inscribed aperture must cover the pupil and iris centre.
    covered_radius = (eye_radius + 0.009) * math.sin(half_angle) * math.cos(math.pi / 15)
    if covered_radius < iris_radius * 0.80:
        raise AssertionError(f"{name}: lid misses the visible iris/pupil centre region")


def _face_details(kitten):
    eye_x, eye_y, eye_z, eye_radius, _opening_radius, iris_radius, pupil_radius = _eye_geometry(kitten)
    pieces = []
    for side in (-1, 1):
        x = side * eye_x
        center = (x, eye_y, eye_z)
        front = _eye_front(side, kitten)
        # Shared twenty-sided boundaries avoid intersecting or jagged layers.
        u = _unit(Vector((1, 0, 0)) - front * front.x)
        v = front.cross(u)
        verts = [tuple(Vector(center) + front * eye_radius)]
        for r in (pupil_radius, iris_radius, _opening_radius):
            for index in range(20):
                phi = math.tau * index / 20
                verts.append(tuple(Vector(center) + front * math.sqrt(eye_radius ** 2 - r ** 2) + r * (u * math.cos(phi) + v * math.sin(phi))))
        back = len(verts)
        verts.append(tuple(Vector(center) + front * (eye_radius - 0.045)))
        faces, slots = [], []
        for index in range(20):
            nxt = (index + 1) % 20
            faces.append((0, 1+index, 1+nxt))
            slots.append('eye')
            for row, slot in ((0, 'gold'), (1, 'cream')):
                a, b = 1+row*20, 21+row*20
                faces.append((a+index, b+index, b+nxt, a+nxt))
                slots.append(slot)
            faces.append((back, 41+nxt, 41+index))
            slots.append('cream')
        pieces.append(_multi_mesh('round eye cap', verts, faces, ['cream', 'gold', 'eye'], slots))
        catch_radius = 0.010 if not kitten else 0.013
        offset = u * (side * pupil_radius * 0.43) + v * (pupil_radius * 0.45)
        catch_direction = _unit(front * math.sqrt(eye_radius ** 2 - offset.length_squared) + offset)
        pieces.append(_spherical_cap('outer cream catchlight', center, eye_radius + 0.003, catch_direction, math.asin(catch_radius / (eye_radius + 0.003)), ['cream'], lambda p: 'cream', nphi=10, ntheta=1, inner_offset=0.001))
    pieces.append(_ellipsoid("left muzzle pad", (-0.078, -0.675 if not kitten else -0.663, 0.822 if not kitten else 0.795), (0.094, 0.057, 0.060), ["Coat_White"], lambda p: "Coat_White", (6 if kitten else 8), 5))
    pieces.append(_ellipsoid("right muzzle pad", (0.078, -0.675 if not kitten else -0.663, 0.822 if not kitten else 0.795), (0.094, 0.057, 0.060), ["Coat_White"], lambda p: "Coat_White", (6 if kitten else 8), 5))
    pieces.append(_ellipsoid("pink nose", (0, -0.760 if not kitten else -0.735, 0.845 if not kitten else 0.815), (0.053, 0.032, 0.040), ["nose"], lambda p: "nose", (4 if kitten else 6), 4))
    pieces.append(_capsule("mouth line", (0, -0.748, 0.803 if not kitten else 0.777), (0, -0.754, 0.772 if not kitten else 0.748), 0.007, 0.005, ["eye"], lambda p, t: "eye", radial=4, cap_rings=0))
    return pieces


def _make_leg(side, y, label, kitten):
    x = side * (0.18 if not kitten else 0.17)
    shoulder = (x, y, 0.69 if not kitten else 0.57)
    knee = (x, y + (0.012 if y > 0 else -0.008), 0.455 if not kitten else 0.34)
    ankle = (x, y - (0.012 if y < 0 else -0.006), 0.145 if not kitten else 0.105)
    upper_slots = ["Coat_Base", "Coat_Marking"]
    def upper_mask(p, t):
        if (0.24 < t < 0.43 and y < 0) or (0.50 < t < 0.72 and y > 0):
            return "Coat_Marking"
        return "Coat_Base"
    # The endpoint ball is deliberately small: its centre remains at the
    # authored shoulder/hip pivot, while the torso hides the entire cap at
    # rest.  The shaft widens immediately toward the knee so the leg still
    # reads as a continuous limb when the parent swings.
    rear_adult = (not kitten) and y > 0
    upper_r0 = 0.012
    upper_r1 = 0.056 if rear_adult else (0.076 if not kitten else 0.066)
    upper = [_capsule("upper leg", shoulder, knee, upper_r0, upper_r1, upper_slots, upper_mask, radial=(8 if kitten else 8), cap_rings=(0 if kitten else 1))]
    # Paw parts swing from the shoulder/hip.  The capsule's top ball is
    # centred on that pivot so animation keeps the joint closed.
    upper_obj = _join_part("Paw" + label, upper, shoulder)
    upper_obj["joint_source"] = tuple(knee)

    def lower_mask(p, t):
        return "Coat_White" if t > 0.78 else "Coat_Point"
    lower = [_capsule("lower leg", knee, ankle, 0.076 if not kitten else 0.066, 0.060 if not kitten else 0.055, ["Coat_Point", "Coat_White"], lower_mask, radial=(8 if kitten else 8), cap_rings=(0 if kitten else 1))]
    paw = _ellipsoid("rounded paw", (x, y - (0.035 if y < 0 else 0.0), 0.075 if not kitten else 0.055), (0.112 if not kitten else 0.105, 0.145 if not kitten else 0.125, 0.072 if not kitten else 0.062), ["Coat_White"], lambda p: "Coat_White", (4 if kitten else 6), 4)
    lower.append(paw)
    lower_obj = _join_part("Leg" + label + "Lower", lower, knee)
    if (Vector(lower_obj["pivot_source"]) - Vector(upper_obj["joint_source"])).length > 1e-6:
        raise AssertionError(f"{lower_obj.name}: pivot is not the upper capsule end centre")
    return upper_obj, lower_obj


def _assert_upper_joint_inside(name, obj, shoulder, torso_center, torso_radii, cap_radius):
    """Keep the endpoint ball inside the torso and overlapping through a swing."""
    shoulder = Vector(shoulder)
    torso_center = Vector(torso_center)
    torso_radii = Vector(torso_radii)
    points = [obj.matrix_world @ vertex.co for vertex in obj.data.vertices]
    ball = [point for point in points if (point - shoulder).length <= cap_radius * 1.35 + 1e-6]
    if not ball:
        raise AssertionError(f"{name}: upper-leg endpoint has no sampled ball vertices")

    def ellipsoid_value(point):
        delta = point - torso_center
        return sum((delta[index] / torso_radii[index]) ** 2 for index in range(3))

    margin = 0.015
    rest_max = max(ellipsoid_value(point) for point in ball)
    if rest_max > 1.0 - margin:
        raise AssertionError(f"{name}: upper-leg top ball leaves torso at rest ({rest_max:.4f})")
    for degrees in (-35.0, 35.0):
        swing = Matrix.Rotation(math.radians(degrees), 4, "X")
        swung = [shoulder + swing @ (point - shoulder) for point in ball]
        if min(ellipsoid_value(point) for point in swung) > 1.0 - margin:
            raise AssertionError(f"{name}: upper-leg connection loses torso overlap at {degrees:+.0f} degrees")


def _triangulate_parts():
    """Bake final face topology before recording counts and exporting FBX."""
    for obj in list(H.CURRENT.objects):
        if obj.type != "MESH":
            continue
        H.bpy.ops.object.select_all(action="DESELECT")
        obj.select_set(True)
        H.bpy.context.view_layer.objects.active = obj
        H.bpy.ops.object.mode_set(mode="EDIT")
        H.bpy.ops.mesh.select_all(action="SELECT")
        H.bpy.ops.mesh.quads_convert_to_tris(quad_method="BEAUTY", ngon_method="BEAUTY")
        H.bpy.ops.object.mode_set(mode="OBJECT")


def _build_cat(name, kitten=False):
    description = "Unmistakable round-proportioned kitten with closed capsule joints and coat masks" if kitten else "Articulated storybook cat with smooth capsule joints, surface coat masks and carry sockets"
    new_asset(name, description)
    for slot in _COAT_HEX:
        _coat_material(slot)

    if kitten:
        body_center, body_radii = (0, 0.01, 0.43), (0.29, 0.43, 0.33)
        head_center, head_radii = (0, -0.37, 0.75), (0.39, 0.30, 0.35)
        body_pivot, head_pivot = (0, 0, 0), (0, -0.31, 0.76)
    else:
        body_center, body_radii = (0, 0.03, 0.57), (0.30, 0.49, 0.30)
        head_center, head_radii = (0, -0.44, 0.88), (0.40, 0.29, 0.30)
        body_pivot, head_pivot = (0, 0, 0), (0, -0.31, 0.80)

    eye_x, eye_y, eye_z, eye_radius, opening_radius, _iris_radius, _pupil_radius = _eye_geometry(kitten)
    for side in (-1, 1):
        _assert_eye_geometry(
            name,
            kitten,
            (side * eye_x, eye_y, eye_z),
            eye_radius,
            opening_radius,
            head_center,
            head_radii,
        )

    body_params = (*body_center, *body_radii, kitten)
    body = [_ellipsoid("torso", body_center, body_radii, ["Coat_Base", "Coat_Marking", "Coat_White"], lambda p: _body_mask(p, body_params), nlon=14 if kitten else 15, nlat=10 if kitten else 10)]
    if not kitten:
        body.append(_ellipsoid("chest bib", (0, -0.39, 0.68), (0.17, 0.105, 0.21), ["Coat_White"], lambda p: "Coat_White", 6, 5))
    body_obj = _join_part("Body", body, body_pivot)

    head_params = (*head_center, *head_radii, kitten)
    head_slots = ["Coat_Base", "Coat_Marking", "Coat_White", "Coat_Point"]
    head_shell = _ellipsoid("chunky cheek head", head_center, head_radii, head_slots, lambda p: _head_mask(p, head_params), nlon=14 if kitten else 14, nlat=10 if kitten else 10)
    _shape_eye_sockets(head_shell, kitten)
    head = [head_shell]
    if not kitten:
        head.extend([
            _ellipsoid("left cheek fluff", (-0.285, -0.46, 0.84), (0.14, 0.19, 0.17), head_slots, lambda p: _head_mask(p, head_params), 6, 5),
            _ellipsoid("right cheek fluff", (0.285, -0.46, 0.84), (0.14, 0.19, 0.17), head_slots, lambda p: _head_mask(p, head_params), 6, 5),
        ])
    # a separate white chin keeps the muzzle readable while the head mask remains flush
    head.append(_ellipsoid("chin", (0, -0.675 if not kitten else -0.655, 0.775 if not kitten else 0.745), (0.12, 0.065, 0.060), ["Coat_White"], lambda p: "Coat_White", (5 if kitten else 6), (4 if kitten else 4)))
    head.extend(_face_details(kitten))
    head_obj = _join_part("Head", head, head_pivot)

    ear_objs = {}
    for side, suffix in ((-1, "L"), (1, "R")):
        ear_objs[suffix] = _join_part("Ear" + suffix, _ear_mesh("ear back", side, kitten), (side * (0.22 if not kitten else 0.235), -0.40, 1.02 if not kitten else 0.96))

    jaw_center = (0, -0.675 if not kitten else -0.65, 0.765 if not kitten else 0.735)
    jaw = _join_part("Jaw", [_ellipsoid("lower jaw", jaw_center, (0.15, 0.09, 0.075), ["Coat_White"], lambda p: "Coat_White", (5 if kitten else 8), (4 if kitten else 5))], (0, -0.59, 0.80 if not kitten else 0.75))
    lids = {}
    # Each lid is a closed spherical shell concentric with its eyeball.  The
    # rest cap is top/back; +70 deg local X carries it over the front.
    lid_angle = math.radians(70)
    for side, suffix in ((-1, "L"), (1, "R")):
        eye_x, eye_y, eye_z, eye_radius, _opening_radius, iris_radius, pupil_radius = _eye_geometry(kitten)
        pivot = (side * eye_x, eye_y, eye_z)
        outer_radius = eye_radius + 0.009 / (1.08 if kitten else 1.0)
        closed_rotation = Matrix.Rotation(lid_angle, 4, "X")
        front = _eye_front(side, kitten)
        rest_direction = closed_rotation.inverted() @ front
        lid_half_angle = math.asin((_opening_radius + 0.003) / outer_radius)

        def lid_material(point):
            closed = closed_rotation @ (point - Vector(pivot))
            u = _unit(Vector((1, 0, 0)) - front * front.x)
            v = front.cross(u)
            # Only the thin boundary wall at the lower middle is dark.
            radial = (closed - front * closed.dot(front)).length
            return "eye" if radial > _opening_radius * 0.97 and closed.dot(v) < -_opening_radius * 0.65 else "Coat_Base"

        shell = _spherical_cap(
            "concentric lid shell",
            pivot,
            outer_radius,
            rest_direction,
            lid_half_angle,
            ["Coat_Base", "eye"],
            lid_material,
            nphi=15,
            ntheta=1,
            inner_offset=outer_radius - eye_radius,
        )
        lid_obj = _join_part("Eyelid" + suffix, [shell], pivot)
        lids[suffix] = lid_obj
        _assert_eyelid_geometry(name, lid_obj, pivot, eye_radius, iris_radius, pupil_radius, kitten, lid_angle, rest_direction, lid_half_angle)
        lid_obj["eyeball_radius"] = eye_radius
        lid_obj["shell_outer_radius"] = outer_radius
        lid_obj["rest_direction"] = tuple(round(value, 6) for value in rest_direction)
        lid_obj["closed_angle_degrees"] = 70
        lid_obj["shell_thickness"] = outer_radius - eye_radius
        lid_obj["triangle_budget"] = 60

    if kitten:
        tail_pivots = [(0, 0.30, 0.52), (0.035, 0.46, 0.59), (0.065, 0.57, 0.70), (0.075, 0.55, 0.80)]
        tail_ends = [tail_pivots[1], tail_pivots[2], tail_pivots[3], (0.07, 0.46, 0.83)]
        tail_radii = [(0.078, 0.073), (0.073, 0.066), (0.066, 0.058), (0.058, 0.047)]
    else:
        tail_pivots = [(0, 0.36, 0.62), (0.05, 0.53, 0.72), (0.10, 0.65, 0.92), (0.14, 0.68, 1.10)]
        tail_ends = [tail_pivots[1], tail_pivots[2], tail_pivots[3], (0.18, 0.56, 1.17)]
        tail_radii = [(0.086, 0.079), (0.079, 0.070), (0.070, 0.061), (0.061, 0.052)]
    tail_parts = []
    for i, (pivot, end, (r0, r1)) in enumerate(zip(tail_pivots, tail_ends, tail_radii, strict=True)):
        slots = ["Coat_Base", "Coat_Marking", "Coat_Point"] if i == 3 else ["Coat_Base", "Coat_Marking"]
        def tail_mask(p, t, index=i):
            if index == 3 and t > 0.55:
                return "Coat_Point"
            if (index in (0, 1, 2) and 0.28 < t < 0.48) or (index == 3 and 0.08 < t < 0.25):
                return "Coat_Marking"
            return "Coat_Base"
        cap = 1
        tail_parts.append(_join_part("Tail" if i == 0 else f"Tail_{i}", [_capsule("tail capsule", pivot, end, r0, r1, slots, tail_mask, radial=(8 if kitten else 10), cap_rings=(0 if kitten else cap))], pivot))

    legs = {}
    for side, side_name in ((-1, "Left"), (1, "Right")):
        for y, limb_name in ((-0.28 if not kitten else -0.23, "Front"), (0.30 if not kitten else 0.23, "Back")):
            legs[limb_name + side_name] = _make_leg(side, y, limb_name + side_name, kitten)

    # Author the kitten in its own proportions, then apply one uniform source
    # scale so nose-to-tail length matches the adult 1.50--1.60 m contract.
    if kitten:
        for obj in list(H.CURRENT.objects):
            if obj.type == "MESH":
                obj.location = obj.location * 1.08
                obj.scale = (1.08, 1.08, 1.08)
                _bake(obj)

    final_scale = 1.08 if kitten else 1.0
    final_torso_center = Vector(body_center) * final_scale
    final_torso_radii = Vector(body_radii) * final_scale
    for limb_name, (upper_obj, _lower_obj) in legs.items():
        _assert_upper_joint_inside(
            f"{name}:{upper_obj.name}",
            upper_obj,
            upper_obj.location,
            final_torso_center,
            final_torso_radii,
            0.012 * final_scale,
        )

    _triangulate_parts()
    finish_asset(name, articulated=True)
    entry = H.ASSETS[name]
    part_triangles = {
        re.sub(r"\.\d{3}$", "", obj.name): sum(len(face.vertices) - 2 for face in obj.data.polygons)
        for obj in entry["meshes"]
    }
    for lid_name in ("EyelidL", "EyelidR"):
        if part_triangles[lid_name] > 60:
            raise AssertionError(f"{name}: {lid_name} has {part_triangles[lid_name]} triangles; limit is 60")
    triangle_limit = 2000 if kitten else 2900
    if entry["triangles"] > triangle_limit:
        raise AssertionError(f"{name}: {entry['triangles']} triangles exceed {triangle_limit} asset budget")
    def canonical(obj):
        return re.sub(r"\.\d{3}$", "", obj.name)
    meshes = {canonical(obj): obj for obj in entry["meshes"]}
    body_obj = meshes["Body"]
    allowed_materials = {
        "Forest_Coat_Base_fur",
        "Forest_Coat_Marking_fur_dark",
        "Forest_Coat_White_fur_light",
        "Forest_Coat_Point_fur",
        "Forest_nose",
        "Forest_gold",
        "Forest_eye",
        "Forest_cream",
    }
    for part in entry["meshes"]:
        actual_materials = {material.name for material in part.data.materials}
        if not actual_materials <= allowed_materials:
            raise AssertionError(f"{name}: forbidden material on {part.name}: {sorted(actual_materials - allowed_materials)}")

    minimum_drop = 0.10 if kitten else 0.15
    for side_name in ("Left", "Right"):
        for limb_name in ("Front", "Back"):
            paw = meshes[f"Paw{limb_name}{side_name}"]
            lower = meshes[f"Leg{limb_name}{side_name}Lower"]
            drop = paw.location.z - lower.location.z
            if drop < minimum_drop:
                raise AssertionError(f"{name}: {paw.name} pivot is too close to {lower.name}: {drop:.4f}")
            if (paw.location - lower.location).length < 1e-4:
                raise AssertionError(f"{name}: lower pivot equals its parent {paw.name}")
            if tuple(round(value, 6) for value in paw["pivot_source"]) != tuple(round(value, 6) for value in paw.location):
                # The adult is authored directly; the kitten is uniformly scaled
                # below, so only the relative pivot contract is checked there.
                if not kitten:
                    raise AssertionError(f"{name}: unexpected Paw pivot transform on {paw.name}")
    for part_name, parent_name in {
        "Head": "Body", "Tail": "Body", "PawFrontLeft": "Body", "PawFrontRight": "Body", "PawBackLeft": "Body", "PawBackRight": "Body",
        "EarL": "Head", "EarR": "Head", "Jaw": "Head", "EyelidL": "Head", "EyelidR": "Head", "Tail_1": "Tail", "Tail_2": "Tail_1", "Tail_3": "Tail_2",
        "LegFrontLeftLower": "PawFrontLeft", "LegFrontRightLower": "PawFrontRight", "LegBackLeftLower": "PawBackLeft", "LegBackRightLower": "PawBackRight",
    }.items():
        _parent_keep_world(meshes[part_name], meshes[parent_name])

    sockets = []
    for socket_name, parent_name, location in (
        ("CarryBack", "Body", (0, 0.04, 0.86 if not kitten else 0.70)),
        ("CarryMouth", "Head", (0, -0.78 if not kitten else -0.74, 0.83 if not kitten else 0.79)),
    ):
        socket = H.bpy.data.objects.new(socket_name, None)
        entry["collection"].objects.link(socket)
        parent = meshes[parent_name]
        socket.parent = parent
        socket.matrix_parent_inverse.identity()
        socket.location = parent.matrix_world.inverted() @ Vector(location)
        socket.rotation_euler = (0, 0, 0)
        socket.scale = (1, 1, 1)
        sockets.append(socket)
    entry["sockets"] = sockets

    expected_parents = {
        "Head": "Body", "Tail": "Body", "PawFrontLeft": "Body", "PawFrontRight": "Body", "PawBackLeft": "Body", "PawBackRight": "Body",
        "EarL": "Head", "EarR": "Head", "Jaw": "Head", "EyelidL": "Head", "EyelidR": "Head", "CarryMouth": "Head",
        "Tail_1": "Tail", "Tail_2": "Tail_1", "Tail_3": "Tail_2", "LegFrontLeftLower": "PawFrontLeft", "LegFrontRightLower": "PawFrontRight",
        "LegBackLeftLower": "PawBackLeft", "LegBackRightLower": "PawBackRight", "CarryBack": "Body",
    }
    socket_map = {canonical(s): s for s in sockets}
    for child, parent in expected_parents.items():
        actual = (meshes if child not in {"CarryBack", "CarryMouth"} else socket_map)[child].parent
        if canonical(actual) != parent:
            raise AssertionError(f"{name}: wrong parent {child} -> {actual.name}")
    for obj in entry["meshes"] + sockets:
        if any(abs(value) > 1e-6 for value in obj.rotation_euler) or any(abs(value - 1) > 1e-6 for value in obj.scale):
            raise AssertionError(f"{name}: non-baked transform on {obj.name}")
    entry["contract"] = {
        "parts": [obj.name for obj in entry["meshes"]],
        "axes": "local X lateral, local Y spine, local Z up; yaw Z, pitch X, roll Y",
        "jaw_open": "+25 deg local X",
        "eyelid_close": "+70 deg local X",
        "triangle_budget": 2000 if kitten else 2900,
        "part_triangles": part_triangles,
        "eyeball": {
            "adult_or_kitten_centre": [round(v, 6) for v in (eye_x, eye_y, eye_z)],
            "radius": eye_radius,
            "opening_radius": opening_radius,
            "iris_radius": _iris_radius,
            "pupil_radius": _pupil_radius,
            "rest_protrusion_limit": 0.030,
        },
        "sockets": [socket.name for socket in sockets],
        "kitten_width": 0.47 if kitten else 0.85,
    }
    return entry


def cat():
    return _build_cat("cat", kitten=False)


def cat_kitten():
    return _build_cat("cat_kitten", kitten=True)


BUILDERS = {"cat": cat, "cat_kitten": cat_kitten}
