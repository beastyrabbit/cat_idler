using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>Generated tiles that route searches read without inserting them (never saved).</summary>
        [NonSerialized] private Dictionary<Int2, Tile> generatedTiles;
        [NonSerialized] private long generatedTilesSignature;
        /// <summary>True only inside a route search, which checks the signature once at its start (the world cannot change during it).</summary>
        [NonSerialized] private bool generatedTilesChecked;
        /// <summary>The generated-tile cache is dropped beyond this many entries.</summary>
        private const int GeneratedTileCacheCap = 50000;

        public Tile GetTile(Int2 p)
        {
            if (tileIndex == null || tileIndex.Count != Tiles.Count)
                tileIndex = Tiles.ToDictionary(t => t.Position);
            return tileIndex.TryGetValue(p, out var tile) ? tile : null;
        }
        /// <summary>The saved tile at <paramref name="p"/>, generating and inserting it when it is unexplored.</summary>
        public Tile TileAt(Int2 p)
        {
            var old = GetTile(p);
            if (old != null)
                return old;
            var t = GenerateTile(p);
            Tiles.Add(t);
            tileIndex[p] = t;
            return t;
        }
        /// <summary>
        /// What <see cref="TileAt"/> would insert at <paramref name="p"/> right now, without inserting it:
        /// the coordinate-pure terrain plus the claim clearing of any village whose radius covers it.
        /// </summary>
        private Tile GenerateTile(Int2 p)
        {
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
            return t;
        }
        /// <summary>
        /// Read-only tile lookup for route searches: the saved tile, or a cached copy of what TileAt would
        /// insert. Never inserts, so a failed search leaves the saved world unchanged. Callers must not
        /// mutate the returned tile.
        /// </summary>
        private Tile ReadTile(Int2 p)
        {
            var saved = GetTile(p);
            if (saved != null)
                return saved;
            if (!generatedTilesChecked)
                RefreshGeneratedTiles();
            if (generatedTiles.Count >= GeneratedTileCacheCap)
                generatedTiles.Clear();
            if (!generatedTiles.TryGetValue(p, out var generated))
            {
                generated = GenerateTile(p);
                generatedTiles[p] = generated;
            }
            return generated;
        }
        /// <summary>Drops the generated-tile cache when anything it depends on changed.</summary>
        private void RefreshGeneratedTiles()
        {
            long signature = GeneratedTileSignature();
            if (generatedTiles == null || generatedTilesSignature != signature)
            {
                generatedTiles = new Dictionary<Int2, Tile>();
                generatedTilesSignature = signature;
            }
        }
        /// <summary>Everything a generated tile depends on besides its coordinate; a change drops the cache.</summary>
        private long GeneratedTileSignature()
        {
            unchecked
            {
                long h = 1469598103934665603L;
                void Mix(long value) => h = (h ^ value) * 1099511628211L;
                Mix(GenerationVersion);
                Mix(Seed);
                Mix(Villages.Count);
                foreach (var v in Villages)
                {
                    Mix(v.Center.X);
                    Mix(v.Center.Z);
                    Mix(v.Radius);
                    Mix(Hash(v.Id));
                }
                Mix(SurfacePlateaus.Count);
                foreach (var plateau in SurfacePlateaus)
                {
                    Mix(plateau.Center.X);
                    Mix(plateau.Center.Z);
                    Mix(plateau.Center.Level);
                    Mix(plateau.CoreRadius);
                    Mix(plateau.Shoulder);
                }
                Mix(Dungeons.Count);
                foreach (var site in Dungeons)
                {
                    Mix(site.Floors.Count);
                    Mix(site.Stairs.Count);
                    foreach (var floor in site.Floors)
                        Mix(floor.Cells.Count);
                }
                return h;
            }
        }
    }
}
