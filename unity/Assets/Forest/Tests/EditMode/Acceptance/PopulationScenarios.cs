using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Population: housing, pregnancy beds, migration, death and colony extinction.</summary>
    public static class PopulationScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            foreach (string housing in new[] { "automatic", "leader_plan_house", "build_house" })
            {
                string mode = housing;
                yield return new Scenario("regression.review_housing_skips_disconnected_" + mode, () => HousingConnectedSite(mode));
            }
            yield return new Scenario("regression.pregnancy_beds_no_overbooking", Beds);
            yield return new Scenario("regression.death_preserves_cargo_and_exact_items", DeathCargo);
            yield return new Scenario("regression.extinction_restores_atomic_founding", Extinction);
            yield return new Scenario("regression.unhoused_migrant_physically_leaves", MigrantDeparture);
        }
        static void HousingConnectedSite(string mode)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            v.FoundedAt = -6 * 3600;
            World.Add(v.Stockpiles[0].Goods, "planks", 16);
            World.Add(v.Stockpiles[0].Goods, "blocks", 8);
            var disconnected = new Int2(-4, -4);
            var connected = new Int2(4, -4);
            v.Known.Clear();
            v.Known.Add(new Int2(-11, 4));
            foreach (var tile in w.Tiles)
                tile.Road = Math.Max(Math.Abs(tile.Position.X), Math.Abs(tile.Position.Z)) == 2;
            foreach (var origin in new[] { disconnected, connected })
                for (int x = 0; x < 2; x++)
                    for (int z = 0; z < 2; z++)
                        v.Known.Add(new Int2(origin.X + x, origin.Z + z));
            w.TileAt(new Int2(3, -2)).Road = w.TileAt(new Int2(4, -2)).Road = true;
            Check(!w.BuildingEntrance(v, new Building { Position = disconnected }).HasValue && w.BuildingEntrance(v, new Building { Position = connected }).HasValue, "Housing fixture must offer a disconnected site before a connected site");
            var denied = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "den", Position = disconnected, CatId = c.Id });
            Check(!denied.Success && denied.Error.Contains("entrance"), "First housing site is not a clear but disconnected footprint");
            if (mode == "automatic")
                w.Step(10);
            else
                Act(w, v, new GameAction { Kind = "RequestJob", Name = mode, CatId = c.Id });
            var den = v.Buildings.SingleOrDefault(b => b.Kind == "den" && !b.Completed);
            Check(den != null && den.Position.Equals(connected), "Housing stopped at the disconnected site instead of selecting the later connected site");
            var job = v.Jobs.Single(j => j.TargetId == den.Id);
            if (mode == "automatic")
                Check(job.AutomatedBy == "leader" && job.Requester == "leader", "Automatic housing lost its Leader work owner");
            w.Step(600);
            Check(den.Completed && job.Completed, "Selected connected housing did not complete through physical delivery and construction");
            Near(Goods(w, v, "planks"), 0, "Housing did not consume its exact Plank bill");
            Near(Goods(w, v, "blocks"), 0, "Housing did not consume its exact Block bill");
            Check(w.Reservations.Count == 0, "Completed housing retained claims");
            Valid(w);
        }
        static void Beds()
        {
            var w = Fixture(out var v, out var c);
            v.Buildings.Add(new Building { Id = w.Id("den"), Kind = "den", Position = new Int2(-4, 1), Completed = true });
            foreach (var mother in v.Cats.Take(5))
                mother.PregnantUntil = 59;
            var migrant = new Cat { Id = w.Id("migrant"), Name = "Probationer", VillageId = v.Id, Position = v.Center, Migration = "probationary", ProbationUntil = 100000 };
            v.Cats.Add(migrant);
            w.Step(60);
            Valid(w);
            Check(v.Cats.Count(x => x.Alive && x.AgeHours < 1) == 5, "Reserved pregnancies failed to receive beds");
            Check(migrant.BedId == "", "Migrant took reserved bed");
        }
        static void DeathCargo()
        {
            var w = Fixture(out var v, out var c);
            c.Health = 0;
            c.Cargo.Add(new Stack("logs", 8));
            var job = new Job { Id = w.Id("job"), Kind = "production", RecipeId = "logs_to_planks", CatId = c.Id, Phase = "working", Position = c.Position, Local = new List<Stack> { new Stack("logs", 2) } };
            v.Jobs.Add(job);
            c.JobId = job.Id;
            var item = new Item { Id = w.Id("tool"), Kind = "tool", VillageId = v.Id, LocationId = c.Id };
            v.Items.Add(item);
            c.Equipment.Add(item.Id);
            w.Step(1);
            Check(!c.Alive, "Zero-health worker did not die");
            Near(Goods(w, v, "logs"), 10, "Death lost/duplicated raw cargo and staged inputs");
            Check(v.Items.Count == 1 && v.Stockpiles.Any(p => p.Id == item.LocationId), "Death lost exact equipped item");
            Valid(w);
        }
        static void Extinction()
        {
            var w = Fixture(out var v, out var c);
            var oldIds = v.Cats.Select(x => x.Id).ToArray();
            foreach (var cat in v.Cats)
                cat.Health = 0;
            v.Blessings = 7;
            var pending = new Job { Id = w.Id("job"), Kind = "haul", CatId = c.Id, Local = new List<Stack> { new Stack("logs", 2) } };
            v.Jobs.Add(pending);
            c.JobId = pending.Id;
            w.Step(60);
            v = w.Villages[0];
            Check(v.Run == 2 && v.Cats.Count(x => x.Alive) == 30, "Extinction did not atomically restore communal founding");
            Check(v.Buildings.Count(b => b.Kind == "den" && b.Completed) == 6, "Extinction did not restore complete housing");
            Check(!v.Cats.Any(x => oldIds.Contains(x.Id)), "Extinction reused cat identities");
            Check(v.Jobs.Count == 0 && w.Reservations.Count == 0, "Failed-run work survived reset");
            Near(v.Blessings, 7, "Blessing remainder lost during reset");
            Valid(w);
        }
        static void MigrantDeparture()
        {
            var w = Fixture(out var v, out var c);
            var migrant = new Cat { Id = w.Id("arrival"), Name = "Unhoused visitor", VillageId = v.Id, Migration = "probationary", ProbationUntil = 1, Position = v.Center };
            v.Cats.Add(migrant);
            w.Step(100);
            Check(!migrant.Alive && Int2.Distance(migrant.Position, v.Center) > v.Radius, "Unhoused probationer did not physically leave after deadline");
            Check(v.Run == 1, "Departure caused colony reset");
        }
    }
}
