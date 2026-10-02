using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    /// <summary>
    /// Founding: classic worlds keep their exact layout; new demo colonies (generation version 2)
    /// start in a revealed, forested landscape with water, a landmark and room to expand.
    /// </summary>
    public static class FoundingScenarios
    {
        // SHA-256 of the founded layout of World.Create(41), recorded before generation version 2
        // existed. A change here means classic geometry moved and every classic coordinate is suspect.
        const string ClassicLayoutHash41 = "d87c8aa9b8fd592c765894e0b50c102ae8338efae0e2d6edff5fe4a00a766c04";

        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("founding.classic_layout_hash_unchanged", () =>
            {
                string actual = LayoutHash(World.Create(41));
                Require(actual == ClassicLayoutHash41, "Classic founded layout of World.Create(41) changed: " + actual);
            });
        }

        private static void Require(bool value, string message)
        {
            if (!value)
                throw new InvalidOperationException(message);
        }

        private static string R(double value) => value.ToString("R", CultureInfo.InvariantCulture);
        private static string P(Int2 p) => p.X.ToString(CultureInfo.InvariantCulture) + "," + p.Z.ToString(CultureInfo.InvariantCulture) + "," + p.Level.ToString(CultureInfo.InvariantCulture);
        private static string Goods(IEnumerable<Stack> stacks) => string.Join(";", stacks.Select(s => s.Resource + "=" + R(s.Amount)));

        /// <summary>Geometry and identities a founding produces; actor entropy and stats are deliberately excluded.</summary>
        internal static string LayoutHash(World w)
        {
            var text = new StringBuilder();
            text.Append("gen=").Append(w.GenerationVersion).Append('\n');
            foreach (var plateau in w.SurfacePlateaus)
                text.Append("plateau ").Append(plateau.VillageId).Append(' ').Append(P(plateau.Center)).Append(' ').Append(plateau.CoreRadius).Append(' ').Append(plateau.Shoulder).Append('\n');
            foreach (var t in w.Tiles)
                text.Append("tile ").Append(P(t.Position)).Append(' ').Append(t.Biome).Append(' ').Append(t.Landform).Append(' ').Append(t.Resource).Append(' ').Append(R(t.Amount)).Append(' ').Append(R(t.FishCapacity))
                    .Append(' ').Append(R(t.Elevation)).Append(' ').Append(R(t.WaterSurface)).Append(' ').Append(R(t.WaterDepth)).Append(' ').Append(t.Water).Append(t.Mountain).Append(t.Wall).Append(t.Road).Append(t.Dirt)
                    .Append(' ').Append(t.ClaimId).Append(' ').Append(Goods(t.Deposits)).Append('\n');
            foreach (var v in w.Villages)
            {
                text.Append("village ").Append(v.Id).Append(' ').Append(v.Name).Append(' ').Append(P(v.Center)).Append(' ').Append(v.Radius).Append(' ').Append(v.LayoutVersion).Append(' ').Append(v.LeaderId).Append(' ').Append(R(v.NextElection)).Append('\n');
                text.Append("known ").Append(string.Join(" ", v.Known.Select(P))).Append('\n');
                text.Append("claimed ").Append(string.Join(" ", v.ClaimedTiles.Select(P))).Append('\n');
                foreach (var b in v.Buildings)
                    text.Append("building ").Append(b.Id).Append(' ').Append(b.Kind).Append(' ').Append(P(b.Position)).Append(' ').Append(b.Width).Append('x').Append(b.Depth).Append(' ').Append(b.HasEntrance).Append(' ').Append(P(b.Entrance)).Append(' ').Append(b.Completed).Append('\n');
                foreach (var s in v.Stockpiles)
                    text.Append("pile ").Append(s.Id).Append(' ').Append(s.Kind).Append(' ').Append(s.ManagedBy).Append(' ').Append(P(s.Position)).Append(' ').Append(s.Width).Append('x').Append(s.Depth).Append(' ').Append(R(s.Capacity)).Append(' ').Append(Goods(s.Goods)).Append('\n');
                foreach (var c in v.Cats)
                    text.Append("cat ").Append(c.Id).Append(' ').Append(c.Name).Append(' ').Append(P(c.Position)).Append(' ').Append(R(c.X)).Append(' ').Append(R(c.Z)).Append(' ').Append(c.BedId).Append('\n');
            }
            foreach (var d in w.Dungeons)
            {
                text.Append("dungeon ").Append(d.Id).Append(' ').Append(P(d.Entrance)).Append(' ').Append(d.Depth).Append(' ').Append(R(d.BaseElevation)).Append('\n');
                foreach (var f in d.Floors)
                    text.Append("floor ").Append(f.Level).Append(' ').Append(P(f.Spawn)).Append(' ').Append(P(f.LootPosition)).Append(' ').Append(string.Join(" ", f.Cells.Select(P))).Append('\n');
                foreach (var s in d.Stairs)
                    text.Append("stair ").Append(P(s.From)).Append(' ').Append(P(s.To)).Append(' ').Append(string.Join(" ", s.Opening.Select(P))).Append('\n');
            }
            using (var sha = SHA256.Create())
                return string.Concat(sha.ComputeHash(Encoding.UTF8.GetBytes(text.ToString())).Select(b => b.ToString("x2", CultureInfo.InvariantCulture)));
        }
    }
}
