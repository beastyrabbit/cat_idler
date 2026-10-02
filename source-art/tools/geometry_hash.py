#!/usr/bin/env python3
"""Compute deterministic geometry fingerprints for Blender binary FBX files."""

import argparse
import hashlib
import json
import struct
import zlib
from pathlib import Path


class FBXError(RuntimeError):
    """Raised when an FBX file cannot be parsed."""


def _read_prop(data, pos):
    if pos >= len(data):
        raise FBXError("truncated property")
    typ = chr(data[pos])
    pos += 1
    scalar_formats = {
        "Y": ("<h", 2),
        "I": ("<i", 4),
        "F": ("<f", 4),
        "D": ("<d", 8),
        "L": ("<q", 8),
    }
    if typ in scalar_formats:
        fmt, size = scalar_formats[typ]
        return struct.unpack_from(fmt, data, pos)[0], pos + size
    if typ == "C":
        return bool(data[pos]), pos + 1
    if typ in ("R", "S"):
        length = struct.unpack_from("<I", data, pos)[0]
        pos += 4
        value = data[pos : pos + length]
        if typ == "S":
            value = value.decode("utf-8", "replace")
        return value, pos + length
    if typ in "fdilb":
        return _read_array(data, pos, typ)
    raise FBXError(f"unknown property type {typ!r}")


def _read_array(data, pos, typ):
    count, encoding, compressed_length = struct.unpack_from("<III", data, pos)
    pos += 12
    raw = data[pos : pos + compressed_length]
    pos += compressed_length
    if encoding == 1:
        raw = zlib.decompress(raw)
    formats = {"f": "<f", "d": "<d", "i": "<i", "l": "<q", "b": "<?"}
    fmt = formats[typ]
    size = struct.calcsize(fmt)
    if len(raw) < count * size:
        raise FBXError("truncated array")
    return list(struct.unpack("<" + fmt[1:] * count, raw[: count * size])), pos


def _node(data, pos, end, wide):
    fmt = "Q" if wide else "I"
    header_size = 8 if wide else 4
    if pos + header_size * 3 + 1 > end:
        return None, pos
    end_offset, _, property_length = struct.unpack_from("<" + fmt * 3, data, pos)
    pos += header_size * 3
    name_length = data[pos]
    pos += 1
    if end_offset == 0 and property_length == 0 and name_length == 0:
        return None, pos
    if pos + name_length > end:
        raise FBXError("bad node name")
    name = data[pos : pos + name_length].decode("utf-8", "replace")
    pos += name_length
    properties = []
    property_end = pos + property_length
    while pos < property_end:
        value, pos = _read_prop(data, pos)
        properties.append(value)
    if pos != property_end:
        raise FBXError("property length mismatch")
    children = []
    child_end = end_offset or end
    while pos < child_end:
        child, pos = _node(data, pos, child_end, wide)
        if child is None:
            break
        children.append(child)
    return (name, properties, children), (end_offset or pos)


def parse(path):
    """Parse a binary FBX into a small tree of ``(name, properties, children)`` tuples."""
    data = Path(path).read_bytes()
    if not data.startswith(b"Kaydara FBX Binary"):
        raise FBXError(f"{path}: not a binary FBX file")
    if len(data) < 27:
        raise FBXError(f"{path}: truncated FBX header")
    version = struct.unpack_from("<I", data, 23)[0]
    if version < 7000:
        raise FBXError(f"{path}: unsupported FBX version {version}")
    wide = version >= 7500
    root = []
    pos = 27
    while pos < len(data):
        node, pos = _node(data, pos, len(data), wide)
        if node is None:
            break
        root.append(node)
    return root


def _first(node, name):
    return next((child for child in node[2] if child[0] == name), None)


def _canon(value, digits=6):
    if isinstance(value, (list, tuple)):
        return [_canon(item, digits) for item in value]
    if isinstance(value, float):
        rounded = round(value, digits)
        return 0.0 if rounded == 0 else rounded
    return value


def _digest(value):
    encoded = json.dumps(value, separators=(",", ":"), sort_keys=False).encode()
    return hashlib.sha256(encoded).hexdigest()


def _index_objects(objects):
    models = {}
    geometries = {}
    materials = {}
    for node in objects[2]:
        if not node[1]:
            continue
        identifier = node[1][0]
        if node[0] == "Model":
            models[identifier] = {
                "name": node[1][1] if len(node[1]) > 1 else str(identifier),
                "node": node,
            }
        elif node[0] == "Geometry":
            geometries[identifier] = node
        elif node[0] == "Material":
            materials[identifier] = node[1][1] if len(node[1]) > 1 else str(identifier)
    return models, geometries, materials


def _index_connections(connections, geometries, models, materials):
    model_geometry = {}
    geometry_materials = {}
    parents = {}
    if connections is None:
        return model_geometry, geometry_materials, parents
    for node in connections[2]:
        if node[0] != "C" or len(node[1]) < 3 or node[1][0] not in ("OO", "OP"):
            continue
        _, source, target = node[1][:3]
        if source in geometries and target in models:
            model_geometry[source] = target
        elif source in materials and target in geometries:
            geometry_materials.setdefault(target, []).append(source)
        elif source in models and target in models:
            parents[source] = target
    return model_geometry, geometry_materials, parents


def _geometry_data(geometries, models, model_geometry, geometry_materials, materials):
    ordered = sorted(
        geometries.items(),
        key=lambda item: models.get(model_geometry.get(item[0]), {"name": ""})["name"],
    )
    records = []
    vertices = []
    faces = []
    material_names = []
    for identifier, geometry in ordered:
        model = models.get(model_geometry.get(identifier), {"name": str(identifier)})
        vertex_node = _first(geometry, "Vertices")
        face_node = _first(geometry, "PolygonVertexIndex")
        raw_vertices = vertex_node[1][0] if vertex_node and vertex_node[1] else []
        raw_faces = face_node[1][0] if face_node and face_node[1] else []
        records.append({"owner": model["name"], "v": _canon(raw_vertices), "f": raw_faces})
        vertices.extend(
            tuple(raw_vertices[index : index + 3])
            for index in range(0, len(raw_vertices), 3)
            if index + 2 < len(raw_vertices)
        )
        faces.extend(raw_faces)
        material_node = _first(geometry, "LayerElementMaterial")
        assignments_node = _first(material_node, "Materials") if material_node else None
        assignments = assignments_node[1][0] if assignments_node and assignments_node[1] else []
        slots = [materials.get(item, str(item)) for item in geometry_materials.get(identifier, [])]
        material_names.extend(
            slots[index] if isinstance(index, int) and 0 <= index < len(slots) else str(index) for index in assignments
        )
    return records, vertices, faces, material_names


def _model_transforms(models, parents):
    transforms = []
    for identifier, model in sorted(models.items(), key=lambda item: item[1]["name"]):
        props = _first(model["node"], "Properties70")
        values = {}
        if props:
            for prop in props[2]:
                if prop[0] == "P" and len(prop[1]) >= 5:
                    values[prop[1][0]] = prop[1][-1] if len(prop[1]) == 5 else prop[1][-3:]
        transforms.append(
            [
                model["name"],
                _canon(values.get("Lcl Translation", [0, 0, 0])),
                _canon(values.get("Lcl Rotation", [0, 0, 0])),
                _canon(values.get("Lcl Scaling", [1, 1, 1])),
                models.get(parents.get(identifier), {}).get("name"),
            ]
        )
    return transforms


def _bounds(vertices):
    if not vertices:
        return [[0, 0, 0], [0, 0, 0]]
    return [
        [round(min(vertex[index] for vertex in vertices), 4) for index in range(3)],
        [round(max(vertex[index] for vertex in vertices), 4) for index in range(3)],
    ]


def _polygons(indices):
    polygons = []
    current = []
    for index in indices:
        current.append(index)
        if index < 0:
            polygons.append(current)
            current = []
    if current:
        polygons.append(current)
    return polygons


def extract(path):
    """Return the stable geometry fields used by baseline comparisons."""
    root = parse(path)
    objects = next((node for node in root if node[0] == "Objects"), None)
    if objects is None:
        raise FBXError(f"{path}: missing Objects node")
    connections = next((node for node in root if node[0] == "Connections"), None)
    models, geometries, materials = _index_objects(objects)
    model_geometry, geometry_materials, parents = _index_connections(connections, geometries, models, materials)
    records, vertices, faces, material_names = _geometry_data(
        geometries, models, model_geometry, geometry_materials, materials
    )
    transforms = _model_transforms(models, parents)
    triangles = sum(max(0, len(polygon) - 2) for polygon in _polygons(faces))
    return {
        "vertices": _digest(records),
        "faces": _digest(_canon(faces)),
        "materials": _digest(material_names),
        "transforms": _digest(transforms),
        "bounds": _bounds(vertices),
        "triangles": triangles,
        "vertex_count": len(vertices),
    }


def _asset_files(directory, names):
    if names:
        return [directory / f"{name}.fbx" for name in names]
    return sorted(directory.glob("*.fbx"))


def main():
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--write")
    group.add_argument("--compare")
    parser.add_argument("--dir", default="unity/Assets/Resources/ForestArt")
    parser.add_argument("--only")
    args = parser.parse_args()
    names = [name for name in args.only.split(",") if name] if args.only else []
    files = _asset_files(Path(args.dir), names)
    current = {}
    for path in files:
        if not path.exists():
            raise SystemExit(f"Missing FBX: {path}")
        current[path.stem] = extract(path)
    if args.write:
        output = Path(args.write)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(dict(sorted(current.items())), indent=2) + "\n")
        return
    baseline = json.loads(Path(args.compare).read_text())
    names_to_check = names or sorted(set(baseline) | set(current))
    mismatches = _compare_assets(baseline, current, names_to_check)
    if mismatches:
        print(f"GEOMETRY_HASH_MISMATCH {mismatches} of {len(names_to_check)} assets")
        raise SystemExit(1)
    print(f"GEOMETRY_HASH_MATCH {len(names_to_check)} assets")


def _compare_assets(baseline, current, names):
    fields = ("vertices", "faces", "materials", "transforms", "bounds", "triangles", "vertex_count")
    mismatches = 0
    for name in names:
        if name not in baseline:
            print(f"{name}: missing from baseline")
            mismatches += 1
            continue
        if name not in current:
            print(f"{name}: missing current")
            mismatches += 1
            continue
        changed = False
        for field in fields:
            if baseline[name].get(field) != current[name].get(field):
                print(f"{name} {field} mismatch")
                changed = True
        mismatches += int(changed)
    return mismatches


if __name__ == "__main__":
    main()
