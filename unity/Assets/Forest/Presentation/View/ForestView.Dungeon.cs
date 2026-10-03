using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Dungeon entrances, stairs, floors, cave walls and loot.</summary>
    public sealed partial class ForestView
    {
        public void FocusDungeon(string id)
        {
            var site = Game.CurrentWorld.Dungeons.Find(d => d.Id == id); if (site == null || DirectControl) return;
            viewedLevel = 0; followSelectedCat = false; nextReconcile = 0; focus = At(site.Entrance); zoom = 10; ApplyManagementCamera();
        }

        private static readonly Int2[] Steps = { new Int2(0, 1), new Int2(1, 0), new Int2(0, -1), new Int2(-1, 0) };

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
                HashSet<Int2> openings = null;
                foreach (var floor in site.Floors.Where(f => f.Level == level || controlled != null && controlled.Path.Count > 0 && controlled.Path[0].Level == f.Level))
                {
                    var cells = new HashSet<Int2>(floor.Cells);
                    openings ??= new HashSet<Int2>(site.Stairs.SelectMany(s => s.Opening));
                    foreach (var p in floor.Cells.Where(p => known.Contains(p) && Int2.Distance(p, groundCenter) <= 76))
                    {
                        // A revealed floor cell is floor even before anything walked it into the tile list.
                        var tile = world.GetTile(p); if (tile != null && tile.Wall) continue;
                        if (!openings.Contains(p) && !site.Stairs.Any(s => s.From.Level == p.Level && InStairOpening(s, p))) Entity("floor:" + p, "world_dungeon_floor", At(p), 1, naturalScale: true);
                        foreach (var step in Steps)
                        {
                            var next = new Int2(p.X + step.X, p.Z + step.Z, p.Level); if (cells.Contains(next) || openings.Contains(next)) continue;
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
        }
    }
}
