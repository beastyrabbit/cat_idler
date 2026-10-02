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
            yield return new Scenario("regression.population_breeding_pair_is_seeded_and_shares_a_den", BreedingPairSeeded);
            yield return new Scenario("regression.population_breeding_pair_skips_migrants_and_kittens", BreedingPairResidents);
            yield return new Scenario("regression.population_kitten_born_without_free_bed", KittenWithoutBed);
            yield return new Scenario("regression.population_successors_exclude_kittens_and_migrants", OfficeSuccessors);
        }

        /// <summary>
        /// A breeding fixture in the demo profile (its per-check chance makes a pregnancy likely within minutes):
        /// founding beds plus one spare den, stock to spare, and the breeding window already open.
        /// </summary>
        static World BreedingFixture(int seed, out Village v)
        {
            var w = World.Create(seed);
            v = w.Villages[0];
            w.PacingProfile = Pacing.DemoProfile;
            v.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            v.Stockpiles.RemoveRange(1, v.Stockpiles.Count - 1);
            v.Stockpiles[0].Goods.Clear();
            v.Stockpiles[0].Capacity = 100000;
            World.Add(v.Stockpiles[0].Goods, "food", 1000);
            World.Add(v.Stockpiles[0].Goods, "water", 1000);
            foreach (var cat in v.Cats)
            {
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
            }
            v.Buildings.Add(new Building { Id = w.Id("den"), Kind = "den", Position = new Int2(-4, 1), Completed = true });
            v.FoundedAt = -1000000;
            v.LastMigration = 0;
            return w;
        }

        /// <summary>Steps upkeep ticks until a pregnancy starts; returns the mother or null.</summary>
        static Cat StepUntilPregnant(World w, Village v, int maxTicks = 240)
        {
            for (int tick = 0; tick < maxTicks; tick++)
            {
                w.Step(60);
                var mother = v.Cats.FirstOrDefault(c => c.PregnantUntil > 0);
                if (mother != null)
                    return mother;
            }
            return null;
        }

        static void BreedingPairSeeded()
        {
            var mothers = new HashSet<int>();
            for (int seed = 1; seed <= 8; seed++)
            {
                var w = BreedingFixture(seed, out var v);
                var mother = StepUntilPregnant(w, v);
                Check(mother != null, "Seed " + seed + ": no pregnancy in the demo breeding window");
                var mate = v.Cats.Find(c => c.Id == mother.PregnancyMateId);
                Check(mate != null && mate != mother, "Seed " + seed + ": pregnancy has no distinct mate");
                Check(mate.BedId != "" && mate.BedId == mother.BedId, "Seed " + seed + ": mate does not share the mother's den although den-mates exist");
                mothers.Add(v.Cats.IndexOf(mother));
                Valid(w);
            }
            Check(mothers.Count >= 3, "Mothers are not chosen by the seeded roll: list positions " + string.Join(",", mothers));
        }

        static void BreedingPairResidents()
        {
            for (int seed = 1; seed <= 4; seed++)
            {
                var w = BreedingFixture(seed, out var v);
                // Walled in outside the fence, these wanderers stay "arriving" for the whole fixture.
                var pen = new Int2(v.Center.X + 16, v.Center.Z + 16);
                for (int dx = -1; dx <= 1; dx++)
                    for (int dz = -1; dz <= 1; dz++)
                        w.TileAt(new Int2(pen.X + dx, pen.Z + dz)).Wall = dx != 0 || dz != 0;
                // Every founder but the last two becomes ineligible, so only a resident pair can breed.
                var founders = v.Cats.ToArray();
                for (int i = 0; i < founders.Length - 2; i++)
                {
                    var c = founders[i];
                    if (i % 2 == 0)
                    {
                        c.Migration = "arriving";
                        c.BedId = "";
                        c.Position = pen;
                        c.X = pen.X;
                        c.Z = pen.Z;
                    }
                    else
                        c.AgeHours = 1;
                }
                var mother = StepUntilPregnant(w, v);
                Check(mother != null, "Seed " + seed + ": the resident pair never bred");
                var mate = v.Cats.Find(c => c.Id == mother.PregnancyMateId);
                foreach (var parent in new[] { mother, mate })
                    Check(parent != null && parent.Migration == "resident" && parent.BedId != "" && parent.AgeHours >= 24, "Seed " + seed + ": a migrant or kitten was chosen as a parent (" + parent?.Name + ")");
            }
        }

        static void KittenWithoutBed()
        {
            var w = Fixture(out var v, out var c);
            var spare = new Building { Id = w.Id("den"), Kind = "den", Position = new Int2(-4, 1), Completed = true };
            v.Buildings.Add(spare);
            var mother = v.Cats[1];
            mother.PregnantUntil = 30;
            mother.PregnancyMateId = v.Cats[2].Id;
            // The reserved den is lost before term: no free bed remains when the kitten is due.
            v.Buildings.Remove(spare);
            int before = v.Cats.Count;
            w.Step(60);
            var kitten = v.Cats.Skip(before).SingleOrDefault();
            Check(kitten != null && kitten.Alive && kitten.AgeHours < 1, "A kitten at term without a free bed was not born");
            Check(kitten.BedId == "" && mother.PregnantUntil < 0 && mother.PregnancyMateId == "", "Birth without a bed left the pregnancy open or invented a bed");
            Check(kitten.ParentIds.Contains(mother.Id) && kitten.ParentIds.Contains(v.Cats[2].Id), "Bedless kitten lost its parents");
            Check(v.Events.Any(e => e.Kind == "birth" && e.EntityId == kitten.Id), "Bedless birth was not announced");
            Valid(w);
            v.Buildings.Add(new Building { Id = w.Id("den"), Kind = "den", Position = new Int2(-4, 1), Completed = true });
            w.Step(60);
            Check(kitten.BedId != "" && kitten.Alive, "Housing did not give the bedless kitten a bed once one was free");
            Valid(w);
        }

        static void OfficeSuccessors()
        {
            var w = Fixture(out var v, out var c);
            // Ordinal ids that sort before every founder, so the old "first by id" rule would pick them.
            var kitten = new Cat { Id = "0-kitten", Name = "Kitten", VillageId = v.Id, Position = v.Center, AgeHours = 2, ControlledBy = "fixture-held", ControlLeaseUntil = 10000000 };
            var probationer = new Cat { Id = "0-probationer", Name = "Probationer", VillageId = v.Id, Position = v.Center, Migration = "probationary", ProbationUntil = 100000000, ControlledBy = "fixture-held", ControlLeaseUntil = 10000000 };
            var arriving = new Cat { Id = "0-arriving", Name = "Arriving", VillageId = v.Id, Position = new Int2(v.Center.X, v.Center.Z + v.Radius + 3), Migration = "arriving" };
            v.Cats.AddRange(new[] { kitten, probationer, arriving });
            var holder = v.Cats.First(x => x.Id.StartsWith("cat-", StringComparison.Ordinal));
            v.LeaderId = holder.Id;
            v.Officers.Add(new Officer { Role = "steward", CatId = holder.Id });
            holder.OfficerRole = "steward";
            holder.Health = 0;
            w.Step(1);
            Check(!holder.Alive, "Fixture office holder did not die");
            var leader = v.Cats.Find(x => x.Id == v.LeaderId);
            var steward = v.Officers.SingleOrDefault(o => o.Role == "steward");
            var officer = steward == null ? null : v.Cats.Find(x => x.Id == steward.CatId);
            Check(leader != null && w.EligibleForOffice(leader), "Leader successor is not eligible: " + leader?.Name);
            Check(officer != null && w.EligibleForOffice(officer) && officer.OfficerRole == "steward", "Officer successor is not eligible: " + officer?.Name);
            foreach (var x in new[] { kitten, probationer, arriving })
                Check(!w.EligibleForOffice(x), x.Name + " counts as eligible for office");
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
