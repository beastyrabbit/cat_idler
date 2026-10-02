using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        public Tile GetTile(Int2 p)
        {
            if (tileIndex == null || tileIndex.Count != Tiles.Count)
                tileIndex = Tiles.ToDictionary(t => t.Position);
            return tileIndex.TryGetValue(p, out var tile) ? tile : null;
        }
        public Tile TileAt(Int2 p)
        {
            if (tileIndex == null || tileIndex.Count != Tiles.Count)
                tileIndex = Tiles.ToDictionary(t => t.Position);
            if (tileIndex.TryGetValue(p, out var old))
                return old;
            var t = p.Level == 0 ? GenerateSurfaceTile(p) : DungeonTileAt(p);
            foreach (var v in Villages)
                if (p.Level == 0 && Math.Abs(p.X - v.Center.X) <= v.Radius && Math.Abs(p.Z - v.Center.Z) <= v.Radius)
                {
                    if (GenerationVersion > 0)
                        ClearSurface(t, 0);
                    t.Biome = "meadow";
                    t.Water = t.Mountain = false;
                    t.Resource = "";
                    t.Amount = 0;
                    t.ClaimId = v.Id;
                }
            Tiles.Add(t);
            tileIndex[p] = t;
            return t;
        }
    }
}
