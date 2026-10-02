using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.Rendering;

namespace IdleCatForest.Presentation
{
    /// <summary>The rendered level and the ground mesh around the camera (the shared terrain).</summary>
    public sealed partial class ForestView
    {
        private const int TerrainLayer = 8;
        private int viewedLevel;
        private int terrainSignature;
        internal Transform groundRoot;
        internal Int2 groundCenter;

        public int RenderedLevel => ControlledCat?.Position.Level ?? viewedLevel;

        public void ViewLevel(int level)
        {
            if (DirectControl || level > 0 || level < -8 || level != 0 && !Game.Selected.Known.Any(p => p.Level == level)) return;
            viewedLevel = level; followSelectedCat = false; nextReconcile = 0;
            var place = level == 0 ? Game.Selected.Center : Game.Selected.Known.Where(p => p.Level == level).OrderBy(p => Math.Abs(p.X - focus.x) + Math.Abs(p.Z - focus.z)).First();
            focus = At(place); terrainSignature = int.MinValue; ApplyManagementCamera();
        }

        /// <summary>Rebuilds the ground when the known tiles around the ground center change; called from Reconcile.</summary>
        private void TerrainTick()
        {
            int signature = TerrainSignature();
            if (signature != terrainSignature) { MakeGround(); terrainSignature = signature; }
        }

        /// <summary>Where a screen ray meets the terrain colliders.</summary>
        private static bool TerrainRaycast(Ray ray, out Vector3 point)
        {
            if (Physics.Raycast(ray, out var hit, 240, 1 << TerrainLayer)) { point = hit.point; return true; }
            point = default;
            return false;
        }

        private int TerrainSignature()
        {
            int level = RenderedLevel;
            int signature = groundCenter.GetHashCode();
            foreach (var p in Game.Selected.Known)
            {
                if (p.Level != level || Math.Abs(p.X - groundCenter.X) > 39 || Math.Abs(p.Z - groundCenter.Z) > 39) continue;
                var t = Game.CurrentWorld.GetTile(p);
                signature = unchecked(signature * 31 + p.GetHashCode());
                if (t != null) signature = unchecked(signature * 31 + t.Elevation.GetHashCode() + t.WaterDepth.GetHashCode() + (t.Water ? 1 : t.Mountain ? 2 : t.Dirt ? 3 : 4));
            }
            return signature;
        }

        private sealed class GroundGeometry
        {
            public readonly List<Vector3> Vertices = new List<Vector3>();
            public readonly List<int> Triangles = new List<int>();
            public readonly List<Color> Colors = new List<Color>();
            public void Quad(Vector3 a, Vector3 b, Vector3 c, Vector3 d, Color? tint = null)
            {
                int i = Vertices.Count; Vertices.Add(a); Vertices.Add(b); Vertices.Add(c); Vertices.Add(d);
                for (int vertex = 0; vertex < 4; vertex++) Colors.Add(tint ?? Color.white);
                Triangles.AddRange(new[] { i, i + 1, i + 2, i, i + 2, i + 3 });
            }
        }

        internal void MakeGround()
        {
            if (Game.CurrentWorld.GenerationVersion > 0 || RenderedLevel != 0) { MakeTerrainGround(); return; }
            foreach (Transform child in groundRoot) { var filter = child.GetComponent<MeshFilter>(); if (filter != null) Destroy(filter.sharedMesh); Destroy(child.gameObject); }
            var v = Game.Selected; var known = new HashSet<Int2>(v.Known);
            var terrain = Game.CurrentWorld.Tiles.ToDictionary(tile => tile.Position);
            var groups = new Dictionary<int, List<Vector3>>();
            for (int z = groundCenter.Z - 38; z <= groundCenter.Z + 38; z++) for (int x = groundCenter.X - 38; x <= groundCenter.X + 38; x++)
            {
                var point = new Int2(x, z); terrain.TryGetValue(point, out var tile);
                int category = !known.Contains(point) ? 0 : tile != null && tile.Water ? 1 : tile != null && tile.Mountain ? 2 : tile != null && tile.Dirt ? 6 : 3 + Math.Abs(unchecked(x * 73 + z * 37)) % 3;
                if (!groups.ContainsKey(category)) groups[category] = new List<Vector3>();
                groups[category].Add(new Vector3(x, 0, z));
            }
            Color[] colors = { new Color(.105f, .18f, .17f), new Color(.18f, .38f, .40f), new Color(.39f, .43f, .39f), new Color(.36f, .47f, .30f), new Color(.367f, .477f, .305f), new Color(.363f, .472f, .302f), new Color(.40f, .43f, .29f) };
            foreach (var pair in groups)
            {
                var vertices = new List<Vector3>(); var triangles = new List<int>();
                foreach (var p in pair.Value)
                {
                    int n = vertices.Count; float h = pair.Key == 1 ? -.07f : -.025f;
                    vertices.Add(p + new Vector3(-.5f, h, -.5f)); vertices.Add(p + new Vector3(-.5f, h, .5f)); vertices.Add(p + new Vector3(.5f, h, .5f)); vertices.Add(p + new Vector3(.5f, h, -.5f));
                    triangles.AddRange(new[] { n, n + 1, n + 2, n, n + 2, n + 3 });
                }
                var mesh = new Mesh { name = "Forest ground " + pair.Key }; mesh.SetVertices(vertices); mesh.SetTriangles(triangles, 0); mesh.RecalculateNormals(); mesh.RecalculateBounds();
                var go = new GameObject("Ground " + pair.Key, typeof(MeshFilter), typeof(MeshRenderer)); go.transform.SetParent(groundRoot);
                go.GetComponent<MeshFilter>().sharedMesh = mesh; go.GetComponent<MeshRenderer>().sharedMaterial = Material("ground:" + pair.Key, colors[pair.Key]);
            }
        }

        private void MakeTerrainGround()
        {
            foreach (Transform child in groundRoot) { var filter = child.GetComponent<MeshFilter>(); if (filter != null) Destroy(filter.sharedMesh); Destroy(child.gameObject); }
            var known = new HashSet<Int2>(Game.Selected.Known); var world = Game.CurrentWorld;
            int level = RenderedLevel;
            var openings = new HashSet<Int2>(world.Dungeons.SelectMany(d => d.Stairs).SelectMany(s => s.Opening));
            var groups = new Dictionary<int, GroundGeometry>();
            GroundGeometry Group(int category) { if (!groups.TryGetValue(category, out var group)) groups[category] = group = new GroundGeometry(); return group; }
            double Height(Int2 p, double fallback) => known.Contains(p) ? world.GetTile(p)?.Elevation ?? fallback : fallback;
            for (int z = groundCenter.Z - 38; z <= groundCenter.Z + 38; z++) for (int x = groundCenter.X - 38; x <= groundCenter.X + 38; x++)
            {
                var p = new Int2(x, z, level); var tile = known.Contains(p) ? world.GetTile(p) : null;
                bool visible = known.Contains(p);
                if (tile?.Landform == "dungeon_opening") continue;
                int category = !visible ? 0 : tile != null && tile.Water ? 1 : tile != null && tile.Mountain ? 2 : tile != null && tile.Dirt ? 6 : 3 + Math.Abs(unchecked(x * 73 + z * 37)) % 3;
                if (tile != null && !tile.Water && !tile.Mountain && !tile.Dirt && tile.Landform != "")
                    category = tile.Biome == "meadow" ? 7 : tile.Biome == "pine_forest" ? 8 : tile.Biome == "wetland" ? 9 : tile.Biome == "highland" ? 10 : category;
                double h = tile?.Elevation ?? (level == 0 ? 0 : focus.y);
                // Four shared corner samples join adjacent dry tiles without thousands of colliders.
                float Corner(int dx, int dz) => (float)((h + Height(new Int2(x + dx, z, level), h) + Height(new Int2(x, z + dz, level), h) + Height(new Int2(x + dx, z + dz, level), h)) * .25 - .025);
                if (tile != null && tile.Water && tile.WaterDepth > 0)
                {
                    Group(11).Quad(new Vector3(x - .5f, (float)h - .04f, z - .5f), new Vector3(x - .5f, (float)h - .04f, z + .5f), new Vector3(x + .5f, (float)h - .04f, z + .5f), new Vector3(x + .5f, (float)h - .04f, z - .5f));
                    float water = (float)tile.WaterSurface - .015f;
                    var tint = Color.Lerp(new Color(.31f, .65f, .60f, .48f), new Color(.07f, .26f, .34f, .92f), Mathf.Clamp01((float)tile.WaterDepth / 3.5f));
                    Group(category).Quad(new Vector3(x - .5f, water, z - .5f), new Vector3(x - .5f, water, z + .5f), new Vector3(x + .5f, water, z + .5f), new Vector3(x + .5f, water, z - .5f), tint);
                }
                else
                {
                    bool opening = tile != null && !tile.Wall && openings.Contains(p);
                    if (opening) continue;
                    Group(category).Quad(new Vector3(x - .5f, Corner(-1, -1), z - .5f), new Vector3(x - .5f, Corner(-1, 1), z + .5f), new Vector3(x + .5f, Corner(1, 1), z + .5f), new Vector3(x + .5f, Corner(1, -1), z - .5f));
                }
            }
            Color[] colors = { new Color(.105f, .18f, .17f), new Color(.22f, .53f, .56f, .65f), new Color(.39f, .43f, .39f), new Color(.36f, .47f, .30f), new Color(.367f, .477f, .305f), new Color(.363f, .472f, .302f), new Color(.40f, .37f, .26f), new Color(.47f, .55f, .31f), new Color(.22f, .37f, .31f), new Color(.32f, .42f, .30f), new Color(.51f, .49f, .39f), new Color(.20f, .29f, .28f) };
            foreach (var pair in groups)
            {
                var mesh = new Mesh { name = "Forest terrain " + pair.Key }; mesh.SetVertices(pair.Value.Vertices); mesh.SetTriangles(pair.Value.Triangles, 0); if (pair.Key == 1) mesh.SetColors(pair.Value.Colors); mesh.RecalculateNormals(); mesh.RecalculateBounds();
                var go = new GameObject("Ground " + pair.Key, typeof(MeshFilter), typeof(MeshRenderer)); go.transform.SetParent(groundRoot); go.layer = TerrainLayer;
                go.GetComponent<MeshFilter>().sharedMesh = mesh;
                var material = Material("ground:" + pair.Key, pair.Key == 1 ? Color.white : colors[pair.Key]);
                if (pair.Key == 1) material.shader = Resources.Load<Shader>("ForestWater");
                go.GetComponent<MeshRenderer>().sharedMaterial = material;
                go.GetComponent<MeshRenderer>().shadowCastingMode = ShadowCastingMode.Off;
                if (pair.Key != 11) go.AddComponent<MeshCollider>().sharedMesh = mesh;
            }
        }
    }
}
