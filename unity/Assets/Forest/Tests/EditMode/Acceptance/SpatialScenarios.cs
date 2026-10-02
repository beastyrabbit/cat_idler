using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    public static class SpatialScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.world_levels_distinct", () =>
            {
                var surface = new Int2(4, 8);
                var lower = new Int2(4, 8, -1);
                if (surface.Equals(lower) || new HashSet<Int2> { surface, lower }.Count != 2 || Int2.Distance(surface, lower) <= 0)
                    throw new Exception("Overlapping dungeon and surface cells must have different spatial identities");
            });
            yield return new Scenario("spatial.colony_reveal_in_ring_order", RevealInRingOrder);
            yield return new Scenario("spatial.colony_founders_spread_on_plaza", FoundersOnPlaza);
            yield return new Scenario("spatial.colony_site_keeps_classic_origin", () =>
            {
                Check(World.Create(41).Villages.Single().Center.Equals(new Int2(0, 0)), "Classic worlds moved the Grand Commons off the origin");
                var centers = Enumerable.Range(1, 8).Select(seed => World.CreateColony(seed, "", Pacing.DemoProfile, 0).Villages.Single().Center).ToList();
                Check(centers.All(c => Math.Abs(c.X) <= 64 && Math.Abs(c.Z) <= 64 && c.Level == 0), "A colony site was chosen far from the origin");
                Check(centers.Distinct().Count() > 1, "Every colony uses the same site regardless of terrain");
            });
        }

        /// <summary>The whole founding reveal is known at once and nearest tiles come first, so nearest-first scans stay cheap.</summary>
        private static void RevealInRingOrder()
        {
            foreach (int seed in new[] { 3, 4242 })
            {
                var w = World.CreateColony(seed, "", Pacing.DemoProfile, 0);
                var v = w.Villages.Single();
                int Ring(Int2 p) => FoundingScenarios.Ring(v, p);
                Check(v.Known.Count > 0 && v.Known[0].Equals(v.Center), "Reveal does not start at the colony centre");
                int founded = v.Known.TakeWhile(p => Ring(p) <= World.ColonyRevealRadius).Count();
                for (int i = 1; i < founded; i++)
                    Check(Ring(v.Known[i - 1]) <= Ring(v.Known[i]), "Founding reveal is not ordered by distance at " + v.Known[i]);
                Check(World.ColonyRevealRadius >= 30 && founded == (2 * World.ColonyRevealRadius + 1) * (2 * World.ColonyRevealRadius + 1), "Founding reveal is not the full square of radius " + World.ColonyRevealRadius);
                Check(v.Known.Skip(founded).All(p => Ring(p) > World.ColonyRevealRadius), "Founding reveal repeats or interleaves later discoveries");
            }
        }

        private static void FoundersOnPlaza()
        {
            var placements = new List<string>();
            foreach (int seed in new[] { 3, 4, 4242 })
            {
                var w = World.CreateColony(seed, "", Pacing.DemoProfile, 0);
                var v = w.Villages.Single();
                string at = " (seed " + seed + ")";
                Check(v.Cats.Select(c => c.Position).Distinct().Count() == v.Cats.Count, "Founders share a tile" + at);
                foreach (var c in v.Cats)
                {
                    Check(FoundingScenarios.Ring(v, c.Position) < v.Radius && FoundingScenarios.Ring(v, c.Position) >= 2, c.Name + " does not start on the plaza" + at);
                    Check(w.Walkable(v, c.Position) && c.X == c.Position.X && c.Z == c.Position.Z, c.Name + " starts on an unwalkable tile" + at);
                    Check(!v.Buildings.Any(b => c.Position.X >= b.Position.X && c.Position.X < b.Position.X + b.Width && c.Position.Z >= b.Position.Z && c.Position.Z < b.Position.Z + b.Depth), c.Name + " starts inside a building" + at);
                    Check(w.Path(c.Position, new Int2(v.Center.X, v.Center.Z - 2), v) != null, c.Name + " cannot reach the plaza road" + at);
                }
                Check(v.Cats.Count(c => w.GetTile(c.Position).Road) <= v.Cats.Count / 2, "Founders still stand in rows on the roads" + at);
                placements.Add(string.Join(" ", v.Cats.Select(c => (c.Position.X - v.Center.X) + "," + (c.Position.Z - v.Center.Z))));
            }
            Check(placements.Distinct().Count() == placements.Count, "Founders take the same places in every colony");
        }
    }
}
