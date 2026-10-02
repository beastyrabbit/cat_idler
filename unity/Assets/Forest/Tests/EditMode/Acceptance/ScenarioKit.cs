using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public sealed class Scenario
    {
        public string Name;
        public Action Run;
        public Scenario(string name, Action run)
        {
            Name = name;
            Run = run;
        }
    }

    /// <summary>Fixtures and assertions shared by the scenario files; import with <c>using static</c>. All fixture credit is established before actions or time begin.</summary>
    public static class ScenarioKit
    {
        internal static World ContinuousFixture(out Village v, out Cat c, bool working)
        {
            var w = Fixture(out v, out c);
            foreach (var cat in v.Cats)
            {
                cat.ControlledBy = "";
                cat.BuildingId = "fixture-held";
            }
            c.BuildingId = "";
            c.Stats.Clear();
            c.Stats.Add(new Stack("hunting", 50));
            for (int x = 1; x <= 6; x++)
            {
                var tile = w.TileAt(new Int2(x, 2));
                tile.Road = tile.Dirt = tile.Water = tile.Wall = tile.Mountain = false;
                tile.Biome = "grass";
            }
            if (working)
            {
                var job = new Job { Id = w.Id("continuous-work"), Kind = "hunt", Phase = "working", CatId = c.Id, Position = c.Position, RequiredWork = 1000 };
                v.Jobs.Add(job);
                c.JobId = job.Id;
            }
            else
                c.Path = w.Path(c.Position, new Int2(4, 2), v);
            return w;
        }
        internal static World FailedPathFixture(out Village v, out Cat c)
        {
            var w = ContinuousFixture(out v, out c, true);
            v.Jobs.Single().Position = new Int2(4, 2);
            // A bounded corridor keeps a failed, passable destination cheap to verify.
            for (int x = 0; x <= 5; x++)
                for (int z = 1; z <= 3; z++)
                {
                    var tile = w.TileAt(new Int2(x, z));
                    tile.Water = tile.Mountain = false;
                    tile.Wall = x == 0 || x == 5 || z == 1 || z == 3;
                }
            v.BoundaryEdges.Add(new BoundaryEdge { From = new Int2(3, 2), To = new Int2(4, 2) });
            return w;
        }
        internal static long PathSearches(World w) => (long)typeof(World).GetField("pathSearchCount", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic).GetValue(w);
        internal static void Check(bool condition, string message)
        {
            if (!condition)
                throw new InvalidOperationException(message);
        }
        internal static void Near(double actual, double expected, string message, double tolerance = 0.00001) => Check(Math.Abs(actual - expected) <= tolerance, message + " expected=" + expected + " actual=" + actual);
        internal static PlayerContext Context(Village v) => new PlayerContext { PlayerId = "acceptance-player", VillageId = v.Id };
        internal static ActionResult Act(World w, Village v, GameAction a)
        {
            var r = w.Apply(Context(v), a);
            Check(r.Success, a.Kind + ": " + r.Error);
            return r;
        }
        internal static World Fixture(out Village v, out Cat c)
        {
            var w = World.Create(41);
            v = w.Villages[0];
            c = v.Cats[0];
            // Isolated rule fixtures retain founding beds/shrine; full blueprints run in campaigns.
            v.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            v.Stockpiles.RemoveRange(1, v.Stockpiles.Count - 1);
            foreach (var other in v.Cats)
            {
                other.ControlledBy = "fixture-held";
                other.ControlLeaseUntil = 10000000;
            }
            c.ControlledBy = "";
            c.BuildingId = "fixture-held";
            c.Position = new Int2(1, 2);
            c.X = 1;
            c.Z = 2;
            v.ResearchPoints = 1000000;
            v.LastLeaderResearch = 0;
            v.Stockpiles[0].Goods.Clear();
            v.Stockpiles[0].Capacity = 100000;
            v.Stockpiles[0].Position = new Int2(3, 2);
            World.Add(v.Stockpiles[0].Goods, "food", 1000);
            World.Add(v.Stockpiles[0].Goods, "water", 1000);
            return w;
        }
        internal static Building Station(World w, Village v, string kind, Int2? at = null)
        {
            var b = new Building { Id = w.Id("fixture-station"), Kind = kind, Position = at ?? new Int2(1, 3), Completed = true };
            v.Buildings.Add(b);
            return b;
        }
        internal static double Goods(World w, Village v, string resource) => v.Stockpiles.Sum(p => World.Amount(p.Goods, resource)) + v.Cats.Sum(c => World.Amount(c.Cargo, resource)) + v.Jobs.Where(j => !j.Completed).Sum(j => World.Amount(j.Local, resource)) + v.Buildings.Sum(b => World.Amount(b.Inputs, resource) + World.Amount(b.Outputs, resource)) + v.Farms.Where(f => f.Crop == resource).Sum(f => f.Harvest) + v.Vehicles.Sum(x => World.Amount(x.Cargo, resource));
        internal static void Valid(World w)
        {
            Check(w.Validate().Count == 0, string.Join("; ", w.Validate()));
            foreach (var v in w.Villages)
            {
                foreach (var item in v.Items)
                    Check(v.Stockpiles.Any(p => p.Id == item.LocationId) || v.Cats.Any(c => c.Alive && c.Id == item.LocationId) || v.Jobs.Any(j => !j.Completed && j.Id == item.LocationId) || v.Vehicles.Any(vehicle => vehicle.Id == item.LocationId), "Orphan exact item " + item.Id + " at " + item.LocationId);
                int beds = v.Buildings.Count(b => b.Completed && b.Kind == "den") * w.DenCapacity(v);
                Check(v.Cats.Count(c => c.Alive && c.BedId != "") + v.Cats.Count(c => c.Alive && c.PregnantUntil > 0) <= beds, "Permanent beds plus pregnancy claims overbooked");
                foreach (var cargo in v.Jobs.Where(j => !j.Completed).SelectMany(j => j.Local).Concat(v.Vehicles.SelectMany(x => x.Cargo)))
                    Check(World.Finite(cargo.Amount) && cargo.Amount >= 0, "Invalid work/vehicle cargo");
            }
            foreach (var claim in w.Reservations)
            {
                var pile = w.Villages.SelectMany(v => v.Stockpiles).FirstOrDefault(p => p.Id == claim.PileId);
                Check(pile != null && World.Finite(claim.Amount) && claim.Amount > 0, "Orphan/nonfinite reservation " + claim.OwnerId);
                Check(w.Villages.Any(v => v.Jobs.Any(j => !j.Completed && (j.Id == claim.OwnerId || j.Id + ":resume" == claim.OwnerId)) || v.Buildings.Any(b => !b.Completed && b.Id == claim.OwnerId)), "Reservation has no live work owner " + claim.OwnerId);
            }
        }
        internal static void Exterior(World w, Village v)
        {
            for (int x = 7; x <= 18; x++)
                for (int z = 7; z <= 18; z++)
                {
                    var p = new Int2(x, z);
                    var t = w.TileAt(p);
                    t.Water = t.Mountain = t.Wall = false;
                    t.Road = t.Rail = false;
                    t.Resource = "";
                    t.Amount = 0;
                    t.ClaimId = "";
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
        }
        internal static void FixtureAccessRoad(World w, Village v, Int2 destination)
        {
            Check(w.TimeSeconds == 0, "Fixture roads must be established before play");
            var at = new Int2(v.Center.X, v.Center.Z + v.Radius + 1);
            while (true)
            {
                var tile = w.TileAt(at);
                World.ClearSurface(tile, 0);
                tile.Road = true;
                tile.Water = tile.Mountain = tile.Wall = false;
                tile.Resource = "";
                tile.Amount = 0;
                if (!v.Known.Contains(at))
                    v.Known.Add(at);
                if (at.Equals(destination))
                    break;
                if (at.Z != destination.Z)
                    at.Z += Math.Sign(destination.Z - at.Z);
                else
                    at.X += Math.Sign(destination.X - at.X);
            }
        }
        internal static string TerrainState(Tile t) => t.ClaimId + ":" + t.Wall + ":" + t.Water + ":" + t.Mountain + ":" + t.Road + ":" + t.Rail + ":" + t.Bridge + ":" + t.Dock + ":" + t.Resource + ":" + t.Amount;
        internal static void PrepareExpansion(World w, Village v)
        {
            int radius = v.Radius + 2;
            for (int x = -radius; x <= radius; x++)
                for (int z = -radius; z <= radius; z++)
                {
                    var at = new Int2(v.Center.X + x, v.Center.Z + z);
                    var t = w.TileAt(at);
                    if (Math.Abs(x) == radius || Math.Abs(z) == radius)
                    {
                        t.Wall = t.Water = t.Mountain = false;
                    }
                    if (!v.Known.Contains(at))
                        v.Known.Add(at);
                }
        }
        internal static List<string> Ancestors(Study node)
        {
            var result = new HashSet<string>();
            void Visit(string id)
            {
                if (!result.Add(id))
                    return;
                foreach (var p in Catalog.Study(id).Prerequisites)
                    Visit(p);
            }
            foreach (var p in node.Prerequisites)
                Visit(p);
            return result.OrderBy(x => x, StringComparer.Ordinal).ToList();
        }
        internal static World StudyFixture(string id, bool own, out Village v, out Cat c)
        {
            var w = Fixture(out v, out c);
            var node = Catalog.Study(id);
            v.Research = Ancestors(node);
            if (own)
                v.Research.Add(id);
            return w;
        }
        internal static string Fingerprint(World w)
        {
            var result = new System.Text.StringBuilder();
            void Append(object value)
            {
                if (value == null)
                {
                    result.Append("null;");
                    return;
                }
                if (value is string text)
                {
                    result.Append(text.Length).Append(':').Append(text).Append(';');
                    return;
                }
                if (value is double number)
                {
                    result.Append(Math.Round(number, 9).ToString("R", System.Globalization.CultureInfo.InvariantCulture)).Append(';');
                    return;
                }
                if (value.GetType().IsPrimitive)
                {
                    result.Append(Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture)).Append(';');
                    return;
                }
                if (value is System.Collections.IEnumerable sequence)
                {
                    result.Append('[');
                    foreach (var entry in sequence)
                        Append(entry);
                    result.Append(']');
                    return;
                }
                result.Append('{');
                foreach (var field in value.GetType().GetFields(System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.Public).OrderBy(f => f.Name, StringComparer.Ordinal))
                {
                    result.Append(field.Name).Append('=');
                    Append(field.GetValue(value));
                }
                result.Append('}');
            }
            Append(w);
            return result.ToString();
        }
    }
}
