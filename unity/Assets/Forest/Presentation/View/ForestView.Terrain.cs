using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.Rendering;

namespace IdleCatForest.Presentation
{
    public sealed partial class ForestView
    {
        private int viewedLevel;
        private bool followSelectedCat;
        public int RenderedLevel => ControlledCat?.Position.Level ?? viewedLevel;
        public void ViewLevel(int level)
        {
            if (DirectControl || level > 0 || level < -8 || level != 0 && !Game.Selected.Known.Any(p => p.Level == level)) return;
            viewedLevel = level; followSelectedCat = false; nextReconcile = 0;
            var place = level == 0 ? Game.Selected.Center : Game.Selected.Known.Where(p => p.Level == level).OrderBy(p => Math.Abs(p.X - focus.x) + Math.Abs(p.Z - focus.z)).First();
            focus = At(place); terrainSignature = int.MinValue; ApplyManagementCamera();
        }
        public void FocusDungeon(string id)
        {
            var site = Game.CurrentWorld.Dungeons.Find(d => d.Id == id); if (site == null || DirectControl) return;
            viewedLevel = 0; followSelectedCat = false; nextReconcile = 0; focus = At(site.Entrance); zoom = 10; ApplyManagementCamera();
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
                var go = new GameObject("Ground " + pair.Key, typeof(MeshFilter), typeof(MeshRenderer)); go.transform.SetParent(groundRoot); go.layer = 8;
                go.GetComponent<MeshFilter>().sharedMesh = mesh;
                var material = Material("ground:" + pair.Key, pair.Key == 1 ? Color.white : colors[pair.Key]);
                if (pair.Key == 1) material.shader = Resources.Load<Shader>("ForestWater");
                go.GetComponent<MeshRenderer>().sharedMaterial = material;
                go.GetComponent<MeshRenderer>().shadowCastingMode = ShadowCastingMode.Off;
                if (pair.Key != 11) go.AddComponent<MeshCollider>().sharedMesh = mesh;
            }
        }
        private static bool InStairOpening(DungeonStair stair, Int2 p)
        {
            int dx = Math.Sign(stair.To.X - stair.From.X), dz = Math.Sign(stair.To.Z - stair.From.Z);
            return Enumerable.Range(1, 4).Any(i => p.X == stair.From.X + dx * i && p.Z == stair.From.Z + dz * i);
        }
        private void RenderDungeonPlaces(HashSet<Int2> known)
        {
            var world = Game.CurrentWorld;
            var controlled = ControlledCat;
            int level = RenderedLevel;
            var subject = controlled == null ? focus : Position(controlled);
            var cameraOffset = Quaternion.Euler(0, yaw, 0) * Vector3.back;
            foreach (var site in world.Dungeons)
            {
                if (level == 0 && known.Contains(site.Entrance))
                {
                    var entrance = Entity("dungeon:" + site.Id, "world_dungeon_entrance", At(site.Entrance), 1, naturalScale: true);
                    entrance.transform.rotation = Quaternion.Euler(0, 180, 0);
                }
                foreach (var stair in site.Stairs)
                {
                    if (stair.From.Level != level && stair.To.Level != level || !known.Contains(stair.From) && !known.Contains(stair.To)) continue;
                    var stairs = Entity("stairs:" + site.Id + ":" + stair.From, "world_dungeon_stairs", new Vector3(stair.From.X, (float)world.DungeonHeight(stair.From), stair.From.Z), 1, naturalScale: true);
                    stairs.transform.rotation = Quaternion.LookRotation(new Vector3(stair.To.X - stair.From.X, 0, stair.To.Z - stair.From.Z));
                }
                foreach (var floor in site.Floors.Where(f => f.Level == level || controlled != null && controlled.Path.Count > 0 && controlled.Path[0].Level == f.Level))
                {
                    var cells = new HashSet<Int2>(floor.Cells);
                    foreach (var p in floor.Cells.Where(p => known.Contains(p) && Int2.Distance(p, groundCenter) <= 76))
                    {
                        var tile = world.GetTile(p); if (tile == null || tile.Wall) continue;
                        if (!site.Stairs.Any(s => s.From.Level == p.Level && InStairOpening(s, p))) Entity("floor:" + p, "world_dungeon_floor", At(p), 1, naturalScale: true);
                        foreach (var step in new[] { new Int2(0, 1), new Int2(1, 0), new Int2(0, -1), new Int2(-1, 0) })
                        {
                            var next = new Int2(p.X + step.X, p.Z + step.Z, p.Level); if (cells.Contains(next) || site.Stairs.Any(s => s.Opening.Contains(next))) continue;
                            if (site.Stairs.Any(s => s.To.Equals(p) && Math.Sign(s.From.X - p.X) == step.X && Math.Sign(s.From.Z - p.Z) == step.Z)) continue;
                            var wall = Entity("cavewall:" + p + ":" + step, "world_dungeon_wall", At(p) + new Vector3(step.X * .5f, 0, step.Z * .5f), 1, naturalScale: true);
                            wall.transform.rotation = Quaternion.Euler(0, step.X == 0 ? 0 : 90, 0);
                            bool foreground = controlled != null && Vector3.Dot(wall.transform.position - subject, cameraOffset) > 0;
                            wall.transform.localScale = new Vector3(1, controlled == null || foreground ? .28f : 1, 1);
                        }
                        if (World.Hash(p.ToString()) % 17 == 0) Entity("cavedetail:" + p, site.Theme == "root_cavern" ? "world_mushrooms" : "world_crystals", At(p) + new Vector3(.3f, 0, .3f), .3f);
                    }
                    if (known.Contains(floor.LootPosition) && floor.Loot.Any(s => s.Amount > 0)) Entity("dungeonloot:" + site.Id + ":" + floor.Level, "world_crystals", At(floor.LootPosition), .65f);
                }
            }
            foreach (var enemy in world.Creatures.Where(e => e.Health > 0 && e.Position.Level == level && known.Contains(e.Position)))
            {
                var go = Entity("creature:" + enemy.Id, "world_cave_beetle", new Vector3((float)enemy.X, (float)world.DungeonHeight(enemy.Position), (float)enemy.Z), enemy.Kind == "stone_guardian" ? 1.3f : .9f);
                if (enemy.Kind == "stone_guardian") Tint(go, Material("stone guardian", new Color(.48f, .55f, .53f)));
            }
        }
        private (string Asset, float Rotation) RoadModel(Int2 p, HashSet<Int2> known)
        {
            bool Road(int x, int z) { var next = new Int2(p.X + x, p.Z + z, p.Level); var tile = known.Contains(next) ? Game.CurrentWorld.GetTile(next) : null; return tile != null && (tile.Road || tile.Overlay == "road_built" || tile.Bridge); }
            int mask = (Road(0, 1) ? 1 : 0) | (Road(1, 0) ? 2 : 0) | (Road(0, -1) ? 4 : 0) | (Road(-1, 0) ? 8 : 0);
            return mask switch { 15 => ("world_road_cross", 0), 11 => ("world_road_junction", 0), 7 => ("world_road_junction", 90), 14 => ("world_road_junction", 180), 13 => ("world_road_junction", 270), 3 => ("world_road_corner", 0), 6 => ("world_road_corner", 90), 12 => ("world_road_corner", 180), 9 => ("world_road_corner", 270), 10 => ("world_road_straight", 90), 5 => ("world_road_straight", 0), 2 => ("world_road_end", 90), 4 => ("world_road_end", 180), 8 => ("world_road_end", 270), _ => ("world_road_end", 0) };
        }
    }
}
